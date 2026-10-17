//! Templates are independent standard Markdown + assets, confined to a versioned config directory.
use super::markdown_document::{DocumentFiles, failure, normalized_name, unpack};
use crate::{
    capture::CaptureError,
    storage::files::{directory, id_valid, ordinary},
};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
}
pub struct TemplateFiles {
    root: PathBuf,
}
impl TemplateFiles {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn root(&self) -> Result<PathBuf, CaptureError> {
        directory(&self.root)
            .map_err(|_| failure("模板目录不可用或存在重解析路径；未修改模板。"))?;
        self.root
            .canonicalize()
            .map_err(|_| failure("无法读取模板目录。"))
    }
    fn folder(&self, id: &str) -> Result<PathBuf, CaptureError> {
        if !id_valid(id) {
            return Err(failure("模板 ID 无效。"));
        }
        let root = self.root()?;
        let folder = root.join(id);
        ordinary(&folder).map_err(|_| failure("模板不存在或目录不安全。"))?;
        if !folder.is_dir()
            || folder
                .canonicalize()
                .map_err(|_| failure("模板目录不可用。"))?
                .parent()
                != Some(root.as_path())
        {
            return Err(failure("模板目录越界。"));
        }
        Ok(folder)
    }
    fn path(&self, id: &str) -> Result<PathBuf, CaptureError> {
        let folder = self.folder(id)?;
        let mut paths = Vec::new();
        for entry in fs::read_dir(folder).map_err(|_| failure("无法读取模板。"))? {
            let path = entry.map_err(|_| failure("无法读取模板。"))?.path();
            if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("md"))
            {
                ordinary(&path).map_err(|_| failure("模板不是安全的普通文件。"))?;
                paths.push(path);
            }
            if paths.len() > 1 {
                return Err(failure(
                    "模板目录含多个 Markdown 文件，请保留原文件并检查。",
                ));
            }
        }
        paths
            .pop()
            .ok_or_else(|| failure("模板文件缺失；未重置模板目录。"))
    }
    pub fn list(&self) -> Result<Vec<TemplateInfo>, CaptureError> {
        let mut result = Vec::new();
        let mut count = 0;
        for entry in fs::read_dir(self.root()?).map_err(|_| failure("无法读取模板列表。"))?
        {
            count += 1;
            if count > 1024 {
                return Err(failure("模板目录条目超过1024，请检查目录。"));
            }
            let entry = entry.map_err(|_| failure("无法读取模板列表。"))?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if !id_valid(&id) {
                continue;
            }
            let path = self.path(&id)?;
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| failure("模板名称无效。"))?
                .to_owned();
            result.push(TemplateInfo { id, name });
            if result.len() > 100 {
                return Err(failure("自定义模板超过100个。"));
            }
        }
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }
    fn unique_name(&self, name: &str, exclude: Option<&str>) -> Result<String, CaptureError> {
        let file = normalized_name(name)?;
        let stem = Path::new(&file)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| failure("模板名称无效。"))?;
        if stem.trim().is_empty() || stem.chars().count() > 80 || stem == ".md" {
            return Err(failure("模板名称需为1至80个字符，不能只有扩展名。"));
        }
        if self
            .list()?
            .iter()
            .any(|t| Some(t.id.as_str()) != exclude && t.name.to_lowercase() == stem.to_lowercase())
        {
            return Err(failure("同名模板已存在，请使用新名称；不会覆盖已有模板。"));
        }
        Ok(file)
    }
    pub fn create(&self, id: &str, bytes: &[u8]) -> Result<(), CaptureError> {
        if !id_valid(id) {
            return Err(failure("模板 ID 无效。"));
        }
        let (input, pngs) = unpack(bytes)?;
        if input.token.is_some() || input.save_as {
            return Err(failure("模板不能使用文档文件授权。"));
        }
        let name = self.unique_name(
            input
                .file_name
                .as_deref()
                .ok_or_else(|| failure("缺少模板名称。"))?,
            None,
        )?;
        if self.list()?.len() >= 100 {
            return Err(failure("最多100个自定义模板。"));
        }
        let root = self.root()?;
        let target = root.join(id);
        if target
            .try_exists()
            .map_err(|_| failure("无法检查模板目标。"))?
        {
            return Err(failure("模板 ID 已存在，未覆盖。"));
        }
        let stage = tempfile::Builder::new()
            .prefix(".pending-")
            .tempdir_in(&root)
            .map_err(|_| failure("无法创建模板暂存目录。"))?;
        let mut files = DocumentFiles::default();
        files.save(Some(&stage.path().join(name)), &input, &pngs)?;
        // Unique identifier and an absent destination; Windows refuses an existing directory.
        fs::rename(stage.path(), &target).map_err(|_| failure("模板提交失败，已有模板未覆盖。"))?;
        Ok(())
    }
    pub fn load(&self, id: &str) -> Result<Vec<u8>, CaptureError> {
        DocumentFiles::default().open(&self.path(id)?)
    }
    pub fn rename(&self, id: &str, name: &str) -> Result<(), CaptureError> {
        let name = self.unique_name(name, Some(id))?;
        let path = self.path(id)?;
        let mut files = DocumentFiles::default();
        let data = files.open(&path)?;
        let size = u32::from_le_bytes(
            data[..4]
                .try_into()
                .map_err(|_| failure("模板封包无效。"))?,
        ) as usize;
        let meta: serde_json::Value =
            serde_json::from_slice(&data[4..4 + size]).map_err(|_| failure("模板封包无效。"))?;
        files.rename(
            meta["token"]
                .as_str()
                .ok_or_else(|| failure("模板授权无效。"))?,
            &name,
        )?;
        Ok(())
    }
    pub fn delete(&self, id: &str) -> Result<(), CaptureError> {
        // Atomically hide the complete template, retaining its files for safe manual recovery.
        // No recursive removal and no possibility of deleting shared document assets.
        let folder = self.folder(id)?;
        self.path(id)?;
        let destination = self.root()?.join(format!("deleted-{id}"));
        if destination
            .try_exists()
            .map_err(|_| failure("无法检查模板归档位置。"))?
        {
            return Err(failure("模板归档已存在，未覆盖。"));
        }
        fs::rename(folder, destination).map_err(|_| failure("删除模板失败；模板文件已保留。"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn images_are_copied_into_independent_template_assets_and_failures_leave_no_visible_template() {
        use crate::{capture::RawFrame, services::screen_capture::encode_png};
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("templates-v1");
        let t = TemplateFiles::new(root.clone());
        let png = encode_png(RawFrame {
            width_px: 1,
            height_px: 1,
            rgba: vec![20, 40, 60, 128],
        })
        .unwrap();
        let name = format!("researchclip-{}.png", "d".repeat(32));
        let markdown = format!("# 图像\n\n![中文](assets/{name})\n\n$\\alpha$");
        let header=serde_json::to_vec(&serde_json::json!({"markdown":markdown,"assets":[{"name":name,"size":png.len()}],"token":null,"saveAs":false,"fileName":"图像模板"})).unwrap();
        let mut bytes = (header.len() as u32).to_le_bytes().to_vec();
        bytes.extend(header);
        bytes.extend(&png);
        let id = "d".repeat(32);
        t.create(&id, &bytes).unwrap();
        assert_eq!(
            fs::read(root.join(&id).join("assets").join(&name)).unwrap(),
            png
        );
        let loaded = t.load(&id).unwrap();
        assert!(loaded.ends_with(&png));
        let before = fs::read(root.join(&id).join("图像模板.md")).unwrap();
        bytes.pop();
        assert!(t.create(&"e".repeat(32), &bytes).is_err());
        assert_eq!(t.list().unwrap().len(), 1);
        assert_eq!(
            fs::read(root.join(&id).join("图像模板.md")).unwrap(),
            before
        );
        let blocked = dir.path().join("blocked");
        fs::write(&blocked, b"keep").unwrap();
        assert!(TemplateFiles::new(blocked.clone()).list().is_err());
        assert_eq!(fs::read(blocked).unwrap(), b"keep");
    }
    fn packet(name: &str, text: &str) -> Vec<u8> {
        let header=serde_json::to_vec(&serde_json::json!({"markdown":text,"assets":[],"token":null,"saveAs":false,"fileName":name})).unwrap();
        let mut out = (header.len() as u32).to_le_bytes().to_vec();
        out.extend(header);
        out
    }
    #[test]
    fn persist_rename_delete_and_preserve_other_documents() {
        let dir = tempfile::tempdir().unwrap();
        let external = dir.path().join("original.md");
        fs::write(&external, "original").unwrap();
        let root = dir.path().join("templates-v1");
        let t = TemplateFiles::new(root.clone());
        let id = "a".repeat(32);
        t.create(&id, &packet("中文模板", "# 中文\n\n$\\alpha$ **English**"))
            .unwrap();
        assert_eq!(t.list().unwrap()[0].name, "中文模板");
        let restarted = TemplateFiles::new(root.clone());
        assert!(restarted.load(&id).unwrap().len() > 40);
        #[cfg(windows)]
        {
            restarted.rename(&id, "新模板").unwrap();
            assert_eq!(restarted.list().unwrap()[0].name, "新模板");
        }
        restarted.delete(&id).unwrap();
        assert!(restarted.list().unwrap().is_empty());
        assert!(root.join(format!("deleted-{id}")).is_dir());
        assert_eq!(fs::read_to_string(external).unwrap(), "original");
    }
    #[test]
    fn rejects_conflicts_paths_invalid_packets_and_capacity_preserves_templates() {
        let dir = tempfile::tempdir().unwrap();
        let t = TemplateFiles::new(dir.path().join("templates-v1"));
        let id = "a".repeat(32);
        let bytes = packet("Note", "# original");
        t.create(&id, &bytes).unwrap();
        assert!(t.create(&id, &packet("other", "changed")).is_err());
        assert!(
            t.create(&"b".repeat(32), &packet("note", "overwrite"))
                .is_err()
        );
        assert!(t.create("../unsafe", &bytes).is_err());
        assert!(t.create(&"b".repeat(32), &packet("../name", "x")).is_err());
        assert!(t.create(&"b".repeat(32), b"invalid").is_err());
        assert!(t.load("../unsafe").is_err());
        assert_eq!(t.list().unwrap().len(), 1);
        for n in 1..100 {
            t.create(&format!("{n:032x}"), &packet(&format!("template{n}"), ""))
                .unwrap();
        }
        assert!(t.create(&"c".repeat(32), &packet("overflow", "")).is_err());
        assert_eq!(t.list().unwrap().len(), 100);
    }
}
