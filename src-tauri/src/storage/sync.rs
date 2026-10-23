//! Resource-level synchronization; domain records stay in their own tables.
use super::{Storage, StorageError, files};
use crate::services::sync::model::{Screenshot as WireScreenshot, *};
use rusqlite::{OptionalExtension, params};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub id: String,
    pub resource_key: String,
    pub reason: String,
    pub local_manifest: Option<String>,
    pub remote_manifest: Option<String>,
}
pub struct Base {
    pub content: String,
    pub manifest: String,
}
pub struct Pending {
    pub key: String,
    pub manifest: String,
    pub expected: Option<String>,
}
impl Storage {
    pub fn sync_new_id(&self) -> Result<String> {
        self.new_id()
    }
    pub fn sync_credential_id(&self, c: &Config) -> String {
        hash(format!("{}:{}", c.id(), self.data_dir.to_string_lossy()).as_bytes())[..32].into()
    }
    pub fn sync_bind_device(&self, device: &str) -> Result<()> {
        if !files::id_valid(device) {
            return Err(invalid("设备身份无效。"));
        }
        self.connection.execute(
            "UPDATE sync_identity SET device_id=?1 WHERE singleton=1",
            [device],
        )?;
        Ok(())
    }
    pub fn sync_config(&self) -> Result<Option<Config>> {
        self.connection
            .query_row(
                "SELECT config_json FROM sync_config WHERE singleton=1",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .map(|s| parse(s.as_bytes(), 32768))
            .transpose()
    }
    pub fn sync_save_config(&self, c: &Config) -> Result<()> {
        c.validate()?;
        self.connection.execute("INSERT INTO sync_config VALUES(1,?1) ON CONFLICT(singleton) DO UPDATE SET config_json=?1",[String::from_utf8(json(c)?).map_err(|_|invalid("配置编码无效。"))?])?;
        Ok(())
    }
    pub fn sync_cache_path(&self, digest: &str) -> Result<PathBuf> {
        if !hash_valid(digest) {
            return Err(invalid("同步缓存摘要无效。"));
        }
        let dir = self.data_dir.join("sync-objects");
        files::directory(&dir)?;
        let sub = dir.join(&digest[..2]);
        files::directory(&sub)?;
        let path = sub.join(digest);
        files::existing_ordinary(&path)?;
        Ok(path)
    }
    pub fn sync_cache(&self, bytes: &[u8]) -> Result<Blob> {
        if bytes.len() > MAX_BLOB {
            return Err(invalid("单个同步文件超过64MiB。"));
        }
        let digest = hash(bytes);
        let path = self.sync_cache_path(&digest)?;
        if path.exists() {
            if self.sync_blob(&digest, MAX_BLOB)? != bytes {
                return Err(invalid("本地内容缓存摘要不一致，未覆盖。"));
            }
        } else {
            let mut temp = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
            temp.write_all(bytes)?;
            temp.as_file().sync_all()?;
            temp.persist_noclobber(&path)
                .map_err(|e| StorageError::Io(e.error))?;
        }
        Ok(Blob {
            hash: digest,
            size: bytes.len(),
        })
    }
    pub fn sync_blob(&self, digest: &str, limit: usize) -> Result<Vec<u8>> {
        let bytes = limited_read(&self.sync_cache_path(digest)?, limit)?;
        if hash(&bytes) != digest {
            return Err(invalid("本地同步缓存损坏，原资料未改动。"));
        }
        Ok(bytes)
    }
    pub fn sync_manifest(&self, digest: &str, workspace: &str) -> Result<Manifest> {
        let m: Manifest = parse(
            &self.sync_blob(digest, MAX_RESOURCE + 8192)?,
            MAX_RESOURCE + 8192,
        )?;
        m.validate(workspace)?;
        Ok(m)
    }
    pub fn sync_base(&self, connection: &str, key: &str) -> Result<Option<Base>> {
        Ok(self.connection.query_row("SELECT content_hash,manifest_hash FROM sync_state WHERE connection_id=?1 AND resource_key=?2",params![connection,key],|r|Ok(Base{content:r.get(0)?,manifest:r.get(1)?})).optional()?)
    }
    pub fn sync_ack(
        &self,
        connection: &str,
        key: &str,
        content: &str,
        manifest: &str,
        etag: &str,
    ) -> Result<()> {
        let tx = self.connection.unchecked_transaction()?;
        tx.execute("INSERT INTO sync_state VALUES(?1,?2,?3,?4,?5) ON CONFLICT(connection_id,resource_key) DO UPDATE SET content_hash=?3,manifest_hash=?4,remote_etag=?5",params![connection,key,content,manifest,etag])?;
        tx.execute("UPDATE sync_queue SET phase='acknowledged' WHERE connection_id=?1 AND resource_key=?2 AND manifest_hash=?3",params![connection,key,manifest])?;
        tx.commit()?;
        Ok(())
    }
    pub fn sync_pending(&self, connection: &str) -> Result<Vec<Pending>> {
        Ok(self.connection.prepare("SELECT resource_key,manifest_hash,expected_remote FROM sync_queue WHERE connection_id=?1 AND phase!='acknowledged' ORDER BY rowid LIMIT 5001")?.query_map([connection],|r|Ok(Pending{key:r.get(0)?,manifest:r.get(1)?,expected:r.get(2)?}))?.collect::<rusqlite::Result<_>>()?)
    }
    pub fn sync_archive(
        &mut self,
        workspace: &str,
        device: &str,
        snapshot: Snapshot,
        parents: Vec<String>,
    ) -> Result<String> {
        snapshot.validate()?;
        let counter: i64 = self.connection.query_row(
            "UPDATE sync_identity SET counter=counter+1 WHERE singleton=1 RETURNING counter",
            [],
            |r| r.get(0),
        )?;
        let m = Manifest {
            protocol: 2,
            workspace: workspace.into(),
            device: device.into(),
            counter: counter.to_string(),
            parents,
            content_hash: snapshot.content()?,
            snapshot,
        };
        m.validate(workspace)?;
        Ok(self.sync_cache(&json(&m)?)?.hash)
    }
    pub fn sync_prepare(
        &mut self,
        connection: &str,
        workspace: &str,
        device: &str,
        snapshot: Snapshot,
        expected: Option<&str>,
        extra_parent: Option<&str>,
    ) -> Result<String> {
        let key = snapshot.key.clone();
        let mut parents: Vec<String> = expected
            .into_iter()
            .chain(extra_parent)
            .map(str::to_owned)
            .collect();
        parents.sort();
        parents.dedup();
        let digest = self.sync_archive(workspace, device, snapshot, parents)?;
        let n=self.connection.execute("INSERT INTO sync_queue VALUES(lower(hex(randomblob(16))),?1,?2,?3,?4,'prepared') ON CONFLICT(connection_id,resource_key) DO UPDATE SET operation_id=lower(hex(randomblob(16))),manifest_hash=?3,expected_remote=?4,phase='prepared' WHERE sync_queue.phase='acknowledged'",params![connection,key,digest,expected])?;
        if n != 1 {
            return Err(StorageError::Conflict);
        }
        Ok(digest)
    }
    pub fn sync_uncertain(&self, connection: &str, key: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE sync_queue SET phase='uncertain' WHERE connection_id=?1 AND resource_key=?2",
            params![connection, key],
        )?;
        Ok(())
    }
    pub fn sync_conflicts(&self, connection: &str) -> Result<Vec<Conflict>> {
        Ok(self.connection.prepare("SELECT id,resource_key,reason,local_manifest,remote_manifest FROM sync_conflicts WHERE connection_id=?1 AND resolved=0 ORDER BY rowid LIMIT 5000")?.query_map([connection],|r|Ok(Conflict{id:r.get(0)?,resource_key:r.get(1)?,reason:r.get(2)?,local_manifest:r.get(3)?,remote_manifest:r.get(4)?}))?.collect::<rusqlite::Result<_>>()?)
    }
    pub fn sync_conflict(
        &self,
        connection: &str,
        key: &str,
        local: Option<&str>,
        remote: Option<&str>,
        reason: &str,
    ) -> Result<()> {
        self.connection.execute("INSERT INTO sync_conflicts VALUES(lower(hex(randomblob(16))),?1,?2,?3,?4,?5,0) ON CONFLICT(connection_id,resource_key) DO UPDATE SET local_manifest=?3,remote_manifest=?4,reason=?5,resolved=0",params![connection,key,local,remote,reason])?;
        Ok(())
    }
    pub fn sync_resolved(&self, connection: &str, key: &str) -> Result<()> {
        let tx = self.connection.unchecked_transaction()?;
        tx.execute(
            "UPDATE sync_conflicts SET resolved=1 WHERE connection_id=?1 AND resource_key=?2",
            params![connection, key],
        )?;
        tx.execute(
            "UPDATE sync_queue SET phase='acknowledged' WHERE connection_id=?1 AND resource_key=?2",
            params![connection, key],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn sync_keys(&self, c: &Config) -> Result<Vec<String>> {
        let count:i64=self.connection.query_row("SELECT (SELECT count(*) FROM screenshots)+(SELECT count(*) FROM markdown_documents)+(SELECT count(*) FROM projects)+(SELECT count(*) FROM sync_tombstones)",[],|r|r.get(0))?;
        if count > 5000 {
            return Err(invalid("当前同步工作区超过5000资源上限，已停止而非截断。"));
        }
        let mut keys = BTreeSet::new();
        for (table, kind) in [
            ("screenshots", "screenshot"),
            ("markdown_documents", "document"),
            ("projects", "project"),
        ] {
            let query = match table {
                "screenshots" => {
                    "SELECT s.id,p.project_id FROM screenshots s LEFT JOIN screenshot_projects p ON p.screenshot_id=s.id LIMIT 5001"
                }
                "markdown_documents" => "SELECT id,project_id FROM markdown_documents LIMIT 5001",
                _ => "SELECT id,id FROM projects LIMIT 501",
            };
            for row in self.connection.prepare(query)?.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
            })? {
                let (id, project) = row?;
                if c.selected(project.as_deref()) {
                    keys.insert(format!("{kind}/{id}"));
                }
            }
        }
        for row in self
            .connection
            .prepare("SELECT resource_key,project_id FROM sync_tombstones LIMIT 5001")?
            .query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
            })?
        {
            let (k, p) = row?;
            if c.selected(p.as_deref()) {
                keys.insert(k);
            }
        }
        if keys.len() > MAX_RESOURCES {
            return Err(invalid("本次同步超过5000资源，未静默截断。"));
        }
        Ok(keys.into_iter().collect())
    }
    fn sync_project(&self, id: Option<&str>) -> Result<Option<Project>> {
        id.map(|id| {
            self.connection
                .query_row(
                    "SELECT id,name,created_at,updated_at FROM projects WHERE id=?1",
                    [id],
                    |r| {
                        Ok(Project {
                            id: r.get(0)?,
                            name: r.get(1)?,
                            created: r.get(2)?,
                            updated: r.get(3)?,
                        })
                    },
                )
                .map_err(StorageError::from)
        })
        .transpose()
    }
    pub fn sync_snapshot(&self, key: &str) -> Result<Option<Snapshot>> {
        if !key_valid(key) {
            return Err(invalid("同步资源ID无效。"));
        }
        let (kind, id) = key.split_once('/').unwrap();
        let body = match kind {
            "screenshot" => {
                let Some(project_id)=self.connection.query_row("SELECT p.project_id FROM screenshots s LEFT JOIN screenshot_projects p ON p.screenshot_id=s.id WHERE s.id=?1",[id],|r|r.get::<_,Option<String>>(0)).optional()? else{return self.sync_deleted(key);};
                let s = self.get_screenshot(id)?;
                let(count,size):(u64,u64)=self.connection.query_row("SELECT count(*),coalesce(sum(length(CAST(annotation_json AS BLOB))),0) FROM annotation_revisions WHERE screenshot_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?)))?;
                if count > 1024 || size > 16 * 1024 * 1024 {
                    return Err(invalid("标注历史超过同步上限；没有删除或截断历史。"));
                }
                let(notes_count,notes_size):(u64,u64)=self.connection.query_row("SELECT count(*),coalesce(sum(length(CAST(content AS BLOB))),0) FROM notes WHERE screenshot_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?)))?;
                if notes_count > 128 || notes_size > 4 * 1024 * 1024 {
                    return Err(invalid("该截图笔记超过同步上限。"));
                }
                let head:String=self.connection.query_row("SELECT revision_id FROM annotation_heads WHERE screenshot_id=?1 AND needs_review=0",[id],|r|r.get(0)).map_err(|_|invalid("当前标注没有可靠历史映射，请先确认并保存标注。"))?;
                let mut history=self.connection.prepare("SELECT revision_id,parent_revision_id,annotation_json,created_at,description,schema_version FROM annotation_revisions WHERE screenshot_id=?1 ORDER BY version_number LIMIT 1025")?.query_map([id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?,r.get::<_,String>(4)?,r.get::<_,u32>(5)?)))?.map(|row|{let(id,parent,text,created,description,schema)=row?;Ok(Revision{id,parent,annotation:parse(text.as_bytes(),4*1024*1024)?,created,description,schema})}).collect::<Result<Vec<_>>>()?;
                sort_history(&mut history)?;
                let notes=self.connection.prepare("SELECT id,content,source_url,created_at,updated_at FROM notes WHERE screenshot_id=?1 ORDER BY id")?.query_map([id],|r|Ok(Note{id:r.get(0)?,content:r.get(1)?,source:r.get(2)?,created:r.get(3)?,updated:r.get(4)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
                let tags=self.connection.prepare("SELECT t.id,t.name FROM tags t JOIN screenshot_tags st ON st.tag_id=t.id WHERE st.screenshot_id=?1 ORDER BY t.id LIMIT 129")?.query_map([id],|r|Ok(Tag{id:r.get(0)?,name:r.get(1)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
                let ocr = self
                    .ocr_content(id)?
                    .filter(|o| o.recognition_status == "completed" || o.corrected)
                    .map(|o| Ocr {
                        text: o.recognized_text,
                        language: o.language,
                        engine: o.ocr_engine_version,
                        updated: o.updated_at,
                        corrected: o.corrected,
                    });
                Body::Screenshot(Box::new(WireScreenshot {
                    title: s.title,
                    width: s.width_px,
                    height: s.height_px,
                    created: s.created_at,
                    updated: s.updated_at,
                    favorite: s.is_favorite,
                    original: self.sync_cache(&self.screenshot_image(id, false)?)?,
                    annotation: s.annotation_data,
                    head,
                    history,
                    notes,
                    tags,
                    project: self.sync_project(project_id.as_deref())?,
                    ocr,
                }))
            }
            "project" => {
                let Some(p) = self.sync_project(Some(id)).optional_not_found()? else {
                    return self.sync_deleted(key);
                };
                Body::Project(p)
            }
            "document" => {
                let Some((path,name,p,created,updated))=self.connection.query_row("SELECT path,name,project_id,created_at,updated_at FROM markdown_documents WHERE id=?1",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,i64>(3)?,r.get::<_,i64>(4)?))).optional()? else{return self.sync_deleted(key);};
                let grant: Option<String> = self
                    .connection
                    .query_row(
                        "SELECT authorized_path FROM sync_document_grants WHERE document_id=?1",
                        [id],
                        |r| r.get(0),
                    )
                    .optional()?;
                if grant.as_deref() != Some(&path) {
                    return Err(invalid(
                        "项目包含尚未授权或路径已变的Markdown，请先选择目录授权只读同步。",
                    ));
                }
                let file = PathBuf::from(&path);
                let bytes = limited_read(&file, 2 * 1024 * 1024)?;
                let text = std::str::from_utf8(&bytes)
                    .map_err(|_| invalid("Markdown不是UTF-8，原文件未改动。"))?;
                let names = document_assets(text)?;
                let mut assets = std::collections::BTreeMap::new();
                let mut total = 0usize;
                for n in names {
                    let image = file
                        .parent()
                        .ok_or(invalid("文档目录无效。"))?
                        .join("assets")
                        .join(&n);
                    let data = limited_read(&image, MAX_BLOB)?;
                    crate::services::workflow::decode_png(&data)
                        .map_err(|_| invalid("Markdown图片不是有效PNG。"))?;
                    total += data.len();
                    if total > 128 * 1024 * 1024 {
                        return Err(invalid("Markdown图片总量超过128MiB。"));
                    }
                    assets.insert(n, self.sync_cache(&data)?);
                }
                if limited_read(&file, 2 * 1024 * 1024)? != bytes {
                    return Err(StorageError::Conflict);
                }
                Body::Document(Document {
                    name,
                    markdown: self.sync_cache(&bytes)?,
                    assets,
                    project: self.sync_project(p.as_deref())?,
                    created,
                    updated,
                })
            }
            _ => unreachable!(),
        };
        let project = match &body {
            Body::Screenshot(s) => s.project.as_ref().map(|p| p.id.clone()),
            Body::Document(d) => d.project.as_ref().map(|p| p.id.clone()),
            Body::Project(p) => Some(p.id.clone()),
            Body::Deleted => None,
        };
        let snapshot = Snapshot {
            key: key.into(),
            project,
            body,
        };
        snapshot.validate()?;
        Ok(Some(snapshot))
    }
    fn sync_deleted(&self, key: &str) -> Result<Option<Snapshot>> {
        Ok(self
            .connection
            .query_row(
                "SELECT project_id FROM sync_tombstones WHERE resource_key=?1",
                [key],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()?
            .map(|project| Snapshot {
                key: key.into(),
                project,
                body: Body::Deleted,
            }))
    }
    pub fn sync_authorize_documents(&self, directory: &Path) -> Result<usize> {
        ordinary_chain(directory)?;
        let root = directory.canonicalize()?;
        let tx = self.connection.unchecked_transaction()?;
        let mut count = 0;
        {
            let mut stmt = tx.prepare("SELECT id,path FROM markdown_documents LIMIT 5001")?;
            for row in
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            {
                let (id, path) = row?;
                let p = Path::new(&path);
                if p.starts_with(&root) && p.exists() {
                    ordinary_chain(p)?;
                    if p.canonicalize()?.starts_with(&root) {
                        tx.execute("INSERT INTO sync_document_grants VALUES(?1,?2) ON CONFLICT(document_id) DO UPDATE SET authorized_path=?2",params![id,path])?;
                        count += 1;
                    }
                }
            }
        }
        tx.commit()?;
        Ok(count)
    }
}
trait OptionalNotFound<T> {
    fn optional_not_found(self) -> Result<Option<T>>;
}
impl<T> OptionalNotFound<T> for Result<Option<T>> {
    fn optional_not_found(self) -> Result<Option<T>> {
        match self {
            Err(StorageError::Database(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
            other => other,
        }
    }
}
pub fn ordinary_chain(path: &Path) -> Result<()> {
    for p in path.ancestors() {
        files::ordinary(p)?;
    }
    Ok(())
}
pub fn limited_read(path: &Path, limit: usize) -> Result<Vec<u8>> {
    ordinary_chain(path)?;
    let file = fs::File::open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > limit as u64 {
        return Err(invalid("同步文件不是普通文件或超过上限。"));
    }
    let mut out = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut out)?;
    if out.len() > limit {
        return Err(invalid("同步文件读取超限。"));
    }
    Ok(out)
}
pub fn document_assets(text: &str) -> Result<BTreeSet<String>> {
    let names = crate::services::markdown_document::names(text)
        .map_err(|_| invalid("Markdown图片引用格式不支持。"))?;
    // Conservative preflight uses the same managed-asset convention as the
    // current editor. Unsupported local links fail instead of silently losing
    // resources when a received document moves to its own directory.
    for tail in text.split("](").skip(1) {
        let target = tail
            .split_once(')')
            .ok_or(invalid("Markdown链接未闭合，未上传。"))?
            .0
            .trim();
        if !target.starts_with("assets/")
            && !target.starts_with("https://")
            && !target.starts_with("http://")
            && !target.starts_with('#')
            && !target.starts_with("mailto:")
        {
            return Err(invalid(
                "Markdown含未支持的本地资源链接；请先导入素材并插入受管图片。",
            ));
        }
    }
    if text
        .lines()
        .any(|line| line.trim_start().starts_with('[') && line.contains("]:"))
        || text.to_ascii_lowercase().contains("<img")
    {
        return Err(invalid(
            "引用式图片或HTML图片尚未纳入同步，请改用应用内插入图片；原文件未改动。",
        ));
    }
    let mut remaining = text.to_owned();
    for name in &names {
        remaining = remaining.replace(&format!("assets/{name}"), "");
    }
    if remaining.contains("assets/")
        || remaining.contains("assets\\")
        || remaining.contains("file://")
        || remaining.contains("](../")
        || remaining.contains("](./")
    {
        return Err(invalid(
            "同步目前仅支持应用生成的assets/researchclip-ID.png资源；其他本地引用须先导入应用，原文未改写。",
        ));
    }
    Ok(names)
}

fn ensure_project(tx: &rusqlite::Connection, p: &Project) -> Result<()> {
    // Embedded references must not overwrite the independently synchronized project.
    tx.execute(
        "INSERT INTO projects VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO NOTHING",
        params![
            p.id,
            p.name,
            p.name.trim().to_ascii_lowercase(),
            p.created,
            p.updated
        ],
    )?;
    Ok(())
}
fn put_project(tx: &rusqlite::Connection, p: &Project) -> Result<()> {
    tx.execute("INSERT INTO projects VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET name=?2,normalized_name=?3,updated_at=?5",params![p.id,p.name,p.name.trim().to_ascii_lowercase(),p.created,p.updated])?;
    Ok(())
}
fn immutable_file(path: &Path, bytes: &[u8]) -> Result<()> {
    files::existing_ordinary(path)?;
    if path.exists() {
        if limited_read(path, MAX_BLOB)? != bytes {
            return Err(invalid("同名受管文件内容不同，未覆盖。"));
        }
        return Ok(());
    }
    let mut tmp = tempfile::NamedTempFile::new_in(path.parent().ok_or(invalid("受管目录无效。"))?)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist_noclobber(path)
        .map_err(|e| StorageError::Io(e.error))?;
    Ok(())
}
impl Storage {
    pub fn sync_apply(
        &mut self,
        connection: &str,
        manifest_hash: &str,
        remote: &Manifest,
        expected: Option<&str>,
        etag: &str,
    ) -> Result<()> {
        remote.snapshot.validate()?;
        let snapshot = &remote.snapshot;
        let key = &snapshot.key;
        let (kind, id) = key.split_once('/').ok_or(invalid("同步资源ID无效。"))?;
        let current = self.sync_snapshot(key)?;
        if current
            .as_ref()
            .map(|s| s.content())
            .transpose()?
            .as_deref()
            != expected
        {
            return Err(StorageError::Conflict);
        }
        let mut document_path = None;
        match &snapshot.body {
            Body::Screenshot(s) => {
                let original = self.sync_blob(&s.original.hash, MAX_BLOB)?;
                if original.len() != s.original.size {
                    return Err(invalid("原图大小与清单不符。"));
                }
                if let Some(Snapshot {
                    body: Body::Screenshot(old),
                    ..
                }) = &current
                    && old.original.hash != s.original.hash
                {
                    return Err(invalid("相同截图ID引用了不同原图，未覆盖不可变原图。"));
                }
                let frame = crate::services::workflow::decode_png(&original)
                    .map_err(|_| invalid("同步原图不是有效PNG。"))?;
                if (frame.width_px, frame.height_px) != (s.width, s.height) {
                    return Err(invalid("同步原图像素尺寸不符。"));
                }
                let thumb =
                    crate::services::screen_capture::encode_png(files::thumbnail_frame(&frame))
                        .map_err(|_| invalid("同步缩略图生成失败。"))?;
                immutable_file(
                    &files::image_path(&self.data_dir, &format!("originals/{id}.png"))?,
                    &original,
                )?;
                immutable_file(
                    &files::image_path(&self.data_dir, &format!("thumbnails/{id}.png"))?,
                    &thumb,
                )?;
            }
            Body::Document(d) => {
                let md = self.sync_blob(&d.markdown.hash, 2 * 1024 * 1024)?;
                if md.len() != d.markdown.size {
                    return Err(invalid("Markdown字节数不符。"));
                }
                let text =
                    std::str::from_utf8(&md).map_err(|_| invalid("远端Markdown不是UTF-8。"))?;
                if document_assets(text)? != d.assets.keys().cloned().collect() {
                    return Err(invalid("Markdown资源引用与清单不符，未激活文档。"));
                }
                // Every received revision gets its own directory. Existing external files
                // and previous revisions are never overwritten or automatically removed.
                let parent = self.data_dir.join("sync-documents");
                files::directory(&parent)?;
                let dir = parent.join(id);
                files::directory(&dir)?;
                let dir = dir.join(manifest_hash);
                files::directory(&dir)?;
                let assets = dir.join("assets");
                files::directory(&assets)?;
                for (name, blob) in &d.assets {
                    let bytes = self.sync_blob(&blob.hash, MAX_BLOB)?;
                    if bytes.len() != blob.size {
                        return Err(invalid("Markdown图片大小不符。"));
                    }
                    crate::services::workflow::decode_png(&bytes)
                        .map_err(|_| invalid("Markdown受管图片不是有效PNG。"))?;
                    immutable_file(&assets.join(name), &bytes)?;
                }
                let path = dir.join(&d.name);
                immutable_file(&path, &md)?;
                document_path = Some(path.canonicalize()?.to_string_lossy().into_owned());
            }
            _ => {}
        }
        if self
            .sync_snapshot(key)?
            .as_ref()
            .map(|s| s.content())
            .transpose()?
            .as_deref()
            != expected
        {
            return Err(StorageError::Conflict);
        }
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        match &snapshot.body {
            Body::Project(p) => put_project(&tx, p)?,
            Body::Screenshot(s) => {
                let exists: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM screenshots WHERE id=?1)",
                    [id],
                    |r| r.get(0),
                )?;
                if exists {
                    tx.execute("UPDATE screenshots SET title=?2,annotation_data=?3,created_at=?4,updated_at=?5,is_favorite=?6,revision=revision+1 WHERE id=?1",params![id,s.title,s.annotation.to_string(),s.created,s.updated,s.favorite])?;
                } else {
                    tx.execute(
                        "INSERT INTO screenshots VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,1)",
                        params![
                            id,
                            s.title,
                            format!("originals/{id}.png"),
                            format!("thumbnails/{id}.png"),
                            s.annotation.to_string(),
                            s.width,
                            s.height,
                            s.created,
                            s.updated,
                            s.favorite
                        ],
                    )?;
                    tx.execute("DELETE FROM annotation_heads WHERE screenshot_id=?1", [id])?;
                    tx.execute(
                        "DELETE FROM annotation_revisions WHERE screenshot_id=?1",
                        [id],
                    )?;
                }
                // A confirmed resurrection supersedes its previous file deletion intent.
                // Both managed images have already been validated/materialized above.
                tx.execute(
                    "DELETE FROM pending_file_deletions WHERE screenshot_id=?1",
                    [id],
                )?;
                for rev in &s.history {
                    let existing:Option<(String,String,Option<String>,i64,String,i64)>=tx.query_row("SELECT screenshot_id,annotation_json,parent_revision_id,created_at,description,schema_version FROM annotation_revisions WHERE revision_id=?1",[&rev.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?;
                    if let Some((owner, json, parent, created, description, schema)) = existing {
                        if owner != id
                            || parse::<serde_json::Value>(json.as_bytes(), 4 * 1024 * 1024)?
                                != rev.annotation
                            || parent != rev.parent
                            || created != rev.created
                            || description != rev.description
                            || schema != i64::from(rev.schema)
                        {
                            return Err(invalid("同一标注版本ID的数据不一致，未覆盖历史。"));
                        }
                    } else {
                        let n:i64=tx.query_row("SELECT coalesce(max(version_number),0)+1 FROM annotation_revisions WHERE screenshot_id=?1",[id],|r|r.get(0))?;
                        tx.execute(
                            "INSERT INTO annotation_revisions VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                            params![
                                rev.id,
                                id,
                                n,
                                rev.parent,
                                rev.annotation.to_string(),
                                rev.created,
                                rev.description,
                                rev.schema
                            ],
                        )?;
                    }
                }
                tx.execute("INSERT INTO annotation_heads VALUES(?1,?2,0) ON CONFLICT(screenshot_id) DO UPDATE SET revision_id=?2,needs_review=0",params![id,s.head])?;
                // All note rows are included; immutable conflict manifests preserve the old
                // values, while row revisions still protect any open editor's stale save.
                let incoming: BTreeSet<&str> = s.notes.iter().map(|n| n.id.as_str()).collect();
                let old_ids: Vec<String> = tx
                    .prepare("SELECT id FROM notes WHERE screenshot_id=?1")?
                    .query_map([id], |r| r.get(0))?
                    .collect::<rusqlite::Result<_>>()?;
                for old in old_ids {
                    if !incoming.contains(old.as_str()) {
                        tx.execute("DELETE FROM notes WHERE id=?1", [old])?;
                    }
                }
                for n in &s.notes {
                    let owner: Option<String> = tx
                        .query_row(
                            "SELECT screenshot_id FROM notes WHERE id=?1",
                            [&n.id],
                            |r| r.get(0),
                        )
                        .optional()?;
                    if owner.as_deref().is_some_and(|o| o != id) {
                        return Err(invalid("笔记ID属于其他截图，未覆盖。"));
                    }
                    tx.execute("INSERT INTO notes VALUES(?1,?2,?3,?4,?5,?6,1) ON CONFLICT(id) DO UPDATE SET content=?3,source_url=?4,created_at=?5,updated_at=?6,revision=revision+1",params![n.id,id,n.content,n.source,n.created,n.updated])?;
                }
                tx.execute("DELETE FROM screenshot_tags WHERE screenshot_id=?1", [id])?;
                for t in &s.tags {
                    let existing: Option<String> = tx
                        .query_row("SELECT name FROM tags WHERE id=?1", [&t.id], |r| r.get(0))
                        .optional()?;
                    if existing.as_ref().is_some_and(|name| name != &t.name) {
                        return Err(invalid(
                            "共享标签名称不同，请先核对并手动统一标签名称后重试。",
                        ));
                    }
                    tx.execute("INSERT INTO tags VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET name=?2,normalized_name=?3",params![t.id,t.name,t.name.trim().to_ascii_lowercase()])?;
                    tx.execute(
                        "INSERT INTO screenshot_tags VALUES(?1,?2)",
                        params![id, t.id],
                    )?;
                }
                tx.execute(
                    "DELETE FROM screenshot_projects WHERE screenshot_id=?1",
                    [id],
                )?;
                if let Some(p) = &s.project {
                    ensure_project(&tx, p)?;
                    tx.execute(
                        "INSERT INTO screenshot_projects VALUES(?1,?2)",
                        params![id, p.id],
                    )?;
                }
                if let Some(o) = &s.ocr {
                    tx.execute("INSERT INTO ocr_content VALUES(?1,?2,?3,'completed',?4,?5,?6,1,NULL,?7) ON CONFLICT(screenshot_id) DO UPDATE SET recognized_text=?2,language=?3,recognition_status='completed',ocr_engine_version=?4,updated_at=?5,source_image_revision=?6,revision=revision+1,error=NULL,corrected=?7",params![id,o.text,o.language,o.engine,o.updated,format!("original-v1:{id}"),o.corrected])?;
                }
            }
            Body::Document(d) => {
                if let Some(p) = &d.project {
                    ensure_project(&tx, p)?;
                }
                let path = document_path
                    .as_ref()
                    .ok_or(invalid("同步文档尚未完成落盘。"))?;
                tx.execute("INSERT INTO markdown_documents VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET path=?2,name=?3,project_id=?4,updated_at=?6",params![id,path,d.name,d.project.as_ref().map(|p|p.id.as_str()),d.created,d.updated])?;
                tx.execute("INSERT INTO sync_document_grants VALUES(?1,?2) ON CONFLICT(document_id) DO UPDATE SET authorized_path=?2",params![id,path])?;
            }
            Body::Deleted => match kind {
                "screenshot" => {
                    if let Some(old) = &current
                        && let Body::Screenshot(_) = old.body
                    {
                        for folder in ["originals", "thumbnails"] {
                            tx.execute(
                                "INSERT OR IGNORE INTO pending_file_deletions VALUES(?1,?2)",
                                params![format!("{folder}/{id}.png"), id],
                            )?;
                        }
                    }
                    tx.execute("DELETE FROM screenshots WHERE id=?1", [id])?;
                }
                "project" => {
                    tx.execute("DELETE FROM projects WHERE id=?1", [id])?;
                }
                "document" => {
                    tx.execute("DELETE FROM markdown_documents WHERE id=?1", [id])?;
                }
                _ => return Err(invalid("未知删除类型。")),
            },
        }
        if !matches!(snapshot.body, Body::Deleted) {
            tx.execute("DELETE FROM sync_tombstones WHERE resource_key=?1", [key])?;
        }
        // A conflict copy is a new local resource, not a remote baseline.
        if connection != "local-conflict-copy" {
            tx.execute("INSERT INTO sync_state VALUES(?1,?2,?3,?4,?5) ON CONFLICT(connection_id,resource_key) DO UPDATE SET content_hash=?3,manifest_hash=?4,remote_etag=?5",params![connection,key,remote.content_hash,manifest_hash,etag])?;
            tx.execute(
            "UPDATE sync_queue SET phase='acknowledged' WHERE connection_id=?1 AND resource_key=?2",
            params![connection, key],
        )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn sync_clone(&mut self, source: &Snapshot, seed: &str) -> Result<Snapshot> {
        let mut copy = source.clone();
        let derive = |value: &str| hash(format!("{seed}/{value}").as_bytes())[..32].to_owned();
        let id = derive(&source.key);
        copy.key = format!("{}/{}", source.key.split_once('/').unwrap().0, id);
        match &mut copy.body {
            Body::Screenshot(s) => {
                s.title = format!(
                    "{} [冲突副本 {}]",
                    s.title.chars().take(160).collect::<String>(),
                    &id[..6]
                );
                let mut map = std::collections::BTreeMap::new();
                for r in &s.history {
                    map.insert(r.id.clone(), derive(&r.id));
                }
                for r in &mut s.history {
                    r.parent = r.parent.as_ref().and_then(|p| map.get(p).cloned());
                    r.id = map[&r.id].clone();
                }
                s.head = map[&s.head].clone();
                sort_history(&mut s.history)?;
                for n in &mut s.notes {
                    n.id = derive(&n.id);
                }
            }
            Body::Document(d) => {
                d.name = format!("conflict-{}.md", &id[..12]);
            }
            Body::Project(p) => {
                p.id = id.clone();
                p.name = format!(
                    "{} [冲突{}]",
                    p.name.chars().take(60).collect::<String>(),
                    &id[..6]
                );
                copy.project = Some(id);
            }
            Body::Deleted => return Err(invalid("删除记录不能复制为空资料。")),
        }
        copy.validate()?;
        Ok(copy)
    }
}
impl super::Storage {
    #[cfg(test)]
    pub(crate) fn sync_id_for_test(&self) -> Result<String> {
        self.new_id()
    }
}
#[cfg(test)]
mod tests {
    use super::{Body, Snapshot};
    use crate::{capture::RawFrame, services::screen_capture::encode_png, storage::Storage};
    use serde_json::json;
    fn legacy(root: &std::path::Path) -> rusqlite::Connection {
        let connection = rusqlite::Connection::open(root.join("researchclip.sqlite3")).unwrap();
        for sql in [
            include_str!("../../migrations/001_workspace.sql"),
            include_str!("../../migrations/002_library.sql"),
            include_str!("../../migrations/003_projects.sql"),
            include_str!("../../migrations/004_ocr_index.sql"),
            include_str!("../../migrations/005_annotation_revisions.sql"),
        ] {
            connection.execute_batch(sql).unwrap();
        }
        connection.pragma_update(None, "user_version", 5).unwrap();
        connection
    }
    #[test]
    fn v5_upgrade_backs_up_preserves_ids_and_bad_migration_rolls_back() {
        let root = tempfile::tempdir().unwrap();
        let old = legacy(root.path());
        let id = "a".repeat(32);
        let doc = json!({"schemaVersion":1,"image":{"widthPx":8,"heightPx":6},"objects":[]});
        old.execute(
            "INSERT INTO screenshots VALUES(?1,'保留',?2,?3,?4,8,6,1,2,0,1)",
            rusqlite::params![
                id,
                format!("originals/{id}.png"),
                format!("thumbnails/{id}.png"),
                doc.to_string()
            ],
        )
        .unwrap();
        let revision: String = old
            .query_row("SELECT revision_id FROM annotation_revisions", [], |r| {
                r.get(0)
            })
            .unwrap();
        let image = encode_png(RawFrame {
            width_px: 8,
            height_px: 6,
            rgba: [10, 20, 30, 128].repeat(48),
        })
        .unwrap();
        for folder in ["originals", "thumbnails"] {
            std::fs::create_dir_all(root.path().join(folder)).unwrap();
            std::fs::write(root.path().join(folder).join(format!("{id}.png")), &image).unwrap();
        }
        drop(old);
        let s = Storage::open(root.path().into()).unwrap();
        assert_eq!(s.screenshot_image(&id, false).unwrap(), image);
        assert_eq!(
            s.status().unwrap().schema_version,
            crate::storage::migrations::SCHEMA_VERSION
        );
        assert_eq!(s.get_screenshot(&id).unwrap().annotation_data, doc);
        assert_eq!(
            s.annotation_history(&id, i64::MAX, 50).unwrap()[0].revision_id,
            revision
        );
        assert_eq!(
            s.connection
                .query_row("SELECT revision_id FROM annotation_heads", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            revision
        );
        let backup = std::fs::read_dir(root.path().join("backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let backup = rusqlite::Connection::open(backup).unwrap();
        assert_eq!(
            backup
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            5
        );
        let broken = tempfile::tempdir().unwrap();
        let mut old = legacy(broken.path());
        old.execute_batch(
            "CREATE TABLE sync_queue (keep TEXT);INSERT INTO sync_queue VALUES ('保留');",
        )
        .unwrap();
        assert!(super::super::migrations::migrate(&mut old, Some(broken.path())).is_err());
        assert_eq!(
            old.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            5
        );
        assert_eq!(
            old.query_row("SELECT keep FROM sync_queue", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "保留"
        );
        assert!(old.prepare("SELECT * FROM annotation_heads").is_err());
    }
    #[test]
    fn explicit_head_survives_branch_and_delete_intent_survives_cleanup() {
        let root = tempfile::tempdir().unwrap();
        let mut s = Storage::open(root.path().into()).unwrap();
        let doc = json!({"schemaVersion":1,"image":{"widthPx":8,"heightPx":6},"objects":[]});
        let png = encode_png(RawFrame {
            width_px: 8,
            height_px: 6,
            rgba: [10, 20, 30, 128].repeat(48),
        })
        .unwrap();
        let shot = s.create_screenshot("原图", doc.clone(), &png).unwrap();
        let first = s.annotation_history(&shot.id, i64::MAX, 50).unwrap()[0]
            .revision_id
            .clone();
        let mut changed = doc;
        changed["objects"] = json!([{"id":"t","type":"text","x":1,"y":1,"text":"分支","fontSize":12,"strokeWidth":1,"color":"#ff0000"}]);
        s.save_annotation_revision(&shot.id, 1, changed, "分支")
            .unwrap();
        s.connection
            .execute(
                "UPDATE annotation_heads SET revision_id=?1 WHERE screenshot_id=?2",
                rusqlite::params![first, shot.id],
            )
            .unwrap();
        assert_eq!(s.get_screenshot(&shot.id).unwrap().annotation_version, 1);
        assert_eq!(s.screenshot_image(&shot.id, false).unwrap(), png);
        s.connection
            .execute("DELETE FROM screenshots WHERE id=?1", [&shot.id])
            .unwrap();
        let key: String = s
            .connection
            .query_row("SELECT resource_key FROM sync_tombstones", [], |r| r.get(0))
            .unwrap();
        assert_eq!(key, format!("screenshot/{}", shot.id));
        drop(s);
        let s = Storage::open(root.path().into()).unwrap();
        assert_eq!(
            s.connection
                .query_row("SELECT count(*) FROM sync_tombstones", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn confirmed_resurrection_cancels_stale_image_cleanup_and_preserves_bytes() {
        let root = tempfile::tempdir().unwrap();
        let mut s = Storage::open(root.path().into()).unwrap();
        let png = encode_png(RawFrame {
            width_px: 8,
            height_px: 6,
            rgba: [10, 20, 30, 128].repeat(48),
        })
        .unwrap();
        let doc = json!({"schemaVersion":1,"image":{"widthPx":8,"heightPx":6},"objects":[]});
        let shot = s.create_screenshot("保留", doc, &png).unwrap();
        let key = format!("screenshot/{}", shot.id);
        let original = s.sync_snapshot(&key).unwrap().unwrap();
        let workspace = "f".repeat(32);
        let digest = s
            .sync_archive(&workspace, &"e".repeat(32), original.clone(), vec![])
            .unwrap();
        let manifest = s.sync_manifest(&digest, &workspace).unwrap();
        let deletion = Snapshot {
            key: key.clone(),
            project: None,
            body: Body::Deleted,
        };
        let deleted_hash = s
            .sync_archive(&workspace, &"e".repeat(32), deletion.clone(), vec![])
            .unwrap();
        let deleted_manifest = s.sync_manifest(&deleted_hash, &workspace).unwrap();
        s.sync_apply(
            "fixture",
            &deleted_hash,
            &deleted_manifest,
            Some(&original.content().unwrap()),
            "",
        )
        .unwrap();
        assert_eq!(s.library_status().unwrap().pending_cleanup, 2);
        s.sync_apply(
            "fixture",
            &digest,
            &manifest,
            Some(&deletion.content().unwrap()),
            "",
        )
        .unwrap();
        assert_eq!(s.reconcile().unwrap().pending_cleanup, 0);
        assert_eq!(s.screenshot_image(&shot.id, false).unwrap(), png);
        drop(s);
        let s = Storage::open(root.path().into()).unwrap();
        assert_eq!(s.screenshot_image(&shot.id, false).unwrap(), png);
    }
}
