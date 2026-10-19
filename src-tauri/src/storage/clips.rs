//! Uses the existing Markdown index. No second note source or schema change.
use super::{Storage, StorageError, files, library::now};
use crate::services::markdown_document::{AssetMeta, DocumentFiles, SaveInput, normalized_name};
use researchclip_clipper_protocol::{Clip, Request, hash};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClipSaved {
    pub document_id: String,
    pub title: String,
    pub duplicate: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    digest: String,
    created_at: i64,
}
fn invalid() -> StorageError {
    StorageError::Invalid("剪藏文件或请求状态异常，已有文件未覆盖。请保留现场后重试。")
}
fn read(path: &Path, max: usize) -> Result<Vec<u8>, StorageError> {
    files::ordinary(path)?;
    let f = fs::File::open(path)?;
    if !f.metadata()?.is_file() || f.metadata()?.len() > max as u64 {
        return Err(invalid());
    }
    let mut b = Vec::new();
    f.take(max as u64 + 1).read_to_end(&mut b)?;
    if b.len() > max {
        return Err(invalid());
    }
    Ok(b)
}
fn inline(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .flat_map(|c| {
            if "\\`*_{}[]<>()#+-!|".contains(c) {
                vec!['\\', c]
            } else {
                vec![c]
            }
        })
        .collect()
}
fn filename(title: &str, id: &str) -> Result<String, StorageError> {
    let clean: String = title
        .chars()
        .take(70)
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '-'
            } else {
                c
            }
        })
        .collect();
    normalized_name(&format!(
        "clip-{}-{}",
        clean.trim_end_matches(['.', ' ']),
        &id[..8]
    ))
    .map_err(|_| invalid())
}
type RenderedClip = (String, Vec<Vec<u8>>, Vec<AssetMeta>);
fn markdown(clip: &Clip, created: i64) -> Result<RenderedClip, StorageError> {
    let mut body = clip.markdown.clone();
    let mut pngs = Vec::new();
    let mut assets = Vec::new();
    let mut warnings = Vec::new();
    for image in &clip.images {
        let source_url = researchclip_clipper_protocol::canonical_url(&image.url)
            .map_err(StorageError::Invalid)?;
        let reference = if let Some(png) = clip.png(image).map_err(|_| invalid())? {
            let frame = crate::services::workflow::decode_png(&png)
                .map_err(|_| StorageError::Invalid("剪藏图片解码失败。"))?;
            if frame.width_px as u64 * frame.height_px as u64 > 4_194_304 {
                return Err(StorageError::Invalid("剪藏单图超过419万像素。"));
            }
            let name = format!("researchclip-{}.png", image.id);
            assets.push(AssetMeta {
                name: name.clone(),
                size: png.len(),
            });
            pngs.push(png);
            format!("assets/{name}")
        } else {
            warnings.push(format!(
                "- 图片未本地复制：[来源](<{}>){}",
                source_url,
                image
                    .warning
                    .as_ref()
                    .map(|s| format!(" — {}", inline(s)))
                    .unwrap_or_default()
            ));
            format!("<{source_url}>")
        };
        let marker = format!("clip-image://{}", image.id);
        if !body.contains(&marker) {
            return Err(StorageError::Invalid("图片与正文关联不完整。"));
        }
        body = body.replace(&marker, &reference);
    }
    if body.contains("clip-image://") {
        return Err(StorageError::Invalid("正文包含未知图片引用。"));
    }
    let warning = if warnings.is_empty() {
        String::new()
    } else {
        format!("\n\n## 图片资源状态\n\n{}", warnings.join("\n"))
    };
    Ok((
        format!(
            "# {}\n\n{}\n\n## 来源\n\n[原网页](<{}>)\n\n站点：{}\n\n作者：{}\n\n剪藏时间（Unix毫秒）：{created}{warning}\n",
            inline(&clip.title),
            body,
            researchclip_clipper_protocol::canonical_url(&clip.url)
                .map_err(StorageError::Invalid)?,
            inline(&clip.site_name),
            inline(&clip.byline)
        ),
        pngs,
        assets,
    ))
}
impl Storage {
    pub fn save_web_clip(&mut self, request: &Request) -> Result<ClipSaved, StorageError> {
        request.validate().map_err(StorageError::Invalid)?;
        let clip = request
            .clip
            .as_ref()
            .filter(|_| request.operation == "save")
            .ok_or_else(invalid)?;
        let digest = hash(&serde_json::to_vec(request).map_err(|_| invalid())?);
        let root = self.data_dir.join("web-clips-v1");
        files::directory(&root)?;
        let folder = root.join(&request.request_id);
        files::directory(&folder)?;
        if folder.canonicalize()?.parent() != Some(root.as_path()) {
            return Err(invalid());
        }
        let receipt_path = folder.join("request.json");
        files::existing_ordinary(&receipt_path)?;
        let receipt: Receipt = if receipt_path.try_exists()? {
            serde_json::from_slice(&read(&receipt_path, 4096)?).map_err(|_| invalid())?
        } else {
            let r = Receipt {
                digest: digest.clone(),
                created_at: now()?,
            };
            let mut stage = tempfile::NamedTempFile::new_in(&folder)?;
            stage.write_all(&serde_json::to_vec(&r).map_err(|_| invalid())?)?;
            stage.as_file().sync_all()?;
            stage
                .persist_noclobber(&receipt_path)
                .map_err(|_| invalid())?;
            r
        };
        if receipt.digest != digest {
            return Err(StorageError::Invalid(
                "同一请求ID内容不同，已拒绝重复写入。请重新预览后保存为新剪藏。",
            ));
        }
        let path = folder.join(filename(&clip.title, &request.request_id)?);
        let existing: Option<String> = self
            .connection
            .query_row(
                "SELECT path FROM markdown_documents WHERE id=?1",
                [&request.request_id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            let current = Path::new(&existing);
            files::ordinary(current)?;
            if current.canonicalize()?.parent() != Some(folder.as_path()) {
                return Err(invalid());
            }
            return Ok(ClipSaved {
                document_id: request.request_id.clone(),
                title: clip.title.clone(),
                duplicate: true,
            });
        }
        // Check project before creating document assets; repeat under the transaction below.
        if let Some(p) = &clip.project_id {
            self.connection
                .query_row("SELECT id FROM projects WHERE id=?1", [p], |r| {
                    r.get::<_, String>(0)
                })
                .optional()?
                .ok_or(StorageError::NotFound)?;
        }
        let (markdown, pngs, assets) = markdown(clip, receipt.created_at)?;
        files::existing_ordinary(&path)?;
        if path.try_exists()? {
            // Crash recovery: never overwrite an already committed or externally modified file.
            if read(&path, 2 * 1024 * 1024)? != markdown.as_bytes() {
                return Err(invalid());
            }
            for (asset, png) in assets.iter().zip(&pngs) {
                files::ordinary(&folder.join("assets"))?;
                if read(&folder.join("assets").join(&asset.name), 256 * 1024)? != *png {
                    return Err(invalid());
                }
            }
        } else {
            DocumentFiles::default()
                .save(
                    Some(&path),
                    &SaveInput {
                        markdown,
                        assets,
                        token: None,
                        save_as: true,
                        file_name: None,
                    },
                    &pngs.iter().map(Vec::as_slice).collect::<Vec<_>>(),
                )
                .map_err(|_| invalid())?;
        }
        let canonical = path.canonicalize()?;
        let canonical = canonical.to_str().ok_or_else(invalid)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO markdown_documents(id,path,name,project_id,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?5)",params![request.request_id,canonical,path.file_name().and_then(|s|s.to_str()).ok_or_else(invalid)?,clip.project_id,receipt.created_at])?;
        tx.commit()?;
        Ok(ClipSaved {
            document_id: request.request_id.clone(),
            title: clip.title.clone(),
            duplicate: false,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> Request {
        Request {
            version: 1,
            request_id: "a".repeat(32),
            operation: "save".into(),
            clip: Some(Clip {
                title: "中文 / Control: notes".into(),
                url: "https://example.org/paper?q=1".into(),
                site_name: "Example".into(),
                byline: "Researcher".into(),
                mode: "article".into(),
                markdown:
                    "## 方法\n\n正文 **English**\n\n```rust\nlet x = 1;\n```\n\n> 引用\n\n- 项目"
                        .into(),
                project_id: None,
                images: vec![],
            }),
        }
    }
    #[test]
    fn save_retry_reopen_and_project_count() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().into()).unwrap();
        s.save_project(None, "研究").unwrap();
        let p = s.projects().unwrap().remove(0);
        let mut r = request();
        r.clip.as_mut().unwrap().project_id = Some(p.id.clone());
        let first = s.save_web_clip(&r).unwrap();
        assert!(!first.duplicate);
        let path = s.indexed_document_path(&first.document_id).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains("```rust\nlet x = 1;"));
        assert_eq!(s.projects().unwrap()[0].document_count, 1);
        assert!(s.save_web_clip(&r).unwrap().duplicate);
        drop(s);
        let mut s = Storage::open(t.path().into()).unwrap();
        assert!(s.save_web_clip(&r).unwrap().duplicate);
        assert_eq!(fs::read(path).unwrap(), bytes);
        r.clip.as_mut().unwrap().title = "changed".into();
        assert!(s.save_web_clip(&r).is_err());
    }
    #[test]
    fn rollback_recovery_and_no_external_overwrite() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().into()).unwrap();
        s.connection.execute_batch("CREATE TRIGGER reject_clip BEFORE INSERT ON markdown_documents BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        let r = request();
        assert!(s.save_web_clip(&r).is_err());
        s.connection
            .execute_batch("DROP TRIGGER reject_clip")
            .unwrap();
        assert!(!s.save_web_clip(&r).unwrap().duplicate);
        assert_eq!(s.project_documents(None, false).unwrap().len(), 1);
    }
    #[test]
    fn image_reference_and_untrusted_paths() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().into()).unwrap();
        let mut r = request();
        r.request_id = "../bad".into();
        assert!(s.save_web_clip(&r).is_err());
        r.request_id = "b".repeat(32);
        r.clip.as_mut().unwrap().markdown = "![](clip-image://missing)".into();
        assert!(s.save_web_clip(&r).is_err());
        assert!(s.project_documents(None, false).unwrap().is_empty());
    }
}
