//! Bounded read-only capabilities. File paths never cross the WebView boundary.
use crate::storage::{StorageError, files, pdf::PdfRecord};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
type Result<T> = std::result::Result<T, StorageError>;
pub const MAX_PDF_BYTES: u64 = 128 * 1024 * 1024;
const MAX_TOTAL: u64 = 256 * 1024 * 1024;
#[derive(Default)]
pub struct PdfDocumentService {
    pub requests: super::pdf_open::PdfOpenRequests,
    jobs: Mutex<HashMap<String, Arc<PdfJob>>>,
}
pub struct PdfJob {
    pub cancelled: AtomicBool,
    loaded: AtomicU64,
    total: AtomicU64,
    file: Mutex<Option<(File, PdfRecord, Option<PathBuf>)>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfProgress {
    loaded: u64,
    total: u64,
}
impl PdfJob {
    pub fn check(&self) -> Result<()> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err(StorageError::Invalid("PDF加载已取消。"))
        } else {
            Ok(())
        }
    }
    fn progress(&self, loaded: u64, total: u64) {
        self.loaded.store(loaded, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
    }
}
fn valid_token(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|v| v.is_ascii_hexdigit() && !v.is_ascii_uppercase())
}
fn readonly(path: &Path) -> Result<File> {
    files::ordinary(path)?;
    if !path.is_file() {
        return Err(StorageError::Invalid("请选择普通PDF文件。"));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1);
    }
    Ok(options.open(path)?)
}
impl PdfDocumentService {
    pub fn verify_managed(path: &Path, record: &PdfRecord) -> Result<()> {
        let job = PdfJob {
            cancelled: AtomicBool::new(false),
            loaded: AtomicU64::new(0),
            total: AtomicU64::new(0),
            file: Mutex::new(None),
        };
        verified_copy(path, &record.content_hash, record.byte_size, &job).map(drop)
    }
    pub fn begin(&self, id: &str) -> Result<Arc<PdfJob>> {
        if !valid_token(id) {
            return Err(StorageError::Invalid("PDF加载标识无效。"));
        }
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| StorageError::Invalid("PDF服务锁异常。"))?;
        if jobs.len() >= 3 || jobs.contains_key(id) {
            return Err(StorageError::Invalid(
                "最多同时打开3份PDF，请先关闭一个PDF面板。",
            ));
        }
        let job = Arc::new(PdfJob {
            cancelled: AtomicBool::new(false),
            loaded: AtomicU64::new(0),
            total: AtomicU64::new(0),
            file: Mutex::new(None),
        });
        jobs.insert(id.into(), job.clone());
        Ok(job)
    }
    pub fn cancel(&self, id: &str) -> Result<()> {
        if !valid_token(id) {
            return Err(StorageError::Invalid("PDF加载标识无效。"));
        }
        if let Some(job) = self
            .jobs
            .lock()
            .map_err(|_| StorageError::Invalid("PDF服务锁异常。"))?
            .remove(id)
        {
            job.cancelled.store(true, Ordering::Relaxed);
        }
        Ok(())
    }
    pub fn progress(&self, id: &str) -> Result<PdfProgress> {
        let jobs = self
            .jobs
            .lock()
            .map_err(|_| StorageError::Invalid("PDF服务锁异常。"))?;
        let j = jobs.get(id).ok_or(StorageError::NotFound)?;
        Ok(PdfProgress {
            loaded: j.loaded.load(Ordering::Relaxed),
            total: j.total.load(Ordering::Relaxed),
        })
    }
    pub fn finish(
        &self,
        id: &str,
        file: File,
        record: PdfRecord,
        source: Option<PathBuf>,
    ) -> Result<()> {
        let jobs = self
            .jobs
            .lock()
            .map_err(|_| StorageError::Invalid("PDF服务锁异常。"))?;
        let j = jobs
            .get(id)
            .ok_or(StorageError::Invalid("PDF加载已取消。"))?;
        j.check()?;
        let mut total = record.byte_size;
        for (key, job) in jobs.iter() {
            if key != id
                && let Some((_, r, _)) = job
                    .file
                    .lock()
                    .map_err(|_| StorageError::Invalid("PDF授权锁异常。"))?
                    .as_ref()
            {
                total = total.saturating_add(r.byte_size);
            }
        }
        if total > MAX_TOTAL {
            return Err(StorageError::Invalid(
                "活跃PDF输入合计超过256MiB，请先关闭一份。",
            ));
        }
        *j.file
            .lock()
            .map_err(|_| StorageError::Invalid("PDF授权锁异常。"))? = Some((file, record, source));
        Ok(())
    }
    pub fn adopt_source(&self, from: &str, to: &str) -> Result<()> {
        let (source, record) = self.source(from)?;
        let job = self
            .jobs
            .lock()
            .map_err(|_| StorageError::Invalid("PDF服务锁异常。"))?
            .get(to)
            .cloned()
            .ok_or(StorageError::NotFound)?;
        job.check()?;
        let mut file = job
            .file
            .lock()
            .map_err(|_| StorageError::Invalid("PDF授权锁异常。"))?;
        let (_, target, path) = file.as_mut().ok_or(StorageError::NotFound)?;
        if target.document_id != record.document_id
            || target.content_hash != record.content_hash
            || target.byte_size != record.byte_size
        {
            return Err(StorageError::Invalid("不能在不同PDF资源之间转交来源授权。"));
        }
        *path = Some(source);
        Ok(())
    }
    pub fn verified_source(&self, id: &str) -> Result<(File, PathBuf, PdfRecord)> {
        let (source, record) = self.source(id)?;
        let job = self
            .jobs
            .lock()
            .map_err(|_| StorageError::Invalid("PDF服务锁异常。"))?
            .get(id)
            .cloned()
            .ok_or(StorageError::NotFound)?;
        let file = verified_copy(&source, &record.content_hash, record.byte_size, &job)?;
        Ok((file, source, record))
    }
    pub fn source(&self, id: &str) -> Result<(PathBuf, PdfRecord)> {
        let job = self
            .jobs
            .lock()
            .map_err(|_| StorageError::Invalid("PDF服务锁异常。"))?
            .get(id)
            .cloned()
            .ok_or(StorageError::NotFound)?;
        job.check()?;
        let guard = job
            .file
            .lock()
            .map_err(|_| StorageError::Invalid("PDF授权锁异常。"))?;
        let (_, record, source) = guard.as_ref().ok_or(StorageError::NotFound)?;
        Ok((
            source.clone().ok_or(StorageError::Invalid(
                "此会话没有原始来源路径；请从Zotero或打开PDF重新选择原件。",
            ))?,
            record.clone(),
        ))
    }
    pub fn record(&self, id: &str) -> Result<PdfRecord> {
        let job = self
            .jobs
            .lock()
            .map_err(|_| StorageError::Invalid("PDF服务锁异常。"))?
            .get(id)
            .cloned()
            .ok_or(StorageError::NotFound)?;
        job.check()?;
        let file = job
            .file
            .lock()
            .map_err(|_| StorageError::Invalid("PDF授权锁异常。"))?;
        Ok(file
            .as_ref()
            .ok_or(StorageError::Invalid("PDF尚未获得读取授权。"))?
            .1
            .clone())
    }
    pub fn read(&self, id: &str, offset: u64, length: u32) -> Result<Vec<u8>> {
        if length == 0 || length > 1024 * 1024 {
            return Err(StorageError::Invalid("PDF读取块超过1MiB。"));
        }
        let job = {
            self.jobs
                .lock()
                .map_err(|_| StorageError::Invalid("PDF服务锁异常。"))?
                .get(id)
                .cloned()
                .ok_or(StorageError::NotFound)?
        };
        job.check()?;
        let mut guard = job
            .file
            .lock()
            .map_err(|_| StorageError::Invalid("PDF授权锁异常。"))?;
        let (file, record, _) = guard
            .as_mut()
            .ok_or(StorageError::Invalid("PDF尚未获得读取授权。"))?;
        if offset
            .checked_add(length as u64)
            .is_none_or(|end| end > record.byte_size)
        {
            return Err(StorageError::Invalid("PDF读取超出文件范围。"));
        }
        let mut bytes = vec![0; length as usize];
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut bytes)?;
        job.check()?;
        Ok(bytes)
    }
}
pub fn import_copy(path: &Path, root: &Path, job: &PdfJob) -> Result<(String, u64)> {
    let mut source = readonly(path)?;
    let total = source.metadata()?.len();
    if total == 0 || total > MAX_PDF_BYTES {
        return Err(StorageError::Invalid("PDF须大于0且不超过128MiB。"));
    }
    job.check()?;
    files::ordinary(root)?;
    let mut stage = tempfile::NamedTempFile::new_in(root)?;
    let mut hash = Sha256::new();
    let mut buf = vec![0; 1024 * 1024];
    let mut loaded = 0;
    loop {
        job.check()?;
        let n = source.read(&mut buf)?;
        if n == 0 {
            break;
        }
        if loaded == 0 && !buf[..n].windows(5).take(1024).any(|v| v == b"%PDF-") {
            return Err(StorageError::Invalid("文件缺少PDF头；未作为PDF导入。"));
        }
        loaded += n as u64;
        if loaded > total || loaded > MAX_PDF_BYTES {
            return Err(StorageError::Invalid("PDF在读取期间改变大小。"));
        }
        hash.update(&buf[..n]);
        stage.write_all(&buf[..n])?;
        job.progress(loaded, total);
    }
    if loaded != total {
        return Err(StorageError::Invalid("PDF在读取期间改变大小。"));
    }
    job.check()?;
    stage.as_file().sync_all()?;
    let hash = format!("{:x}", hash.finalize());
    let target = root.join(format!("{hash}.pdf"));
    if target.try_exists()? {
        let _ = verified_copy(&target, &hash, total, job)?;
    } else {
        match stage.persist_noclobber(&target) {
            Ok(_) => {}
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                let _ = verified_copy(&target, &hash, total, job)?;
            }
            Err(e) => return Err(StorageError::Io(e.error)),
        }
    }
    job.check()?;
    Ok((hash, total))
}
pub fn verified_copy(path: &Path, expected: &str, size: u64, job: &PdfJob) -> Result<File> {
    let mut file = readonly(path)?;
    if file.metadata()?.len() != size || size > MAX_PDF_BYTES {
        return Err(StorageError::Invalid(
            "受管PDF大小不匹配，请保留文件后检查。",
        ));
    }
    let mut hash = Sha256::new();
    let mut buf = vec![0; 1024 * 1024];
    let mut loaded = 0;
    loop {
        job.check()?;
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
        loaded += n as u64;
        job.progress(loaded, size);
    }
    if format!("{:x}", hash.finalize()) != expected {
        return Err(StorageError::Invalid(
            "受管PDF摘要改变，拒绝读取；原件未修改。",
        ));
    }
    file.rewind()?;
    Ok(file)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capability_cancel_and_limits() {
        let s = PdfDocumentService::default();
        let id = "a".repeat(32);
        let j = s.begin(&id).unwrap();
        assert!(s.begin(&id).is_err());
        assert!(s.read(&id, 0, 1).is_err());
        s.begin(&"b".repeat(32)).unwrap();
        s.begin(&"c".repeat(32)).unwrap();
        assert!(s.begin(&"d".repeat(32)).is_err());
        assert!(s.begin("../escape").is_err());
        s.cancel(&id).unwrap();
        assert!(j.check().is_err());
        assert!(s.progress(&id).is_err());
        assert!(
            s.finish(&id, tempfile::tempfile().unwrap(), dummy(), None)
                .is_err()
        );
        s.cancel(&id).unwrap();
    }
    fn dummy() -> PdfRecord {
        PdfRecord {
            document_id: "a".repeat(32),
            attachment_id: "b".repeat(32),
            title: "PDF".into(),
            content_hash: "a".repeat(64),
            byte_size: 12,
            reading: None,
        }
    }
    #[test]
    fn immutable_copy_hash_bounds_and_release() {
        let d = tempfile::tempdir().unwrap();
        let from = d.path().join("source.pdf");
        std::fs::write(&from, b"%PDF-1.7\nabc").unwrap();
        let root = d.path().join("pdfs");
        std::fs::create_dir(&root).unwrap();
        let s = PdfDocumentService::default();
        let id = "a".repeat(32);
        let job = s.begin(&id).unwrap();
        let (h, n) = import_copy(&from, &root, &job).unwrap();
        assert_eq!(n, 12);
        assert_eq!(import_copy(&from, &root, &job).unwrap(), (h.clone(), n));
        let f = verified_copy(&root.join(format!("{h}.pdf")), &h, n, &job).unwrap();
        let mut r = dummy();
        r.content_hash = h;
        s.finish(&id, f, r, None).unwrap();
        assert_eq!(s.read(&id, 0, 5).unwrap(), b"%PDF-");
        assert!(s.read(&id, 11, 2).is_err());
        assert!(s.read(&id, u64::MAX, 1).is_err());
        assert!(s.read(&id, 0, 1024 * 1024 + 1).is_err());
        s.cancel(&id).unwrap();
        assert!(s.read(&id, 0, 1).is_err());
        assert_eq!(std::fs::read(&from).unwrap(), b"%PDF-1.7\nabc");
    }
    #[test]
    fn corrupt_and_pre_cancelled_never_copy() {
        let d = tempfile::tempdir().unwrap();
        let source = d.path().join("bad.pdf");
        std::fs::write(&source, b"not PDF").unwrap();
        let root = d.path().join("pdfs");
        std::fs::create_dir(&root).unwrap();
        let s = PdfDocumentService::default();
        let j = s.begin(&"a".repeat(32)).unwrap();
        assert!(import_copy(&source, &root, &j).is_err());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        s.cancel(&"a".repeat(32)).unwrap();
        assert!(import_copy(&source, &root, &j).is_err());
    }
    #[test]
    fn source_adoption_requires_matching_live_document_and_is_revoked_on_close() {
        let s = PdfDocumentService::default();
        let a = "a".repeat(32);
        let b = "b".repeat(32);
        let c = "c".repeat(32);
        s.begin(&a).unwrap();
        s.begin(&b).unwrap();
        s.begin(&c).unwrap();
        s.finish(
            &a,
            tempfile::tempfile().unwrap(),
            dummy(),
            Some(PathBuf::from("authorized.pdf")),
        )
        .unwrap();
        s.finish(&b, tempfile::tempfile().unwrap(), dummy(), None)
            .unwrap();
        let mut other = dummy();
        other.document_id = "d".repeat(32);
        s.finish(&c, tempfile::tempfile().unwrap(), other, None)
            .unwrap();
        assert!(s.adopt_source(&a, &c).is_err());
        s.adopt_source(&a, &b).unwrap();
        assert_eq!(s.source(&b).unwrap().0, PathBuf::from("authorized.pdf"));
        s.cancel(&a).unwrap();
        assert!(s.adopt_source(&a, &b).is_err());
        s.cancel(&b).unwrap();
        assert!(s.source(&b).is_err());
    }
}
