use crate::{
    capture::{CaptureError, ErrorCode, MAX_PNG_BYTES},
    services::workflow::{decode_png, export_png_new},
    storage::files::{id_valid, ordinary},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const MAX_TEXT: usize = 2 * 1024 * 1024;
pub const MAX_ASSETS: usize = 128 * 1024 * 1024;
pub const MAX_HEADER: usize = MAX_TEXT * 2;
pub fn failure(message: &str) -> CaptureError {
    CaptureError::new(ErrorCode::SaveFailed, message)
}
#[cfg(target_os = "windows")]
pub fn shell_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    let prefix: Vec<u16> = "\\\\?\\".encode_utf16().collect();
    let unc: Vec<u16> = "\\\\?\\UNC\\".encode_utf16().collect();
    let mut result = if wide.starts_with(&unc) {
        [vec![92, 92], wide[unc.len()..].to_vec()].concat()
    } else if wide.starts_with(&prefix) {
        wide[prefix.len()..].to_vec()
    } else {
        wide
    };
    result.push(0);
    result
}
fn io_error() -> CaptureError {
    failure(
        "Markdown 文件操作失败，请检查文件、权限或磁盘空间；原文件未主动删除，可能留下本次未引用的assets。",
    )
}
pub fn asset_name(name: &str) -> bool {
    name.strip_prefix("researchclip-")
        .and_then(|s| s.strip_suffix(".png"))
        .is_some_and(id_valid)
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssetMeta {
    pub name: String,
    pub size: usize,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveInput {
    pub markdown: String,
    pub assets: Vec<AssetMeta>,
    pub token: Option<String>,
    pub save_as: bool,
    #[serde(default)]
    pub file_name: Option<String>,
}
#[derive(Serialize)]
pub struct SavedDocument {
    pub token: String,
    pub path: String,
}
#[derive(Serialize)]
struct OpenInfo {
    token: String,
    path: String,
    markdown: String,
    assets: Vec<AssetMeta>,
}
struct Grant {
    path: PathBuf,
    bytes: Vec<u8>,
}
#[derive(Default)]
pub struct DocumentFiles {
    grants: HashMap<String, Grant>,
    sequence: u64,
    indexed: HashMap<String, PathBuf>,
}
fn names(markdown: &str) -> Result<BTreeSet<String>, CaptureError> {
    let mut result = BTreeSet::new();
    for tail in markdown.split("](assets/").skip(1) {
        let name = tail
            .split_once(')')
            .ok_or_else(|| failure("图片相对引用无效。"))?
            .0;
        if name.starts_with("researchclip-") {
            if !asset_name(name) {
                return Err(failure("图片引用越界或名称无效。"));
            }
            result.insert(name.to_owned());
        }
    }
    if result.len() > 32 {
        return Err(failure("文档最多32张图片。"));
    }
    Ok(result)
}
pub fn normalized_name(input: &str) -> Result<String, CaptureError> {
    let raw = input.trim();
    if raw.is_empty()
        || raw.len() > 1024
        || raw.ends_with(['.', ' '])
        || raw
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err(failure("文件名不能为空或包含路径、控制字符及非法字符。"));
    }
    let name = if raw.to_ascii_lowercase().ends_with(".md") {
        raw.to_owned()
    } else {
        format!("{raw}.md")
    };
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    let device = stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"))
        .is_some_and(|v| ["1", "2", "3", "4", "5", "6", "7", "8", "9", "¹", "²", "³"].contains(&v));
    if name.encode_utf16().count() > 255
        || ["CON", "PRN", "AUX", "NUL", "CLOCK$"].contains(&stem.as_str())
        || device
    {
        return Err(failure("文件名过长或属于 Windows 保留名称。"));
    }
    Ok(name)
}
#[cfg(target_os = "windows")]
fn rename_no_replace(source: &Path, destination: &Path) -> Result<(), CaptureError> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{Win32::Storage::FileSystem::MoveFileW, core::PCWSTR};
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // Both NUL-terminated buffers live for the call. MoveFileW never replaces a destination.
    unsafe { MoveFileW(PCWSTR(source.as_ptr()), PCWSTR(destination.as_ptr())) }
        .map_err(|_| failure("重命名失败：目标可能已存在、文件被占用或目录不可写；请检查原文件。"))
}
#[cfg(not(target_os = "windows"))]
fn rename_no_replace(_source: &Path, _destination: &Path) -> Result<(), CaptureError> {
    Err(failure("当前平台尚未启用安全重命名，请使用另存为。"))
}
pub fn unpack(bytes: &[u8]) -> Result<(SaveInput, Vec<&[u8]>), CaptureError> {
    if bytes.len() < 4 || bytes.len() > 4 + MAX_HEADER + MAX_ASSETS {
        return Err(failure("文档封包大小无效。"));
    }
    let len = u32::from_le_bytes(bytes[..4].try_into().map_err(|_| io_error())?) as usize;
    if len > MAX_HEADER || len + 4 > bytes.len() {
        return Err(failure("文档封包头无效。"));
    }
    let input: SaveInput =
        serde_json::from_slice(&bytes[4..4 + len]).map_err(|_| failure("文档封包格式无效。"))?;
    if let Some(name) = &input.file_name {
        normalized_name(name)?;
    }
    if input.markdown.len() > MAX_TEXT || input.assets.len() > 32 {
        return Err(failure("文档超过2 MiB或32张图片上限。"));
    }
    let mut offset = 4 + len;
    let mut pngs = Vec::new();
    let mut seen = BTreeSet::new();
    for asset in &input.assets {
        if !asset_name(&asset.name)
            || !seen.insert(asset.name.clone())
            || asset.size == 0
            || asset.size > MAX_PNG_BYTES
            || asset.size > bytes.len().saturating_sub(offset)
        {
            return Err(failure("文档图片数据无效。"));
        }
        let png = &bytes[offset..offset + asset.size];
        decode_png(png)?;
        pngs.push(png);
        offset += asset.size;
    }
    if offset != bytes.len() || seen != names(&input.markdown)? {
        return Err(failure("Markdown图片引用与资源不一致，未保存。"));
    }
    Ok((input, pngs))
}
fn read(path: &Path, limit: usize) -> Result<Vec<u8>, CaptureError> {
    ordinary(path).map_err(|_| io_error())?;
    let file = File::open(path).map_err(|_| io_error())?;
    if !file.metadata().map_err(|_| io_error())?.is_file()
        || file.metadata().map_err(|_| io_error())?.len() > limit as u64
    {
        return Err(failure("文档或图片不是普通文件，或超过大小限制。"));
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| io_error())?;
    if bytes.len() > limit {
        return Err(failure("文件超过大小限制。"));
    }
    Ok(bytes)
}
fn target(path: &Path) -> Result<PathBuf, CaptureError> {
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("md"))
    {
        return Err(failure("请选择.md文件。"));
    }
    let parent = path.parent().ok_or_else(io_error)?;
    ordinary(parent).map_err(|_| io_error())?;
    let parent = parent.canonicalize().map_err(|_| io_error())?;
    Ok(parent.join(path.file_name().ok_or_else(io_error)?))
}
fn asset_path(parent: &Path, name: &str) -> Result<PathBuf, CaptureError> {
    if !asset_name(name) {
        return Err(failure("图片名称无效。"));
    }
    let directory = parent.join("assets");
    ordinary(&directory).map_err(|_| failure("assets目录缺失、不可用或存在重解析路径。"))?;
    if directory.canonicalize().map_err(|_| io_error())?.parent() != Some(parent) {
        return Err(failure("图片目录超出所选文档文件夹。"));
    }
    let path = directory.join(name);
    if path.try_exists().map_err(|_| io_error())? {
        ordinary(&path).map_err(|_| io_error())?;
    }
    Ok(path)
}
impl DocumentFiles {
    fn capacity(&self) -> Result<(), CaptureError> {
        if self.grants.len() >= 4 {
            Err(failure("文档文件授权数量超限，请关闭文档后重试。"))
        } else {
            Ok(())
        }
    }
    fn remember(&mut self, path: PathBuf, bytes: Vec<u8>) -> SavedDocument {
        self.sequence = self.sequence.wrapping_add(1);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let token = format!("{stamp:x}-{:x}", self.sequence);
        let result = SavedDocument {
            token: token.clone(),
            path: path.to_string_lossy().into(),
        };
        self.grants.insert(token, Grant { path, bytes });
        result
    }
    pub fn release(&mut self, token: &str) {
        self.grants.remove(token);
        self.indexed.remove(token);
    }
    pub fn index_paths(&self, token: &str) -> Result<(PathBuf, Option<PathBuf>), CaptureError> {
        let path = self
            .grants
            .get(token)
            .ok_or_else(|| failure("文档授权已失效。"))?
            .path
            .clone();
        Ok((path, self.indexed.get(token).cloned()))
    }
    pub fn external_path(&self, token: &str) -> Result<PathBuf, CaptureError> {
        if token.len() > 64 {
            return Err(failure("文档授权无效。"));
        }
        self.unchanged(token)?;
        Ok(self
            .grants
            .get(token)
            .ok_or_else(|| failure("文档授权已失效。"))?
            .path
            .clone())
    }
    pub fn mark_indexed(&mut self, token: &str, path: PathBuf) {
        self.indexed.insert(token.into(), path);
    }
    pub fn rename(&mut self, token: &str, file_name: &str) -> Result<SavedDocument, CaptureError> {
        let name = normalized_name(file_name)?;
        self.unchanged(token)?;
        let old = self
            .grants
            .get(token)
            .ok_or_else(|| failure("文档授权已失效。"))?
            .path
            .clone();
        if old.file_name() == Some(std::ffi::OsStr::new(&name)) {
            return Ok(SavedDocument {
                token: token.into(),
                path: old.to_string_lossy().into(),
            });
        }
        let destination = target(&old.parent().ok_or_else(io_error)?.join(name))?;
        if destination.try_exists().map_err(|_| io_error())? {
            return Err(failure(
                "同名文件已存在（Windows不区分大小写），请使用不同名称；未覆盖文件。",
            ));
        }
        rename_no_replace(&old, &destination)?;
        self.grants.get_mut(token).ok_or_else(io_error)?.path = destination.clone();
        Ok(SavedDocument {
            token: token.into(),
            path: destination.to_string_lossy().into(),
        })
    }
    pub fn open(&mut self, path: &Path) -> Result<Vec<u8>, CaptureError> {
        self.capacity()?;
        let path = target(path)?;
        let bytes = read(&path, MAX_TEXT)?;
        let markdown = String::from_utf8(bytes.clone())
            .map_err(|_| failure("Markdown不是有效UTF-8，未替换当前文档。"))?;
        let mut pngs = Vec::new();
        let mut assets = Vec::new();
        let mut total = 0;
        for name in names(&markdown)? {
            let png = read(
                &asset_path(path.parent().ok_or_else(io_error)?, &name)?,
                MAX_PNG_BYTES,
            )?;
            total += png.len();
            if total > MAX_ASSETS {
                return Err(failure("图片合计超过128 MiB。"));
            }
            decode_png(&png)?;
            assets.push(AssetMeta {
                name,
                size: png.len(),
            });
            pngs.push(png);
        }
        let saved = self.remember(path, bytes);
        let meta = serde_json::to_vec(&OpenInfo {
            token: saved.token.clone(),
            path: saved.path,
            markdown,
            assets,
        })
        .map_err(|_| io_error())?;
        if meta.len() > MAX_HEADER {
            self.release(&saved.token);
            return Err(failure("文档元数据超过限制。"));
        }
        let mut output = Vec::with_capacity(4 + meta.len() + total);
        output.extend_from_slice(&(meta.len() as u32).to_le_bytes());
        output.extend(meta);
        for png in pngs {
            output.extend(png);
        }
        Ok(output)
    }
    pub fn save(
        &mut self,
        selected: Option<&Path>,
        input: &SaveInput,
        pngs: &[&[u8]],
    ) -> Result<SavedDocument, CaptureError> {
        let updating = selected.is_none();
        let path = if let Some(path) = selected {
            self.capacity()?;
            let path = target(path)?;
            if path.try_exists().map_err(|_| io_error())? {
                return Err(failure(
                    "同名Markdown文件已存在，请另选新名称；已有文件未覆盖。",
                ));
            }
            path
        } else {
            self.grants
                .get(input.token.as_deref().unwrap_or(""))
                .ok_or_else(|| failure("文档文件授权已失效，请另存为新文件。"))?
                .path
                .clone()
        };
        if updating {
            self.unchanged(input.token.as_deref().unwrap_or(""))?;
        }
        // Validate every image before creating anything. Existing immutable resources are compared, never overwritten.
        if pngs.len() != input.assets.len()
            || input.markdown.len() > MAX_TEXT
            || input.assets.len() > 32
            || input
                .assets
                .iter()
                .map(|a| a.name.clone())
                .collect::<BTreeSet<_>>()
                != names(&input.markdown)?
        {
            return Err(failure("图片关联不完整。"));
        }
        for png in pngs {
            decode_png(png)?;
        }
        if !input.assets.is_empty() {
            let directory = path.parent().ok_or_else(io_error)?.join("assets");
            match fs::create_dir(&directory) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(io_error()),
            }
            for (asset, png) in input.assets.iter().zip(pngs) {
                let destination = asset_path(path.parent().ok_or_else(io_error)?, &asset.name)?;
                if destination.try_exists().map_err(|_| io_error())? {
                    if read(&destination, MAX_PNG_BYTES)? != *png {
                        return Err(failure(
                            "同名assets资源内容不同，已拒绝覆盖；原Markdown未改动。",
                        ));
                    }
                } else {
                    export_png_new(&destination, png)?;
                }
            }
        }
        let mut stage = tempfile::NamedTempFile::new_in(path.parent().ok_or_else(io_error)?)
            .map_err(|_| io_error())?;
        stage
            .write_all(input.markdown.as_bytes())
            .and_then(|_| stage.as_file().sync_all())
            .map_err(|_| io_error())?;
        if updating {
            self.unchanged(input.token.as_deref().unwrap_or(""))?;
            stage.persist(&path).map_err(|_| io_error())?;
            let token = input.token.clone().ok_or_else(io_error)?;
            self.grants.get_mut(&token).ok_or_else(io_error)?.bytes =
                input.markdown.as_bytes().to_vec();
            Ok(SavedDocument {
                token,
                path: path.to_string_lossy().into(),
            })
        } else {
            stage.persist_noclobber(&path).map_err(|_| {
                failure("Markdown提交失败或目标已存在，已有文件未覆盖；已写资源保留。")
            })?;
            Ok(self.remember(path, input.markdown.as_bytes().to_vec()))
        }
    }
    fn unchanged(&self, token: &str) -> Result<(), CaptureError> {
        let grant = self
            .grants
            .get(token)
            .ok_or_else(|| failure("文档授权已失效。"))?;
        if read(&grant.path, MAX_TEXT)? != grant.bytes {
            return Err(failure(
                "文件已被其他程序修改，未覆盖。请另存为，或保留草稿后重新打开。",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{capture::RawFrame, services::screen_capture::encode_png};
    #[test]
    fn external_open_requires_live_unchanged_grant() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("数学 研究.md");
        let source = "# 中文\r\n\r\n$\\dot{x}=Ax+Bu$\r\n\n$$\n\\begin{bmatrix}1&2\\\\3&4\\end{bmatrix}\n$$\n";
        let mut files = DocumentFiles::default();
        assert!(files.external_path("../not-authorized.md").is_err());
        let mut data = input();
        data.markdown = source.into();
        let saved = files.save(Some(&path), &data, &[]).unwrap();
        assert_eq!(
            files.external_path(&saved.token).unwrap(),
            path.canonicalize().unwrap()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
        fs::write(&path, "changed externally").unwrap();
        assert!(files.external_path(&saved.token).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "changed externally");
        files.release(&saved.token);
        assert!(files.external_path(&saved.token).is_err());
        assert!(files.external_path(&"x".repeat(65)).is_err());
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn shell_paths_preserve_unicode_and_unc_without_extended_prefix() {
        for (input, expected) in [
            (r"\\?\C:\研究 文件\数学.md", r"C:\研究 文件\数学.md"),
            (r"\\?\UNC\server\share\文档.md", r"\\server\share\文档.md"),
            (r"C:\normal.md", r"C:\normal.md"),
        ] {
            let value = shell_path(Path::new(input));
            assert_eq!(
                String::from_utf16(&value[..value.len() - 1]).unwrap(),
                expected
            );
            assert_eq!(value.last(), Some(&0));
        }
    }
    fn png() -> Vec<u8> {
        encode_png(RawFrame {
            width_px: 2,
            height_px: 1,
            rgba: vec![255, 0, 0, 0, 0, 255, 0, 255],
        })
        .unwrap()
    }
    fn input() -> SaveInput {
        SaveInput {
            markdown: "# 中文 $x_1$\n".into(),
            assets: vec![],
            token: None,
            save_as: false,
            file_name: None,
        }
    }
    #[test]
    fn safe_markdown_names_reject_paths_devices_and_invalid_characters() {
        assert_eq!(normalized_name("  研究 note  ").unwrap(), "研究 note.md");
        assert_eq!(normalized_name("notes.MD").unwrap(), "notes.MD");
        for name in [
            "", "../x", "C:\\x", "x/y", "a:b", "a?b", "name.", "a\0b", "CON", "con.txt", "LPT1",
            "COM¹",
        ] {
            assert!(normalized_name(name).is_err(), "{name}");
        }
        assert!(normalized_name(&"🙂".repeat(127)).is_err());
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn rename_keeps_assets_and_grant_and_supports_subsequent_save() {
        let d = tempfile::tempdir().unwrap();
        let old = d.path().join("old.md");
        let mut files = DocumentFiles::default();
        let mut v = input();
        let name = format!("researchclip-{}.png", "d".repeat(32));
        let image = png();
        v.markdown = format!("![image](assets/{name})\n\n原注释");
        v.assets.push(AssetMeta {
            name: name.clone(),
            size: image.len(),
        });
        let saved = files.save(Some(&old), &v, &[&image]).unwrap();
        let result = files.rename(&saved.token, "研究 重命名").unwrap();
        let new = PathBuf::from(&result.path);
        assert!(!old.exists());
        assert_eq!(new.file_name().unwrap(), "研究 重命名.md");
        assert_eq!(fs::read_to_string(&new).unwrap(), v.markdown);
        assert_eq!(fs::read(d.path().join("assets").join(name)).unwrap(), image);
        v.token = Some(saved.token);
        v.markdown.push_str("\nnew content");
        files.save(None, &v, &[&image]).unwrap();
        assert_eq!(fs::read_to_string(new).unwrap(), v.markdown);
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn rename_conflict_external_change_and_invalid_token_preserve_files() {
        let d = tempfile::tempdir().unwrap();
        let old = d.path().join("old.md");
        let other = d.path().join("other.md");
        let mut files = DocumentFiles::default();
        let saved = files.save(Some(&old), &input(), &[]).unwrap();
        fs::write(&other, "other user's content").unwrap();
        assert!(files.rename(&saved.token, "other").is_err());
        assert!(rename_no_replace(&old, &other).is_err());
        assert_eq!(fs::read_to_string(&other).unwrap(), "other user's content");
        assert_eq!(fs::read_to_string(&old).unwrap(), input().markdown);
        assert!(files.rename(&saved.token, "../escape").is_err());
        assert!(files.rename("fake", "new").is_err());
        fs::write(&old, "external").unwrap();
        assert!(files.rename(&saved.token, "new").is_err());
        assert_eq!(fs::read_to_string(old).unwrap(), "external");
        assert!(!d.path().join("new.md").exists());
    }
    #[test]
    fn new_save_reopen_update_and_utf8() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("研究 notes.md");
        let mut s = DocumentFiles::default();
        let mut v = input();
        let first = s.save(Some(&p), &v, &[]).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), v.markdown);
        let packet = s.open(&p).unwrap();
        assert!(packet.len() > 4);
        v.token = Some(first.token);
        v.markdown.push_str("**next**\n");
        s.save(None, &v, &[]).unwrap();
        assert_eq!(fs::read_to_string(p).unwrap(), v.markdown);
    }
    #[test]
    fn no_overwrite_and_external_conflict() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("notes.md");
        let mut s = DocumentFiles::default();
        let mut v = input();
        v.token = Some(s.save(Some(&p), &v, &[]).unwrap().token);
        assert!(s.save(Some(&p), &v, &[]).is_err());
        fs::write(&p, "external").unwrap();
        assert!(
            s.save(None, &v, &[])
                .err()
                .unwrap()
                .message
                .contains("其他程序")
        );
        assert_eq!(fs::read_to_string(p).unwrap(), "external");
    }
    #[test]
    fn assets_relative_stable_and_survive_folder_move() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a");
        fs::create_dir(&a).unwrap();
        let p = a.join("notes.md");
        let name = format!("researchclip-{}.png", "a".repeat(32));
        let image = png();
        let mut v = input();
        v.markdown = format!("# 文档\n\n![a](assets/{name})\n\n注释\n");
        v.assets.push(AssetMeta {
            name: name.clone(),
            size: image.len(),
        });
        let mut s = DocumentFiles::default();
        s.save(Some(&p), &v, &[&image]).unwrap();
        assert_eq!(fs::read(a.join("assets").join(&name)).unwrap(), image);
        let b = d.path().join("moved 中文");
        fs::rename(a, &b).unwrap();
        assert!(s.open(&b.join("notes.md")).is_ok());
    }
    #[test]
    fn conflicting_asset_cannot_damage_document() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("notes.md");
        let mut s = DocumentFiles::default();
        let mut v = input();
        v.token = Some(s.save(Some(&p), &v, &[]).unwrap().token);
        let name = format!("researchclip-{}.png", "b".repeat(32));
        fs::create_dir(d.path().join("assets")).unwrap();
        fs::write(d.path().join("assets").join(&name), b"other user's file").unwrap();
        v.markdown = format!("![b](assets/{name})\n\ncaption");
        v.assets.push(AssetMeta {
            name: name.clone(),
            size: png().len(),
        });
        assert!(s.save(None, &v, &[&png()]).is_err());
        assert_eq!(fs::read_to_string(&p).unwrap(), input().markdown);
        assert_eq!(
            fs::read(d.path().join("assets").join(name)).unwrap(),
            b"other user's file"
        );
    }
    #[test]
    fn malformed_upload_and_paths_are_rejected() {
        for bytes in [&[][..], &[1, 2, 3][..], &[255, 255, 255, 255][..]] {
            assert!(unpack(bytes).is_err());
        }
        assert!(!asset_name("../x.png"));
        assert!(!asset_name("researchclip-../../x.png"));
        assert!(names("](assets/researchclip-../../x.png)").is_err());
        let mut s = DocumentFiles::default();
        assert!(
            s.save(Some(Path::new("relative.md")), &input(), &[])
                .is_err()
        );
        assert!(s.save(None, &input(), &[]).is_err());
    }
    #[test]
    fn failed_write_missing_image_and_invalid_utf8_preserve_existing_files() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("notes.md");
        fs::write(&p, [0xff]).unwrap();
        let mut s = DocumentFiles::default();
        assert!(s.open(&p).is_err());
        assert_eq!(fs::read(&p).unwrap(), [0xff]);
        let name = format!("researchclip-{}.png", "c".repeat(32));
        fs::write(&p, format!("![missing](assets/{name})")).unwrap();
        assert!(s.open(&p).is_err());
        let mut v = input();
        v.markdown = format!("![a](assets/{name})");
        v.assets.push(AssetMeta {
            name,
            size: png().len(),
        });
        fs::write(d.path().join("assets"), b"keep").unwrap();
        assert!(
            s.save(Some(&d.path().join("new.md")), &v, &[&png()])
                .is_err()
        );
        assert!(!d.path().join("new.md").exists());
        assert_eq!(fs::read(d.path().join("assets")).unwrap(), b"keep");
    }
}
