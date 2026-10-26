//! Read-only, bounded loopback API. Zotero responses are data, never instructions.
use crate::services::references::{ReferenceInput, normalize_doi, validate};
use reqwest::{Url, blocking::Client, redirect::Policy};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
pub type Result<T> = std::result::Result<T, String>;
const BODY_LIMIT: usize = 2 * 1024 * 1024;
#[path = "zotero_write_transport.rs"]
mod write_transport;
pub use write_transport::{LocalWriteGrant, WriteIntent, ZoteroWriteError, ZoteroWriteOutcome};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ZoteroScope {
    pub library_type: String,
    pub library_id: u64,
}
impl ZoteroScope {
    pub fn validate_request(&self) -> Result<()> {
        if !matches!(self.library_type.as_str(), "user" | "group")
            || self.library_id > 9_007_199_254_740_991
            || (self.library_type == "user" && self.library_id != 0)
            || (self.library_type == "group" && self.library_id == 0)
        {
            return Err("请选择个人库，或提供有效群组库ID。".into());
        }
        Ok(())
    }
    fn prefix(&self) -> String {
        if self.library_type == "user" {
            "users/0".into()
        } else {
            format!("groups/{}", self.library_id)
        }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoteroHealth {
    pub api_version: u8,
    pub schema_version: Option<u32>,
    pub instance_id: String,
    pub product_version: Option<String>,
    #[serde(skip)]
    pub server_id: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoteroCandidate {
    pub instance_id: String,
    pub library_type: String,
    pub library_id: u64,
    pub item_key: String,
    pub attachment_key: String,
    pub reference: ReferenceInput,
    pub warnings: Vec<String>,
    #[serde(skip)]
    pub server_id: String,
    #[serde(skip)]
    pub item_version: u64,
    #[serde(skip)]
    pub attachment_version: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoteroLookup {
    pub health: ZoteroHealth,
    pub candidates: Vec<ZoteroCandidate>,
    pub message: String,
}
pub fn valid_item_key(key: &str) -> bool {
    key.len() == 8
        && key
            .bytes()
            .all(|v| v.is_ascii_uppercase() || v.is_ascii_digit())
}
pub fn instance_identity(server: &str) -> Result<String> {
    if !(8..=128).contains(&server.len())
        || !server
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || v == b'_' || v == b'-')
    {
        return Err("Zotero实例标识无效，未建立来源关联。".into());
    }
    let hash = Sha256::digest(format!("researchclip:zotero-local:v1:{server}").as_bytes());
    Ok(hash[..16].iter().map(|v| format!("{v:02x}")).collect())
}
pub fn local_api_base() -> String {
    #[cfg(debug_assertions)]
    if std::env::var_os("RESEARCHCLIP_TEST_SETTINGS_DIR").is_some()
        && let Ok(value) = std::env::var("RESEARCHCLIP_TEST_ZOTERO_PORT")
        && let Ok(port) = value.parse::<u16>()
        && port != 0
    {
        return format!("http://127.0.0.1:{port}/api/");
    }
    "http://127.0.0.1:23119/api/".into()
}
pub struct ZoteroClient {
    client: Client,
    base: Url,
}
impl ZoteroClient {
    pub fn new() -> Result<Self> {
        Self::at(&local_api_base())
    }
    fn at(base: &str) -> Result<Self> {
        let url = Url::parse(base).map_err(|_| "Zotero API地址无效。")?;
        if url.scheme() != "http"
            || url.host_str() != Some("127.0.0.1")
            || url.path() != "/api/"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
        {
            return Err("Zotero API只允许固定本机接口。".into());
        }
        let client = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_millis(500))
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|_| "Zotero只读客户端初始化失败。")?;
        Ok(Self { client, base: url })
    }
    fn check(cancel: &AtomicBool, start: Instant) -> Result<()> {
        if cancel.load(Ordering::Relaxed) {
            Err("Zotero查询已取消。".into())
        } else if start.elapsed() > Duration::from_secs(12) {
            Err("Zotero查询达到12秒时限，请缩小范围重试；PDF仍可阅读。".into())
        } else {
            Ok(())
        }
    }
    fn get(
        &self,
        path: &str,
        query: &[(&str, &str)],
        server: Option<&str>,
        cancel: &AtomicBool,
        start: Instant,
    ) -> Result<Option<(reqwest::header::HeaderMap, Vec<u8>)>> {
        Self::check(cancel, start)?;
        let mut url = self.base.join(path).map_err(|_| "Zotero接口路径无效。")?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        if url.origin() != self.base.origin() || !url.path().starts_with("/api/") {
            return Err("Zotero请求超出本机范围。".into());
        }
        let mut request = self
            .client
            .get(url)
            .header("Zotero-API-Version", "3")
            .header("User-Agent", "ResearchClip-Local-ReadOnly");
        if let Some(server) = server {
            request = request.header("Zotero-Server-ID", server);
        }
        let remaining = Duration::from_secs(12).saturating_sub(start.elapsed());
        let response = request
            .timeout(remaining.min(Duration::from_secs(2)))
            .send()
            .map_err(|_| {
                if cancel.load(Ordering::Relaxed) {
                    "Zotero查询已取消。"
                } else {
                    "无法连接Zotero Local API。可继续阅读PDF；请检查Zotero是否启动及API设置。"
                }
            })?;
        Self::check(cancel, start)?;
        match response.status().as_u16() {
            404 => return Ok(None),
            403 => {
                return Err("Zotero拒绝读取；请在设置→高级允许本机应用通信。PDF仍可阅读。".into());
            }
            412 => return Err("Zotero数据实例已经切换，请重新查询来源。".into()),
            200 => {}
            _ => return Err("Zotero接口返回不支持的状态；PDF仍可阅读。".into()),
        }
        let headers = response.headers().clone();
        if headers
            .get("Zotero-API-Version")
            .and_then(|v| v.to_str().ok())
            != Some("3")
        {
            return Err("当前Zotero API版本不受支持；保留普通PDF阅读。".into());
        }
        let current = headers
            .get("Zotero-Server-ID")
            .and_then(|v| v.to_str().ok())
            .ok_or("此Zotero接口没有实例标识，本阶段不支持持久来源关联；PDF仍可阅读。")?;
        instance_identity(current)?;
        if server.is_some_and(|expected| expected != current) {
            return Err("Zotero数据实例已经切换，来源查询未保存。".into());
        }
        if response
            .content_length()
            .is_some_and(|len| len > BODY_LIMIT as u64)
        {
            return Err("Zotero响应超过2MiB保护预算。".into());
        }
        let mut bytes = Vec::new();
        response
            .take((BODY_LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "Zotero响应读取失败。")?;
        if bytes.len() > BODY_LIMIT {
            return Err("Zotero响应超过2MiB保护预算。".into());
        }
        Self::check(cancel, start)?;
        Ok(Some((headers, bytes)))
    }
    pub fn health(&self, cancel: &AtomicBool, start: Instant) -> Result<ZoteroHealth> {
        let (headers, _) = self
            .get("", &[], None, cancel, start)?
            .ok_or("Zotero Local API不可用。")?;
        let server_id = headers
            .get("Zotero-Server-ID")
            .and_then(|v| v.to_str().ok())
            .ok_or("Zotero未提供实例标识。")?
            .to_owned();
        Ok(ZoteroHealth {
            api_version: 3,
            schema_version: headers
                .get("Zotero-Schema-Version")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok()),
            instance_id: instance_identity(&server_id)?,
            product_version: headers
                .get("Zotero-Version")
                .or_else(|| headers.get("X-Zotero-Version"))
                .and_then(|v| v.to_str().ok())
                .filter(|v| v.len() <= 64)
                .map(str::to_owned),
            server_id,
        })
    }
    fn item(
        &self,
        scope: &ZoteroScope,
        key: &str,
        health: &ZoteroHealth,
        cancel: &AtomicBool,
        start: Instant,
    ) -> Result<Option<Value>> {
        if !valid_item_key(key) {
            return Err("Zotero条目键无效。".into());
        }
        let path = format!("{}/items/{key}", scope.prefix());
        self.get(&path, &[], Some(&health.server_id), cancel, start)?
            .map(|(_, b)| serde_json::from_slice(&b).map_err(|_| "Zotero条目JSON无效。".into()))
            .transpose()
    }
    fn file_matches(
        &self,
        scope: &ZoteroScope,
        key: &str,
        health: &ZoteroHealth,
        source: &Path,
        cancel: &AtomicBool,
        start: Instant,
    ) -> Result<bool> {
        let path = format!("{}/items/{key}/file/view/url", scope.prefix());
        let Some((_, bytes)) = self.get(&path, &[], Some(&health.server_id), cancel, start)? else {
            return Ok(false);
        };
        if bytes.len() > 32768 {
            return Err("Zotero文件位置超过保护预算。".into());
        }
        let uri = std::str::from_utf8(&bytes).map_err(|_| "Zotero文件URL不是UTF-8。")?;
        Ok(same_authorized_file_url(source, uri.trim()))
    }
    pub fn lookup(
        &self,
        source: &Path,
        scope: &ZoteroScope,
        query: &str,
        cancel: &AtomicBool,
    ) -> Result<ZoteroLookup> {
        scope.validate_request()?;
        if query.chars().count() > 200 || query.contains('\0') {
            return Err("Zotero检索词过长或无效。".into());
        }
        let start = Instant::now();
        let health = self.health(cancel, start)?;
        let mut items = Vec::new();
        let mut keys = std::collections::HashSet::new();
        if let Some(hint) = source
            .parent()
            .and_then(Path::file_name)
            .and_then(|s| s.to_str())
            .filter(|s| valid_item_key(s))
            && let Some(item) = self.item(scope, hint, &health, cancel, start)?
        {
            keys.insert(hint.to_owned());
            items.push(item);
        }
        let default_query = source
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .chars()
            .take(160)
            .collect::<String>();
        let term = if query.trim().is_empty() {
            default_query.trim()
        } else {
            query.trim()
        };
        let mut partial = false;
        if !term.is_empty() {
            let path = format!("{}/items", scope.prefix());
            if let Some((_, bytes)) = self.get(
                &path,
                &[
                    ("itemType", "attachment"),
                    ("q", term),
                    ("limit", "50"),
                    ("format", "json"),
                ],
                Some(&health.server_id),
                cancel,
                start,
            )? {
                let values: Vec<Value> =
                    serde_json::from_slice(&bytes).map_err(|_| "Zotero附件列表JSON无效。")?;
                if values.len() > 50 {
                    return Err("Zotero附件列表超出请求范围。".into());
                }
                partial = values.len() == 50;
                for item in values {
                    if let Some(key) = item.get("key").and_then(Value::as_str)
                        && valid_item_key(key)
                        && keys.insert(key.into())
                    {
                        items.push(item);
                    }
                }
            }
        }
        let mut candidates = Vec::new();
        let mut standalone = false;
        for attachment in items {
            Self::check(cancel, start)?;
            let Some(key) = attachment.get("key").and_then(Value::as_str) else {
                continue;
            };
            let Some(data) = attachment.get("data") else {
                continue;
            };
            if data.get("itemType").and_then(Value::as_str) != Some("attachment")
                || data.get("contentType").and_then(Value::as_str) != Some("application/pdf")
            {
                continue;
            }
            validate_library(&attachment, scope)?;
            if !self.file_matches(scope, key, &health, source, cancel, start)? {
                continue;
            }
            let Some(parent) = data
                .get("parentItem")
                .and_then(Value::as_str)
                .filter(|s| valid_item_key(s))
            else {
                standalone = true;
                continue;
            };
            let Some(item) = self.item(scope, parent, &health, cancel, start)? else {
                continue;
            };
            let candidate = metadata_candidate(&health, scope, &attachment, &item)?;
            candidates.push(candidate);
            if candidates.len() >= 8 {
                partial = true;
                break;
            }
        }
        let message = if partial {
            "候选达到保护上限，结果可能不完整；请缩小检索范围。"
        } else if candidates.is_empty() && standalone {
            "匹配到独立附件，但没有父文献；本阶段继续作为普通PDF阅读。"
        } else if candidates.is_empty() {
            "未找到能够核对原始文件的文献来源；可继续阅读PDF。群组资料请选择对应群组库。"
        } else {
            "已核对附件文件位置；元数据仍待人工核对，请确认后关联。"
        }
        .into();
        Ok(ZoteroLookup {
            health,
            candidates,
            message,
        })
    }

    fn annotation_context(
        &self,
        source: &super::zotero_annotations::AnnotationSource,
        original: &Path,
        cancel: &AtomicBool,
        start: Instant,
    ) -> Result<(ZoteroHealth, ZoteroScope)> {
        use super::zotero_annotations::SOURCE_MISMATCH;
        source.validate()?;
        let health = self.health(cancel, start)?;
        if health.instance_id != source.instance_id {
            return Err(SOURCE_MISMATCH.into());
        }
        let scope = ZoteroScope {
            library_type: source.library_type.clone(),
            library_id: if source.library_type == "user" {
                0
            } else {
                source.library_id
            },
        };
        scope.validate_request()?;
        let attachment = self
            .item(&scope, &source.attachment_key, &health, cancel, start)?
            .ok_or("已关联的Zotero附件不存在。")?;
        let (kind, id) = validate_library(&attachment, &scope).map_err(|_| SOURCE_MISMATCH)?;
        let data = &attachment["data"];
        if kind != source.library_type
            || id != source.library_id
            || attachment["key"] != source.attachment_key
            || data["itemType"] != "attachment"
            || data["parentItem"] != source.item_key
            || !self.file_matches(
                &scope,
                &source.attachment_key,
                &health,
                original,
                cancel,
                start,
            )?
        {
            return Err(SOURCE_MISMATCH.into());
        }
        Ok((health, scope))
    }
    pub fn read_annotations(
        &self,
        source: &super::zotero_annotations::AnnotationSource,
        original: &Path,
        pdf_hash: &str,
        cancel: &AtomicBool,
    ) -> Result<super::zotero_annotations::AnnotationReadCache> {
        use super::zotero_annotations::{AnnotationReadCache, parse_annotation};
        let start = Instant::now();
        let (health, scope) = self.annotation_context(source, original, cancel, start)?;
        let mut annotations: std::collections::BTreeMap<
            String,
            super::zotero_annotations::ZoteroAnnotation,
        > = std::collections::BTreeMap::new();
        let mut library_version = None;
        let mut truncated = false;
        let mut bytes = 0;
        for page in 0..50 {
            Self::check(cancel, start)?;
            let offset = (page * 20).to_string();
            let path = format!(
                "{}/items/{}/children",
                scope.prefix(),
                source.attachment_key
            );
            let (headers, body) = self
                .get(
                    &path,
                    &[
                        ("itemType", "annotation"),
                        ("format", "json"),
                        ("limit", "20"),
                        ("start", &offset),
                    ],
                    Some(&health.server_id),
                    cancel,
                    start,
                )?
                .ok_or("Zotero未提供附件批注读取接口或附件已经删除。")?;
            bytes += body.len();
            if bytes > 8 * 1024 * 1024 {
                return Err("批注读取超过8MiB预算，原缓存保留。".into());
            }
            let version = headers
                .get("Last-Modified-Version")
                .and_then(|v| v.to_str().ok())
                .map(str::parse::<u64>)
                .transpose()
                .map_err(|_| "Zotero库版本头无效。")?;
            if page == 0 {
                library_version = version;
            } else if library_version.is_none() || version != library_version {
                return Err(
                    "批注读取期间库版本改变或缺少分页版本校验，请重试；旧缓存保留。".into(),
                );
            }
            let batch: Vec<Value> =
                serde_json::from_slice(&body).map_err(|_| "Zotero批注列表JSON无效。")?;
            let count = batch.len();
            if count > 20 {
                return Err("Zotero未遵守批注分页上限。".into());
            }
            for item in batch {
                let a = parse_annotation(&item, source)?;
                let key = a.identity.annotation_key.clone();
                if let Some(old) = annotations.get(&key) {
                    if old.version == a.version && old != &a {
                        return Err("同一Zotero批注版本返回冲突数据，旧缓存保留。".into());
                    }
                    if old.version >= a.version {
                        continue;
                    }
                }
                annotations.insert(key, a);
            }
            if count < 20 {
                break;
            }
            if page == 49 {
                truncated = true;
            }
        }
        Self::check(cancel, start)?;
        let fetched_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "系统时间无效。")?
            .as_millis()
            .try_into()
            .map_err(|_| "系统时间超限。")?;
        let mut items: Vec<_> = annotations.into_values().collect();
        items.sort_by_key(|a| {
            (
                a.position.as_ref().map_or(u32::MAX, |p| p.page_index),
                a.identity.annotation_key.clone(),
            )
        });
        let result = AnnotationReadCache {
            format_version: 1,
            source: source.clone(),
            pdf_content_hash: pdf_hash.into(),
            fetched_at,
            product_version: health.product_version,
            library_version,
            annotations: items,
            truncated,
        };
        result.validate()?;
        Ok(result)
    }
    pub fn revalidate(
        &self,
        candidate: &ZoteroCandidate,
        source: &Path,
        cancel: &AtomicBool,
    ) -> Result<()> {
        let start = Instant::now();
        let health = self.health(cancel, start)?;
        if health.server_id != candidate.server_id {
            return Err("Zotero实例已切换，请重新预览。".into());
        }
        let scope = ZoteroScope {
            library_type: candidate.library_type.clone(),
            library_id: if candidate.library_type == "user" {
                0
            } else {
                candidate.library_id
            },
        };
        let attachment = self
            .item(&scope, &candidate.attachment_key, &health, cancel, start)?
            .ok_or("Zotero附件已经缺失。")?;
        let parent = self
            .item(&scope, &candidate.item_key, &health, cancel, start)?
            .ok_or("Zotero文献已经缺失。")?;
        let latest = metadata_candidate(&health, &scope, &attachment, &parent)?;
        if latest.item_version != candidate.item_version
            || latest.attachment_version != candidate.attachment_version
            || latest.library_id != candidate.library_id
            || !self.file_matches(
                &scope,
                &candidate.attachment_key,
                &health,
                source,
                cancel,
                start,
            )?
        {
            return Err("Zotero来源在预览后改变，请重新查询，原关联未覆盖。".into());
        }
        Ok(())
    }
}
fn validate_library(item: &Value, requested: &ZoteroScope) -> Result<(String, u64)> {
    let library = item.get("library").ok_or("Zotero条目缺少库范围。")?;
    let kind = library
        .get("type")
        .and_then(Value::as_str)
        .ok_or("Zotero库类型缺失。")?;
    let id = library
        .get("id")
        .and_then(Value::as_u64)
        .ok_or("Zotero库ID无效。")?;
    if kind != requested.library_type
        || id > 9_007_199_254_740_991
        || (kind == "group" && id != requested.library_id)
    {
        return Err("Zotero响应库范围与请求不匹配。".into());
    }
    Ok((kind.into(), id))
}
fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}
pub fn metadata_candidate(
    health: &ZoteroHealth,
    scope: &ZoteroScope,
    attachment: &Value,
    parent: &Value,
) -> Result<ZoteroCandidate> {
    let (kind, library) = validate_library(attachment, scope)?;
    if validate_library(parent, scope)? != (kind.clone(), library) {
        return Err("Zotero父文献与附件属于不同库。".into());
    }
    let a = attachment.get("data").ok_or("Zotero附件字段缺失。")?;
    let p = parent.get("data").ok_or("Zotero文献字段缺失。")?;
    let attachment_key = string(attachment, "key");
    let item_key = string(parent, "key");
    if !valid_item_key(attachment_key)
        || !valid_item_key(item_key)
        || string(a, "parentItem") != item_key
        || string(a, "key") != attachment_key
        || string(p, "key") != item_key
        || string(a, "itemType") != "attachment"
        || matches!(
            string(p, "itemType"),
            "attachment" | "note" | "annotation" | ""
        )
    {
        return Err("Zotero父文献/附件键或类型不匹配。".into());
    }
    let item_version = parent
        .get("version")
        .and_then(Value::as_u64)
        .ok_or("Zotero文献版本缺失。")?;
    let attachment_version = attachment
        .get("version")
        .and_then(Value::as_u64)
        .ok_or("Zotero附件版本缺失。")?;
    let mut authors: Vec<String> = Vec::new();
    if let Some(creators) = p.get("creators") {
        let creators = creators.as_array().ok_or("Zotero作者字段无效。")?;
        if creators.len() > 200 {
            return Err("Zotero作者超过200项保护上限。".into());
        }
        for c in creators {
            if string(c, "creatorType") != "author" {
                continue;
            }
            let name: String = if !string(c, "name").trim().is_empty() {
                string(c, "name").trim().into()
            } else {
                format!(
                    "{} {}",
                    string(c, "firstName").trim(),
                    string(c, "lastName").trim()
                )
                .trim()
                .into()
            };
            if !name.is_empty() {
                authors.push(name);
            }
        }
    }
    let mut warnings = Vec::new();
    let doi = normalize_doi(string(p, "DOI")).unwrap_or_else(|_| {
        warnings.push("接口DOI格式无效，未生成可点击DOI；原字段保存在来源快照。".into());
        String::new()
    });
    let url = string(p, "url");
    let url = if url.is_empty() {
        String::new()
    } else if Url::parse(url).is_ok_and(|u| {
        matches!(u.scheme(), "http" | "https")
            && u.username().is_empty()
            && u.password().is_none()
            && url.len() <= 2048
    }) {
        url.into()
    } else {
        warnings.push("接口URL不受支持，仅保留原始字段。".into());
        String::new()
    };
    let snapshot = json!({"provider":"zoteroLocal","status":"unverified","formatVersion":1,"instanceId":health.instance_id,"libraryType":kind,"libraryId":library,"itemKey":item_key,"attachmentKey":attachment_key,"itemVersion":item_version,"attachmentVersion":attachment_version,"reported":{"title":p.get("title"),"creators":p.get("creators"),"date":p.get("date"),"DOI":p.get("DOI"),"url":p.get("url"),"publicationTitle":p.get("publicationTitle"),"itemType":p.get("itemType")}});
    if snapshot.to_string().len() > 65536 {
        return Err("Zotero来源快照超过64KiB保护预算。".into());
    }
    let reference = ReferenceInput {
        title: string(p, "title").trim().into(),
        authors,
        year: reported_year(string(p, "date")),
        journal_or_conference: [
            "publicationTitle",
            "proceedingsTitle",
            "bookTitle",
            "publisher",
        ]
        .into_iter()
        .map(|key| string(p, key))
        .find(|s| !s.is_empty())
        .unwrap_or("")
        .into(),
        doi,
        url,
        bibtex_key: format!(
            "Zotero_{}_{}_{}_{}",
            health.instance_id,
            if kind == "user" { "u" } else { "g" },
            library,
            item_key
        ),
        bibtex_entry: String::new(),
        verified_source: "manual".into(),
        source_metadata: snapshot,
    };
    validate(&reference)?;
    Ok(ZoteroCandidate {
        instance_id: health.instance_id.clone(),
        library_type: kind,
        library_id: library,
        item_key: item_key.into(),
        attachment_key: attachment_key.into(),
        reference,
        warnings,
        server_id: health.server_id.clone(),
        item_version,
        attachment_version,
    })
}
fn reported_year(date: &str) -> Option<i32> {
    date.split(|c: char| !c.is_ascii_digit())
        .find(|part| part.len() == 4)
        .and_then(|part| part.parse::<i32>().ok())
        .filter(|n| *n > 0)
}
pub fn same_authorized_file_url(source: &Path, uri: &str) -> bool {
    let Ok(url) = Url::parse(uri) else {
        return false;
    };
    if url.scheme() != "file"
        || url.host_str().is_some_and(|h| h != "localhost")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return false;
    }
    let Ok(from_api) = url.to_file_path() else {
        return false;
    };
    let Ok(authorized) = source.canonicalize() else {
        return false;
    };
    // Never open/canonicalize a path supplied by the untrusted API. Compare to the authorized path.
    let normalize = |path: &Path| {
        let value = path.to_string_lossy().replace('/', "\\");
        value.strip_prefix("\\\\?\\").unwrap_or(&value).to_owned()
    };
    normalize(&from_api).eq_ignore_ascii_case(&normalize(&authorized))
}
#[cfg(test)]
#[path = "zotero_annotations_http_tests.rs"]
mod annotation_tests;

#[cfg(test)]
mod tests {
    use super::*;
    fn values() -> (ZoteroHealth, ZoteroScope, Value, Value) {
        let h = ZoteroHealth {
            api_version: 3,
            schema_version: Some(44),
            instance_id: instance_identity("FixtureServer1").unwrap(),
            server_id: "FixtureServer1".into(),
            product_version: None,
        };
        let scope = ZoteroScope {
            library_type: "user".into(),
            library_id: 0,
        };
        let a = json!({"key":"ATTACH01","version":3,"library":{"type":"user","id":0},"data":{"key":"ATTACH01","itemType":"attachment","contentType":"application/pdf","parentItem":"PARENT01"}});
        let p = json!({"key":"PARENT01","version":2,"library":{"type":"user","id":0},"data":{"key":"PARENT01","itemType":"journalArticle","title":"练习文献（未核实）","creators":[{"creatorType":"author","firstName":"A","lastName":"Researcher"},{"creatorType":"editor","name":"Not an author"}],"date":"Spring 2024","DOI":"10.1234/TEST","url":"https://example.invalid/paper"}});
        (h, scope, a, p)
    }
    #[test]
    fn scoped_metadata_preserves_unknowns_and_unverified_provenance() {
        let (h, scope, a, mut p) = values();
        let c = metadata_candidate(&h, &scope, &a, &p).unwrap();
        assert_eq!(c.reference.authors, ["A Researcher"]);
        assert_eq!(c.reference.year, Some(2024));
        assert_eq!(c.reference.doi, "10.1234/test");
        assert_eq!(c.library_id, 0);
        assert_eq!(c.reference.source_metadata["status"], "unverified");
        p["data"]["DOI"] = json!("not a DOI");
        p["data"]["date"] = json!("forthcoming");
        p["data"]["creators"] = json!([]);
        let c = metadata_candidate(&h, &scope, &a, &p).unwrap();
        assert_eq!(c.reference.year, None);
        assert!(c.reference.doi.is_empty() && c.reference.authors.is_empty());
        assert_eq!(c.warnings.len(), 1);
        assert_eq!(c.reference.source_metadata["reported"]["DOI"], "not a DOI");
    }
    #[test]
    fn cross_library_and_wrong_keys_are_rejected() {
        let (h, scope, a, mut p) = values();
        p["library"]["id"] = json!(123);
        assert!(metadata_candidate(&h, &scope, &a, &p).is_err());
        p["library"]["id"] = json!(0);
        p["key"] = json!("WRONG001");
        assert!(metadata_candidate(&h, &scope, &a, &p).is_err());
        assert!(
            ZoteroScope {
                library_type: "group".into(),
                library_id: 0
            }
            .validate_request()
            .is_err()
        );
        assert!(instance_identity("bad\nheader").is_err());
        assert_ne!(
            instance_identity("ServerOne").unwrap(),
            instance_identity("ServerTwo").unwrap()
        );
    }
    #[test]
    fn only_authorized_file_urls_match_without_opening_other_files() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("中文 & 空格.pdf");
        std::fs::write(&source, b"%PDF").unwrap();
        let uri = Url::from_file_path(&source).unwrap();
        assert!(same_authorized_file_url(&source, uri.as_str()));
        let wrong = dir.path().join("other.pdf");
        assert!(!same_authorized_file_url(
            &source,
            Url::from_file_path(wrong).unwrap().as_str()
        ));
        assert!(!same_authorized_file_url(
            &source,
            "file://evil.invalid/share/file.pdf"
        ));
        assert!(!same_authorized_file_url(
            &source,
            "https://example.invalid/file.pdf"
        ));
        assert!(!same_authorized_file_url(
            &source,
            &format!("{}#page=1", uri)
        ));
    }
    #[test]
    fn read_only_http_rejects_redirect_disabled_api_missing_identity_and_oversized_body() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            thread,
        };
        for (status, headers, expected) in [
            ("403 Forbidden", "", "拒绝"),
            (
                "302 Found",
                "Location: http://127.0.0.1:1/other\r\n",
                "不支持",
            ),
            ("200 OK", "Zotero-API-Version: 3\r\n", "实例标识"),
            (
                "200 OK",
                "Zotero-API-Version: 2\r\nZotero-Server-ID: FixtureServer1\r\n",
                "版本",
            ),
            (
                "200 OK",
                "Zotero-API-Version: 3\r\nZotero-Server-ID: FixtureServer1\r\nContent-Length: 3000000\r\n",
                "2MiB",
            ),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let base = format!("http://{}/api/", listener.local_addr().unwrap());
            let handle = thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut b = [0; 4096];
                let n = stream.read(&mut b).unwrap();
                let request = String::from_utf8_lossy(&b[..n]);
                assert!(request.starts_with("GET /api/"));
                assert!(!request.contains("Authorization:"));
                let body = format!("HTTP/1.1 {status}\r\n{headers}Connection: close\r\n\r\n");
                stream.write_all(body.as_bytes()).unwrap();
            });
            let error = ZoteroClient::at(&base)
                .unwrap()
                .health(&AtomicBool::new(false), Instant::now())
                .unwrap_err();
            assert!(error.contains(expected), "{error}");
            handle.join().unwrap();
        }
        assert!(ZoteroClient::at("https://example.invalid/api/").is_err());
        assert!(ZoteroClient::check(&AtomicBool::new(true), Instant::now()).is_err());
    }
}
