use super::model::*;
use quick_xml::{NsReader, events::Event, name::ResolveResult};
use reqwest::{Method, StatusCode, blocking::Client, redirect::Policy};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    io::Read,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub struct Credentials {
    pub username: String,
    pub password: String,
    pub device: String,
}
impl Drop for Credentials {
    fn drop(&mut self) {
        self.password.clear();
    }
}
#[derive(Clone)]
pub struct Dav {
    client: Client,
    base: reqwest::Url,
    auth: Arc<Credentials>,
    cancel: Arc<AtomicBool>,
    directories: Arc<Mutex<BTreeSet<String>>>,
}
pub struct Response {
    pub status: u16,
    pub etag: Option<String>,
    pub bytes: Vec<u8>,
}
pub fn strong_etag(s: &str) -> bool {
    s.len() >= 2
        && s.len() <= 512
        && s.starts_with('"')
        && s.ends_with('"')
        && s[1..s.len() - 1]
            .bytes()
            .all(|b| b >= 0x21 && b != 0x22 && b != 0x7f)
}
impl Dav {
    pub fn new(config: &Config, auth: Credentials, cancel: Arc<AtomicBool>) -> Result<Self> {
        config.validate()?;
        Self::build(
            &format!("{}{}/", config.server, config.folder),
            auth,
            cancel,
        )
    }
    fn build(url: &str, auth: Credentials, cancel: Arc<AtomicBool>) -> Result<Self> {
        let base = reqwest::Url::parse(url).map_err(|_| invalid("WebDAV地址无效。"))?;
        let client = Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(6))
            .timeout(Duration::from_secs(30))
            .user_agent("ResearchClip-Sync/2")
            .build()
            .map_err(|_| invalid("无法建立TLS网络客户端。"))?;
        Ok(Self {
            client,
            base,
            auth: Arc::new(auth),
            cancel,
            directories: Arc::new(Mutex::new(BTreeSet::new())),
        })
    }
    #[cfg(test)]
    pub fn local(url: &str, cancel: Arc<AtomicBool>) -> Self {
        assert!(url.starts_with("http://127.0.0.1:"));
        Self::build(
            url,
            Credentials {
                username: "test".into(),
                password: "isolated".into(),
                device: "a".repeat(32),
            },
            cancel,
        )
        .unwrap()
    }
    pub fn stopped(&self) -> Result<()> {
        if self.cancel.load(Ordering::Acquire) {
            Err(invalid(
                "同步已停止；已完成资源保留，未确认操作可安全重试。",
            ))
        } else {
            Ok(())
        }
    }
    pub fn request(
        &self,
        method: &str,
        path: &str,
        condition: Option<(&str, &str)>,
        data: Option<Vec<u8>>,
        limit: usize,
    ) -> Result<Response> {
        self.stopped()?;
        if path.starts_with('/')
            || path.contains("..")
            || path.contains(['\\', '%', '?', '#'])
            || path
                .bytes()
                .any(|c| !c.is_ascii_alphanumeric() && !b"-_/ .".contains(&c))
        {
            return Err(invalid("WebDAV资源路径无效。"));
        }
        let url = self
            .base
            .join(path)
            .map_err(|_| invalid("WebDAV资源地址无效。"))?;
        if url.origin() != self.base.origin() || !url.path().starts_with(self.base.path()) {
            return Err(invalid("资源越出专属同步目录。"));
        }
        let mut req = self
            .client
            .request(
                Method::from_bytes(method.as_bytes()).map_err(|_| invalid("WebDAV方法无效。"))?,
                url,
            )
            .basic_auth(&self.auth.username, Some(&self.auth.password))
            .header("Cache-Control", "no-cache");
        if method == "PROPFIND" {
            req = req
                .header("Depth", "1")
                .header("Content-Type", "application/xml");
        }
        if let Some((name, value)) = condition {
            req = req.header(name, value);
        }
        if let Some(data) = data {
            if data.len() > MAX_BLOB {
                return Err(invalid("上传超过64MiB。"));
            }
            req = req.body(data);
        }
        let mut res = req
            .send()
            .map_err(|_| invalid("网络、TLS或超时错误；同步操作已保留，请检查连接后重试。"))?;
        let status = res.status().as_u16();
        if [401, 403].contains(&status) {
            return Err(invalid(
                "WebDAV认证或权限失败，请检查第三方应用授权密码与目录权限。",
            ));
        }
        if [429, 507].contains(&status) {
            return Err(invalid(
                "服务器请求频率或空间不足，请稍后重试；未退回无条件上传。",
            ));
        }
        let etag = res
            .headers()
            .get("ETag")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        if res.content_length().is_some_and(|n| n > limit as u64) {
            return Err(invalid("服务器响应超过限制。"));
        }
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 65536];
        loop {
            self.stopped()?;
            let n = res
                .read(&mut chunk)
                .map_err(|_| invalid("下载不完整；未激活资源，请重试。"))?;
            if n == 0 {
                break;
            }
            if bytes.len() + n > limit {
                return Err(invalid("服务器响应超过限制。"));
            }
            bytes.extend_from_slice(&chunk[..n]);
        }
        Ok(Response {
            status,
            etag,
            bytes,
        })
    }
    pub fn propfind(&self, path: &str) -> Result<usize> {
        let r=self.request("PROPFIND",path,None,Some(br#"<?xml version="1.0"?><d:propfind xmlns:d="DAV:"><d:prop><d:resourcetype/></d:prop></d:propfind>"#.to_vec()),1024*1024)?;
        if r.status != 207 {
            return Err(invalid("服务器未返回WebDAV目录状态207。"));
        }
        validate_multistatus(&r.bytes)
    }
    pub fn mkdir(&self, path: &str) -> Result<()> {
        if self
            .directories
            .lock()
            .map_err(|_| invalid("同步目录锁异常。"))?
            .contains(path)
        {
            return Ok(());
        }
        let r = self.request("MKCOL", path, None, None, 65536)?;
        match r.status {
            201 => {}
            405 => {
                self.propfind(path)?;
            }
            _ => return Err(invalid("无法建立或验证专属同步目录。")),
        };
        self.directories
            .lock()
            .map_err(|_| invalid("同步目录锁异常。"))?
            .insert(path.into());
        Ok(())
    }
    pub fn get(&self, path: &str, limit: usize) -> Result<Option<Response>> {
        let r = self.request("GET", path, None, None, limit)?;
        match r.status {
            200 => Ok(Some(r)),
            404 => Ok(None),
            _ => Err(invalid("读取WebDAV资源失败，原资料未修改。")),
        }
    }
    pub fn put_new(&self, path: &str, bytes: Vec<u8>) -> Result<Response> {
        self.request(
            "PUT",
            path,
            Some(("If-None-Match", "*")),
            Some(bytes),
            65536,
        )
    }
    pub fn object(&self, hash: &str, limit: usize) -> Result<Vec<u8>> {
        if !hash_valid(hash) {
            return Err(invalid("同步对象摘要无效。"));
        }
        let r = self
            .get(&format!("objects/{}/{hash}", &hash[..2]), limit)?
            .ok_or(invalid("远端资源依赖缺失，未激活资料。"))?;
        if super::model::hash(&r.bytes) != hash {
            return Err(invalid("资源摘要不符或下载不完整，未激活资料。"));
        }
        Ok(r.bytes)
    }
    pub fn upload_object(&self, hash: &str, bytes: Vec<u8>) -> Result<()> {
        if !hash_valid(hash) || super::model::hash(&bytes) != hash {
            return Err(invalid("待上传对象校验失败。"));
        }
        self.mkdir("objects/")?;
        self.mkdir(&format!("objects/{}/", &hash[..2]))?;
        let path = format!("objects/{}/{hash}", &hash[..2]);
        // Existing content-addressed objects are read and verified, never overwritten.
        if let Some(existing) = self.get(&path, MAX_BLOB)? {
            if super::model::hash(&existing.bytes) == hash {
                return Ok(());
            }
            return Err(invalid("远端同名内容对象损坏；已有对象未覆盖。"));
        }
        let r = self.put_new(&path, bytes)?;
        if ![201, 204, 412].contains(&r.status) {
            return Err(invalid("不可变对象上传失败，未提交资源头。"));
        }
        self.object(hash, MAX_BLOB)?;
        Ok(())
    }
    pub fn initialize(&self, id: &str) -> Result<String> {
        self.mkdir("")?;
        let r = self.get("workspace.json", 4096)?;
        if r.is_none() {
            let bytes = json(&Workspace {
                protocol: 2,
                id: id.into(),
            })?;
            let status = self.put_new("workspace.json", bytes)?.status;
            if ![201, 204, 412].contains(&status) {
                return Err(invalid("不能创建同步空间。"));
            }
        }
        let w: Workspace = parse(
            &self
                .get("workspace.json", 4096)?
                .ok_or(invalid("同步空间不存在。"))?
                .bytes,
            4096,
        )?;
        if w.protocol != 2 || !crate::storage::files::id_valid(&w.id) {
            return Err(invalid("该目录不是兼容的ResearchClip同步空间。"));
        }
        Ok(w.id)
    }
    pub fn catalog(&self, workspace: &str) -> Result<(Catalog, Option<String>)> {
        match self.get("catalog.json", MAX_CATALOG)? {
            None => Ok((
                Catalog {
                    protocol: 2,
                    workspace: workspace.into(),
                    entries: Default::default(),
                },
                None,
            )),
            Some(r) => {
                let tag = r
                    .etag
                    .filter(|t| strong_etag(t))
                    .ok_or(invalid("服务器缺少强ETag，禁止同步写入。"))?;
                let c: Catalog = parse(&r.bytes, MAX_CATALOG)?;
                c.validate(workspace)?;
                Ok((c, Some(tag)))
            }
        }
    }
    pub fn publish(
        &self,
        workspace: &str,
        key: &str,
        expected: Option<&str>,
        desired: &str,
        project: Option<&str>,
    ) -> Result<String> {
        for _ in 0..3 {
            self.stopped()?;
            let (mut catalog, etag) = self.catalog(workspace)?;
            if catalog.entries.get(key).map(|h| h.manifest.as_str()) == Some(desired) {
                return Ok(etag.unwrap_or_default());
            }
            if catalog.entries.get(key).map(|h| h.manifest.as_str()) != expected {
                return Err(crate::storage::StorageError::Conflict);
            }
            catalog.entries.insert(
                key.into(),
                Head {
                    manifest: desired.into(),
                    project: project.map(str::to_owned),
                },
            );
            catalog.validate(workspace)?;
            let bytes = json(&catalog)?;
            if bytes.len() > MAX_CATALOG {
                return Err(invalid("远端目录索引超过2MiB，请缩小工作区规模。"));
            }
            let condition = etag
                .as_deref()
                .map(|v| ("If-Match", v))
                .unwrap_or(("If-None-Match", "*"));
            let r = self.request("PUT", "catalog.json", Some(condition), Some(bytes), 65536)?;
            if r.status == 412 {
                continue;
            }
            if ![200, 201, 204].contains(&r.status) {
                return Err(invalid("资源头提交失败；请重试确认，不会无条件覆盖。"));
            }
            let (c, t) = self.catalog(workspace)?;
            if c.entries.get(key).map(|h| h.manifest.as_str()) == Some(desired) {
                return t.ok_or(invalid("提交后缺少ETag。"));
            }
            return Err(crate::storage::StorageError::Conflict);
        }
        Err(invalid("其他设备持续写入，请稍后重试。"))
    }
    pub fn probe(&self, id: &str) -> Result<usize> {
        if !crate::storage::files::id_valid(id) {
            return Err(invalid("探针标识无效。"));
        }
        self.mkdir("")?;
        let entries = self.propfind("")?;
        self.mkdir("probes/")?;
        let dir = format!("probes/{id}/");
        let created = self.request("MKCOL", &dir, None, None, 65536)?;
        if created.status != 201 {
            return Err(invalid("专属探针目录已存在，未执行覆盖。"));
        }
        let path = format!("{dir}conditional.txt");
        if ![201, 204].contains(&self.put_new(&path, b"first".to_vec())?.status) {
            return Err(invalid("服务器拒绝条件创建。"));
        }
        if self.put_new(&path, b"overwrite".to_vec())?.status != 412 {
            return Err(invalid("服务器忽略If-None-Match，双向同步已禁用。"));
        }
        let first = self.get(&path, 1000)?.ok_or(invalid("探针文件丢失。"))?;
        if first.bytes != b"first" {
            return Err(invalid("条件创建不可靠，双向同步已禁用。"));
        }
        let etag = first
            .etag
            .filter(|t| strong_etag(t))
            .ok_or(invalid("服务器不返回强ETag，双向同步已禁用。"))?;
        let good = self.request(
            "PUT",
            &path,
            Some(("If-Match", &etag)),
            Some(b"second".to_vec()),
            1000,
        )?;
        if !StatusCode::from_u16(good.status).is_ok_and(|s| s.is_success()) {
            return Err(invalid("服务器拒绝有效ETag写入。"));
        }
        for tag in [&etag, "\"researchclip-forged-etag\""] {
            if self
                .request(
                    "PUT",
                    &path,
                    Some(("If-Match", tag)),
                    Some(b"bad".to_vec()),
                    1000,
                )?
                .status
                != 412
            {
                return Err(invalid("服务器未拒绝过期或伪造ETag，禁止同步。"));
            }
        }
        let second = self.get(&path, 1000)?.ok_or(invalid("探针读取失败。"))?;
        if second.bytes != b"second" {
            return Err(invalid("ETag条件写入内容被覆盖，禁止同步。"));
        }
        let etag = second
            .etag
            .filter(|s| strong_etag(s))
            .ok_or(invalid("探针缺少强ETag。"))?;
        let statuses = std::thread::scope(|scope| {
            let a = scope.spawn(|| {
                self.request(
                    "PUT",
                    &path,
                    Some(("If-Match", &etag)),
                    Some(b"race-A".to_vec()),
                    1000,
                )
            });
            let b = scope.spawn(|| {
                self.request(
                    "PUT",
                    &path,
                    Some(("If-Match", &etag)),
                    Some(b"race-B".to_vec()),
                    1000,
                )
            });
            Ok::<_, crate::storage::StorageError>((
                a.join().map_err(|_| invalid("并发探针异常。"))??.status,
                b.join().map_err(|_| invalid("并发探针异常。"))??.status,
            ))
        })?;
        let success = |n: u16| (200..300).contains(&n);
        if !((success(statuses.0) && statuses.1 == 412)
            || (success(statuses.1) && statuses.0 == 412))
        {
            return Err(invalid("并发条件写入不可靠，禁止同步。"));
        }
        let final_bytes = self
            .get(&path, 1000)?
            .ok_or(invalid("探针最终文件缺失。"))?
            .bytes;
        if final_bytes
            != if success(statuses.0) {
                b"race-A"
            } else {
                b"race-B"
            }
        {
            return Err(invalid("并发写入结果校验失败。"));
        }
        Ok(entries)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Workspace {
    protocol: u32,
    id: String,
}
pub fn validate_multistatus(bytes: &[u8]) -> Result<usize> {
    if bytes.len() > 1024 * 1024 {
        return Err(invalid("WebDAV目录响应过大。"));
    }
    let mut reader = NsReader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let (mut depth, mut count, mut root, mut status) = (0usize, 0usize, false, false);
    loop {
        let (ns, event) = reader
            .read_resolved_event()
            .map_err(|_| invalid("WebDAV XML无效。"))?;
        match event {
            Event::DocType(_) => return Err(invalid("WebDAV XML禁止DTD或实体声明。")),
            Event::Start(ref e) => {
                depth += 1;
                if depth > 32 {
                    return Err(invalid("WebDAV XML嵌套过深。"));
                }
                let dav = matches!(ns,ResolveResult::Bound(n) if n.as_ref()=="DAV:");
                if depth == 1 {
                    root = dav && e.local_name().as_ref() == "multistatus";
                }
                if dav && e.local_name().as_ref() == "response" {
                    count += 1;
                    if count > 1000 {
                        return Err(invalid("目录过大，未将截断响应当作完整列表。"));
                    }
                }
                status = dav && e.local_name().as_ref() == "status";
            }
            Event::Text(t) if status => {
                let text = t.as_ref();
                if !text.split_whitespace().nth(1).is_some_and(|s| s == "200") {
                    return Err(invalid("WebDAV目录包含失败的属性状态。"));
                }
            }
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or(invalid("XML层级无效。"))?;
                status = false;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !root || depth != 0 || count == 0 {
        return Err(invalid("WebDAV目录响应缺少有效集合。"));
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn etag_and_xml_fail_closed() {
        assert!(strong_etag("\"abc\""));
        assert!(!strong_etag("W/\"abc\""));
        assert!(!strong_etag("\"bad\n\""));
        assert!(validate_multistatus(br#"<d:multistatus xmlns:d="DAV:"><d:response><d:propstat><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"#).is_ok());
        for xml in [
            "<!DOCTYPE a [<!ENTITY secret SYSTEM 'file:///secret'>]><a/>",
            "<multistatus><response/></multistatus>",
            "<d:multistatus xmlns:d='DAV:'><d:response><d:propstat><d:status>HTTP/1.1 403 Forbidden</d:status></d:propstat></d:response></d:multistatus>",
        ] {
            assert!(validate_multistatus(xml.as_bytes()).is_err());
        }
    }
    #[test]
    #[ignore = "requires separately started scripts/qa-webdav-server.py"]
    fn real_wsgidav_conditionals_and_catalog_cas() {
        let url = std::env::var("RESEARCHCLIP_QA_DAV_BASE").expect("isolated test URL required");
        let dav = Dav::local(&url, Arc::new(AtomicBool::new(false)));
        let root = tempfile::tempdir().unwrap();
        let s = crate::storage::Storage::open(root.path().into()).unwrap();
        let id = s.sync_id_for_test().unwrap();
        assert!(dav.probe(&id).unwrap() >= 1);
        let workspace = dav.initialize(&id).unwrap();
        let key = format!("project/{id}");
        let digest = hash(b"payload");
        dav.upload_object(&digest, b"payload".to_vec()).unwrap();
        assert_eq!(dav.object(&digest, 100).unwrap(), b"payload");
        dav.publish(&workspace, &key, None, &digest, Some(&id))
            .unwrap();
        assert!(
            dav.publish(&workspace, &key, None, &hash("bad".as_bytes()), Some(&id))
                .is_err()
        );
        assert_eq!(
            dav.catalog(&workspace).unwrap().0.entries[&key].manifest,
            digest
        );
    }
}
