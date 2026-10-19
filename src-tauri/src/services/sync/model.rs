use crate::storage::{StorageError, files::id_valid};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub type Result<T> = std::result::Result<T, StorageError>;
pub const MAX_BLOB: usize = 64 * 1024 * 1024;
pub const MAX_RESOURCE: usize = 24 * 1024 * 1024;
pub const MAX_CATALOG: usize = 2 * 1024 * 1024;
pub const MAX_RESOURCES: usize = 5000;
pub fn invalid(message: &'static str) -> StorageError {
    StorageError::Invalid(message)
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn hash_valid(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn key_valid(s: &str) -> bool {
    s.split_once('/').is_some_and(|(kind, id)| {
        ["screenshot", "project", "document"].contains(&kind) && id_valid(id)
    })
}
pub fn json<T: Serialize>(v: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(v).map_err(|_| invalid("同步数据不能序列化。"))
}
pub fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8], limit: usize) -> Result<T> {
    if bytes.len() > limit {
        return Err(invalid("同步资源超过大小上限。"));
    }
    serde_json::from_slice(bytes).map_err(|_| invalid("同步数据格式或版本无效；原资料未改动。"))
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub server: String,
    pub folder: String,
    pub projects: Vec<String>,
    pub unclassified: bool,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        let url = reqwest::Url::parse(&self.server).map_err(|_| invalid("服务器地址无效。"))?;
        if url.scheme() != "https"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.host_str().is_none()
            || self.server.len() > 2048
        {
            return Err(invalid("仅接受不含凭据、查询和片段的HTTPS服务器地址。"));
        }
        if !self.server.ends_with('/') || !folder_valid(&self.folder) {
            return Err(invalid(
                "服务器地址须以/结束；专属目录仅允许字母、数字、连字符，不能使用已有应用目录。",
            ));
        }
        if self.projects.len() > 500
            || self.projects.iter().any(|p| !id_valid(p))
            || self.projects.iter().collect::<BTreeSet<_>>().len() != self.projects.len()
        {
            return Err(invalid("同步项目选择无效。"));
        }
        Ok(())
    }
    pub fn id(&self) -> String {
        hash(format!("{}{}", self.server, self.folder).as_bytes())[..32].into()
    }
    pub fn selected(&self, project: Option<&str>) -> bool {
        project.map_or(self.unclassified, |p| {
            self.projects.iter().any(|id| id == p)
        })
    }
}
pub fn folder_valid(s: &str) -> bool {
    s.starts_with("researchclip-")
        && (14..=80).contains(&s.len())
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Blob {
    pub hash: String,
    pub size: usize,
}
impl Blob {
    pub fn validate(&self) -> Result<()> {
        if !hash_valid(&self.hash) || self.size > MAX_BLOB {
            Err(invalid("同步图片或文件引用无效。"))
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created: i64,
    pub updated: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tag {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub id: String,
    pub content: String,
    pub source: Option<String>,
    pub created: i64,
    pub updated: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub id: String,
    pub parent: Option<String>,
    pub annotation: Value,
    pub created: i64,
    pub description: String,
    pub schema: u32,
}
pub fn history_order(history: &[Revision]) -> Result<Vec<String>> {
    let parents: BTreeMap<&str, Option<&str>> = history
        .iter()
        .map(|r| (r.id.as_str(), r.parent.as_deref()))
        .collect();
    if parents.len() != history.len() {
        return Err(invalid("标注历史包含重复ID。"));
    }
    let mut pending: BTreeSet<&str> = parents.keys().copied().collect();
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    while !pending.is_empty() {
        let ready: Vec<&str> = pending
            .iter()
            .copied()
            .filter(|id| parents[id].is_none_or(|p| seen.contains(p)))
            .collect();
        if ready.is_empty() {
            return Err(invalid("标注历史缺少父版本或存在循环。"));
        }
        for id in ready {
            pending.remove(id);
            seen.insert(id);
            result.push(id.into());
        }
    }
    Ok(result)
}
pub fn sort_history(history: &mut [Revision]) -> Result<()> {
    let order: BTreeMap<String, usize> = history_order(history)?
        .into_iter()
        .enumerate()
        .map(|(n, id)| (id, n))
        .collect();
    history.sort_by_key(|r| order[&r.id]);
    Ok(())
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ocr {
    pub text: String,
    pub language: String,
    pub engine: String,
    pub updated: i64,
    pub corrected: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Screenshot {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub created: i64,
    pub updated: i64,
    pub favorite: bool,
    pub original: Blob,
    pub annotation: Value,
    pub head: String,
    pub history: Vec<Revision>,
    pub notes: Vec<Note>,
    pub tags: Vec<Tag>,
    pub project: Option<Project>,
    pub ocr: Option<Ocr>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub name: String,
    pub markdown: Blob,
    pub assets: BTreeMap<String, Blob>,
    pub project: Option<Project>,
    pub created: i64,
    pub updated: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum Body {
    Screenshot(Box<Screenshot>),
    Project(Project),
    Document(Document),
    Deleted,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub key: String,
    pub project: Option<String>,
    pub body: Body,
}
fn text(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.chars().count() <= max && !s.chars().any(char::is_control)
}
fn project(p: &Project) -> Result<()> {
    if !id_valid(&p.id) || !text(&p.name, 80) {
        return Err(invalid("同步项目元数据无效。"));
    }
    Ok(())
}
impl Snapshot {
    pub fn content(&self) -> Result<String> {
        Ok(hash(&json(self)?))
    }
    pub fn blobs(&self) -> Vec<Blob> {
        match &self.body {
            Body::Screenshot(s) => vec![s.original.clone()],
            Body::Document(d) => std::iter::once(d.markdown.clone())
                .chain(d.assets.values().cloned())
                .collect(),
            _ => vec![],
        }
    }
    pub fn validate(&self) -> Result<()> {
        if !key_valid(&self.key) || self.project.as_ref().is_some_and(|p| !id_valid(p)) {
            return Err(invalid("同步资源标识或项目范围无效。"));
        }
        match &self.body {
            Body::Screenshot(s) => {
                if !self.key.starts_with("screenshot/")
                    || !text(&s.title, 200)
                    || s.width == 0
                    || s.height == 0
                    || u64::from(s.width) * u64::from(s.height) > 16777216
                    || s.history.len() > 1024
                    || s.notes.len() > 128
                    || s.tags.len() > 128
                    || s.history.is_empty()
                {
                    return Err(invalid("同步截图超过限制或元数据无效。"));
                }
                crate::services::editor::validate_document(&s.annotation)
                    .map_err(|_| invalid("当前标注格式不支持；同步已停止。"))?;
                if s.annotation["image"]["widthPx"] != s.width
                    || s.annotation["image"]["heightPx"] != s.height
                {
                    return Err(invalid("同步标注与原图尺寸不符。"));
                }
                if history_order(&s.history)?
                    != s.history.iter().map(|r| r.id.clone()).collect::<Vec<_>>()
                {
                    return Err(invalid("同步历史必须采用稳定的父先序与ID排序。"));
                }
                let mut ids = BTreeSet::new();
                for r in &s.history {
                    if !id_valid(&r.id)
                        || !ids.insert(&r.id)
                        || r.parent.as_ref().is_some_and(|p| !ids.contains(p))
                        || r.schema != 1
                        || r.description.chars().count() > 500
                        || r.description.contains('\0')
                    {
                        return Err(invalid("标注历史父关系、版本或标识无效；没有截断历史。"));
                    }
                    crate::services::editor::validate_document(&r.annotation)
                        .map_err(|_| invalid("历史标注不支持或损坏。"))?;
                    if r.annotation["image"]["widthPx"] != s.width
                        || r.annotation["image"]["heightPx"] != s.height
                    {
                        return Err(invalid("历史标注尺寸不一致。"));
                    }
                }
                if !s.history.iter().any(|r| {
                    r.id == s.head
                        && crate::storage::revisions::same_content(&r.annotation, &s.annotation)
                }) {
                    return Err(invalid("标注当前版本映射无效，请在编辑器确认后再同步。"));
                }
                if s.notes.iter().any(|n| {
                    !id_valid(&n.id)
                        || n.content.len() > 2 * 1024 * 1024
                        || n.content.contains('\0')
                }) || s
                    .tags
                    .iter()
                    .any(|t| !id_valid(&t.id) || !text(&t.name, 50))
                {
                    return Err(invalid("同步笔记或标签无效。"));
                }
                for n in &s.notes {
                    crate::storage::knowledge::source_url(n.source.as_deref())?;
                }
                if let Some(p) = &s.project {
                    project(p)?;
                }
                if self.project != s.project.as_ref().map(|p| p.id.clone()) {
                    return Err(invalid("截图项目范围不一致。"));
                }
                if s.ocr.as_ref().is_some_and(|o| {
                    o.text.len() > 2 * 1024 * 1024 || o.language.len() > 64 || o.engine.len() > 256
                }) {
                    return Err(invalid("同步OCR文本超过上限。"));
                }
            }
            Body::Project(p) => {
                project(p)?;
                if self.key != format!("project/{}", p.id) || self.project.as_deref() != Some(&p.id)
                {
                    return Err(invalid("同步项目身份不一致。"));
                }
            }
            Body::Document(d) => {
                if !self.key.starts_with("document/")
                    || !d.name.to_ascii_lowercase().ends_with(".md")
                    || d.name.trim() != d.name
                    || crate::services::markdown_document::normalized_name(&d.name).is_err()
                    || d.markdown.size > 2 * 1024 * 1024
                    || d.assets.len() > 32
                    || d.assets
                        .keys()
                        .any(|n| !crate::services::markdown_document::asset_name(n))
                    || d.assets.values().map(|b| b.size as u64).sum::<u64>() > 128 * 1024 * 1024
                {
                    return Err(invalid("Markdown名称或受管资源无效。"));
                }
                if let Some(p) = &d.project {
                    project(p)?;
                }
                if self.project != d.project.as_ref().map(|p| p.id.clone()) {
                    return Err(invalid("文档项目范围不一致。"));
                }
            }
            Body::Deleted => {}
        }
        for b in self.blobs() {
            b.validate()?;
        }
        if json(self)?.len() > MAX_RESOURCE {
            return Err(invalid("单资源超过24MiB，未截断或上传。"));
        }
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub protocol: u32,
    pub workspace: String,
    pub device: String,
    pub counter: String,
    pub parents: Vec<String>,
    pub content_hash: String,
    pub snapshot: Snapshot,
}
impl Manifest {
    pub fn validate(&self, workspace: &str) -> Result<()> {
        if self.protocol != 2
            || self.workspace != workspace
            || !id_valid(&self.device)
            || self.counter.parse::<u64>().is_err()
            || self.parents.len() > 2
            || self.parents.iter().any(|p| !hash_valid(p))
            || self.snapshot.content()? != self.content_hash
        {
            return Err(invalid("同步协议版本、工作区或摘要不符。"));
        }
        self.snapshot.validate()
    }
}
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub protocol: u32,
    pub workspace: String,
    pub entries: BTreeMap<String, Head>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Head {
    pub manifest: String,
    pub project: Option<String>,
}
impl Catalog {
    pub fn validate(&self, workspace: &str) -> Result<()> {
        if self.protocol != 2
            || self.workspace != workspace
            || self.entries.len() > MAX_RESOURCES
            || self.entries.iter().any(|(k, v)| {
                !key_valid(k)
                    || !hash_valid(&v.manifest)
                    || v.project.as_ref().is_some_and(|p| !id_valid(p))
            })
        {
            Err(invalid("远端目录索引版本、标识或资源数量无效。"))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsafe_servers_paths_and_future_formats() {
        let mut c = Config {
            server: "https://dav.jianguoyun.com/dav/zotero/".into(),
            folder: "researchclip-sync-v2".into(),
            projects: vec![],
            unclassified: false,
        };
        assert!(c.validate().is_ok());
        for url in [
            "http://example.com/",
            "https://a:b@example.com/",
            "https://example.com/?key=secret",
        ] {
            c.server = url.into();
            assert!(c.validate().is_err());
        }
        assert!(!folder_valid("zotero"));
        assert!(!folder_valid("researchclip-../../x"));
        assert!(!key_valid("document/../../x"));
        assert!(!hash_valid(&"A".repeat(64)));
        assert!(
            parse::<Catalog>(
                br#"{"protocol":99,"workspace":"x","entries":{},"extra":1}"#,
                1000
            )
            .is_err()
        );
    }
}
