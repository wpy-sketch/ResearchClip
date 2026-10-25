//! Native argv offers are capabilities; the renderer only receives opaque offer IDs.
use crate::storage::StorageError;
use serde::Serialize;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
static SEQUENCE: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfOpenOffer {
    pub id: String,
    pub title: String,
}
struct Pending {
    offer: PdfOpenOffer,
    path: PathBuf,
}
#[derive(Default)]
pub struct PdfOpenRequests {
    pending: Mutex<VecDeque<Pending>>,
}
type Result<T> = std::result::Result<T, StorageError>;
pub(super) fn ephemeral() -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64;
    format!(
        "{stamp:016x}{:016x}",
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}
pub fn pdf_arguments(args: &[String], cwd: &Path) -> Result<Vec<PathBuf>> {
    if args.len() > 18 {
        return Err(StorageError::Invalid("一次最多接收16份外部PDF。"));
    }
    let mut paths = Vec::new();
    let mut literal = false;
    for arg in args.iter().skip(1) {
        if arg == "--" && !literal {
            literal = true;
            continue;
        }
        if arg.is_empty()
            || arg.len() > 32760
            || arg.contains('\0')
            || (!literal && arg.starts_with('-'))
            || arg.contains("://")
        {
            return Err(StorageError::Invalid(
                "外部打开参数不受支持；只接收PDF文件路径。",
            ));
        }
        let path = PathBuf::from(arg);
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        {
            return Err(StorageError::Invalid("外部打开只支持.pdf文件。"));
        }
        let path = if path.is_absolute() {
            path
        } else {
            cwd.join(path)
        };
        // Validating all offers first prevents a bad argument partially opening earlier files.
        crate::storage::files::ordinary(&path)?;
        if !path.is_file() {
            return Err(StorageError::Invalid("外部PDF文件不存在或不是普通文件。"));
        }
        let canonical = path.canonicalize()?;
        if !paths.contains(&canonical) {
            paths.push(canonical);
        }
    }
    Ok(paths)
}
impl PdfOpenRequests {
    pub fn enqueue(&self, paths: Vec<PathBuf>) -> Result<usize> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| StorageError::Invalid("外部PDF队列锁异常。"))?;
        let paths = paths
            .into_iter()
            .filter(|path| !pending.iter().any(|item| &item.path == path))
            .collect::<Vec<_>>();
        if pending.len() + paths.len() > 16 {
            return Err(StorageError::Invalid(
                "外部PDF请求队列已满，请先处理当前请求。",
            ));
        }
        for path in paths {
            let title = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .chars()
                .filter(|c| !c.is_control())
                .take(160)
                .collect();
            pending.push_back(Pending {
                offer: PdfOpenOffer {
                    id: ephemeral(),
                    title,
                },
                path,
            });
        }
        Ok(pending.len())
    }
    pub fn list(&self) -> Result<Vec<PdfOpenOffer>> {
        Ok(self
            .pending
            .lock()
            .map_err(|_| StorageError::Invalid("外部PDF队列锁异常。"))?
            .iter()
            .map(|item| item.offer.clone())
            .collect())
    }
    pub fn take(&self, id: &str) -> Result<PathBuf> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| StorageError::Invalid("外部PDF队列锁异常。"))?;
        let index = pending
            .iter()
            .position(|item| item.offer.id == id)
            .ok_or(StorageError::NotFound)?;
        Ok(pending.remove(index).ok_or(StorageError::NotFound)?.path)
    }
    pub fn dismiss(&self, id: &str) -> Result<()> {
        self.take(id).map(|_| ())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arguments_preserve_legal_special_characters_and_do_not_execute_them() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("中文 空格 [A] & (B) ' # +.PDF");
        std::fs::write(&path, b"%PDF-1.7").unwrap();
        let args = vec![
            "researchclip.exe".into(),
            path.to_string_lossy().into_owned(),
        ];
        let parsed = pdf_arguments(&args, d.path()).unwrap();
        assert_eq!(parsed, [path.canonicalize().unwrap()]);
        assert!(pdf_arguments(&[args[0].clone(), "--run=shell".into()], d.path()).is_err());
        assert!(
            pdf_arguments(
                &[args[0].clone(), "https://example.invalid/a.pdf".into()],
                d.path()
            )
            .is_err()
        );
        assert!(pdf_arguments(&[args[0].clone(), "zotero.sqlite".into()], d.path()).is_err());
    }
    #[test]
    fn offers_are_bounded_deduplicated_and_consumed_once() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("file.pdf");
        std::fs::write(&path, b"%PDF-1.7").unwrap();
        let queue = PdfOpenRequests::default();
        assert_eq!(queue.enqueue(vec![path.clone()]).unwrap(), 1);
        assert_eq!(queue.enqueue(vec![path.clone()]).unwrap(), 1);
        let offer = queue.list().unwrap().remove(0);
        assert_eq!(offer.id.len(), 32);
        assert_eq!(queue.take(&offer.id).unwrap(), path);
        assert!(queue.take(&offer.id).is_err());
        assert_eq!(queue.list().unwrap().len(), 0);
        assert!(
            queue
                .enqueue((0..17).map(|i| d.path().join(format!("{i}.pdf"))).collect())
                .is_err()
        );
        assert!(queue.list().unwrap().is_empty());
    }
}
