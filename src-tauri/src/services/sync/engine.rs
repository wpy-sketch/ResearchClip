use super::{model::*, transport::Dav};
use crate::storage::{Storage, StorageError, sync::Conflict};
use std::collections::BTreeSet;

pub trait Store {
    fn access<T>(&self, f: impl FnOnce(&mut Storage) -> Result<T>) -> Result<T>;
}
impl Store for crate::commands::library::LibraryService {
    fn access<T>(&self, f: impl FnOnce(&mut Storage) -> Result<T>) -> Result<T> {
        self.access(f)
    }
}
#[cfg(test)]
impl Store for std::sync::Mutex<Storage> {
    fn access<T>(&self, f: impl FnOnce(&mut Storage) -> Result<T>) -> Result<T> {
        {
            let mut guard = self.lock().map_err(|_| invalid("测试资料锁异常。"))?;
            f(&mut guard)
        }
    }
}
#[derive(Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub completed: usize,
    pub total: usize,
    pub uploaded: usize,
    pub downloaded: usize,
    pub conflicts: usize,
    pub skipped: usize,
}
pub fn manifest(store: &impl Store, dav: &Dav, workspace: &str, digest: &str) -> Result<Manifest> {
    let local = store.access(|s| s.sync_manifest(digest, workspace));
    if let Ok(m) = local {
        return Ok(m);
    }
    let bytes = dav.object(digest, MAX_RESOURCE + 8192)?;
    let m: Manifest = parse(&bytes, MAX_RESOURCE + 8192)?;
    m.validate(workspace)?;
    store.access(|s| {
        s.sync_cache(&bytes)?;
        Ok(())
    })?;
    Ok(m)
}
pub fn download(store: &impl Store, dav: &Dav, m: &Manifest) -> Result<()> {
    for blob in m.snapshot.blobs() {
        dav.stopped()?;
        if store
            .access(|s| s.sync_blob(&blob.hash, MAX_BLOB))
            .is_ok_and(|data| data.len() == blob.size)
        {
            continue;
        }
        let bytes = dav.object(&blob.hash, MAX_BLOB)?;
        if bytes.len() != blob.size {
            return Err(invalid("下载大小与清单不符；没有登记半个资源。"));
        }
        store.access(|s| {
            s.sync_cache(&bytes)?;
            Ok(())
        })?;
    }
    Ok(())
}
fn push(
    store: &impl Store,
    dav: &Dav,
    workspace: &str,
    connection: &str,
    digest: &str,
    expected: Option<&str>,
) -> Result<()> {
    let m = store.access(|s| s.sync_manifest(digest, workspace))?;
    for blob in m.snapshot.blobs() {
        dav.stopped()?;
        let bytes = store.access(|s| s.sync_blob(&blob.hash, MAX_BLOB))?;
        if bytes.len() != blob.size {
            return Err(invalid("待上传文件大小与快照不符。"));
        }
        dav.upload_object(&blob.hash, bytes)?;
    }
    let bytes = store.access(|s| s.sync_blob(digest, MAX_RESOURCE + 8192))?;
    dav.upload_object(digest, bytes)?;
    // Mark uncertain before the conditional commit; a lost ACK reuses this exact
    // operation, manifest, revision and expected remote value on manual retry.
    store.access(|s| s.sync_uncertain(connection, &m.snapshot.key))?;
    let etag = dav.publish(
        workspace,
        &m.snapshot.key,
        expected,
        digest,
        m.snapshot.project.as_deref(),
    )?;
    store.access(|s| s.sync_ack(connection, &m.snapshot.key, &m.content_hash, digest, &etag))
}
fn preserve_conflict(
    store: &impl Store,
    identity: (&str, &str, &str),
    key: &str,
    local: Option<Snapshot>,
    remote: Option<&str>,
    reason: &str,
) -> Result<()> {
    let (workspace, device, connection) = identity;
    let previous = store
        .access(|s| s.sync_conflicts(connection))?
        .into_iter()
        .find(|c| c.resource_key == key)
        .and_then(|c| c.local_manifest);
    let local_hash = if let Some(snapshot) = local {
        let content = snapshot.content()?;
        let same = previous
            .as_ref()
            .map(|digest| store.access(|s| s.sync_manifest(digest, workspace)))
            .transpose()?
            .is_some_and(|m| m.content_hash == content);
        if same {
            previous
        } else {
            Some(store.access(|s| s.sync_archive(workspace, device, snapshot, vec![]))?)
        }
    } else {
        previous
    };
    store.access(|s| s.sync_conflict(connection, key, local_hash.as_deref(), remote, reason))
}
pub fn run(
    store: &impl Store,
    dav: &Dav,
    c: &Config,
    workspace: &str,
    device: &str,
    mut progress: impl FnMut(Progress),
) -> Result<Progress> {
    let connection = c.id();
    let mut stats = Progress::default();
    let pending = store.access(|s| s.sync_pending(&connection))?;
    if pending.len() > MAX_RESOURCES {
        return Err(invalid("待确认队列超过上限，未截断。"));
    }
    for p in pending {
        dav.stopped()?;
        let m = store.access(|s| s.sync_manifest(&p.manifest, workspace))?;
        if !c.selected(m.snapshot.project.as_deref()) {
            stats.skipped += 1;
            continue;
        }
        if store
            .access(|s| s.sync_conflicts(&connection))?
            .iter()
            .any(|r| r.resource_key == p.key)
        {
            continue;
        }
        match push(
            store,
            dav,
            workspace,
            &connection,
            &p.manifest,
            p.expected.as_deref(),
        ) {
            Ok(()) => stats.uploaded += 1,
            Err(StorageError::Conflict) => {
                let catalog = dav.catalog(workspace)?.0;
                let remote = catalog.entries.get(&p.key);
                if let Some(head) = remote {
                    let rm = manifest(store, dav, workspace, &head.manifest)?;
                    download(store, dav, &rm)?;
                }
                preserve_conflict(
                    store,
                    (workspace, device, &connection),
                    &p.key,
                    store.access(|s| s.sync_snapshot(&p.key))?,
                    remote.map(|h| h.manifest.as_str()),
                    "上次提交未确认且远端已改变；双方快照保留。",
                )?;
            }
            Err(e) => return Err(e),
        }
    }
    let (catalog, catalog_etag) = dav.catalog(workspace)?;
    let mut keys: BTreeSet<String> = store.access(|s| s.sync_keys(c))?.into_iter().collect();
    for (key, head) in &catalog.entries {
        if c.selected(head.project.as_deref()) {
            keys.insert(key.clone());
        }
    }
    if keys.len() > MAX_RESOURCES {
        return Err(invalid("同步合并超过5000资源上限，未截断。"));
    }
    stats.total = keys.len();
    progress(stats.clone());
    for key in keys {
        dav.stopped()?;
        let local = store.access(|s| s.sync_snapshot(&key))?;
        if local
            .as_ref()
            .is_some_and(|l| !c.selected(l.project.as_deref()))
        {
            stats.skipped += 1;
            stats.completed += 1;
            progress(stats.clone());
            continue;
        }
        let remote_hash = catalog
            .entries
            .get(&key)
            .filter(|head| c.selected(head.project.as_deref()))
            .map(|head| head.manifest.as_str());
        let remote = remote_hash
            .map(|h| manifest(store, dav, workspace, h))
            .transpose()?;
        if let Some(r) = &remote {
            let head = &catalog.entries[&key];
            if r.snapshot.key != key || r.snapshot.project != head.project {
                return Err(invalid("远端清单身份/范围与目录不一致，已停止。"));
            }
        }
        let base = store.access(|s| s.sync_base(&connection, &key))?;
        let lhash = local.as_ref().map(Snapshot::content).transpose()?;
        let rhash = remote.as_ref().map(|r| r.content_hash.as_str());
        let conflict = store
            .access(|s| s.sync_conflicts(&connection))?
            .into_iter()
            .find(|conflict| conflict.resource_key == key);
        if conflict.is_some() {
            if let Some(r) = &remote {
                download(store, dav, r)?;
            }
            preserve_conflict(
                store,
                (workspace, device, &connection),
                &key,
                local,
                remote_hash,
                "冲突未解决；已刷新双方已保存内容，请明确选择。",
            )?;
            stats.completed += 1;
            progress(stats.clone());
            continue;
        }
        if lhash.as_deref() == rhash {
            if let (Some(r), Some(h)) = (&remote, remote_hash) {
                store.access(|s| {
                    s.sync_ack(
                        &connection,
                        &key,
                        &r.content_hash,
                        h,
                        catalog_etag.as_deref().unwrap_or(""),
                    )
                })?;
            }
        } else if local.is_none()
            && remote
                .as_ref()
                .is_some_and(|r| matches!(r.snapshot.body, Body::Deleted))
        {
            let r = remote.as_ref().unwrap();
            store.access(|s| {
                s.sync_ack(
                    &connection,
                    &key,
                    &r.content_hash,
                    remote_hash.unwrap(),
                    catalog_etag.as_deref().unwrap_or(""),
                )
            })?;
        } else {
            let b = base.as_ref().map(|b| b.content.as_str());
            let l_changed = lhash.as_deref() != b;
            let r_changed = rhash != b;
            let local_deleted = local
                .as_ref()
                .is_some_and(|s| matches!(s.body, Body::Deleted));
            if base.is_none() && local_deleted && remote.is_none() {
                stats.skipped += 1;
            }
            // Unknown historical absence never propagates a deletion.
            else if local.is_some() && remote.is_none() && base.is_none()
                || l_changed && !r_changed && local.is_some()
            {
                let snapshot = local.unwrap();
                let digest = store.access(|s| {
                    s.sync_prepare(&connection, workspace, device, snapshot, remote_hash, None)
                })?;
                match push(store, dav, workspace, &connection, &digest, remote_hash) {
                    Ok(()) => stats.uploaded += 1,
                    Err(StorageError::Conflict) => {
                        let latest = dav.catalog(workspace)?.0;
                        let head = latest.entries.get(&key);
                        if let Some(h) = head {
                            let r = manifest(store, dav, workspace, &h.manifest)?;
                            download(store, dav, &r)?;
                        }
                        preserve_conflict(
                            store,
                            (workspace, device, &connection),
                            &key,
                            store.access(|s| s.sync_snapshot(&key))?,
                            head.map(|h| h.manifest.as_str()),
                            "上传期间远端改变，未覆盖；请处理冲突。",
                        )?;
                    }
                    Err(e) => return Err(e),
                }
            } else if let Some(r) = remote.as_ref()
                && ((base.is_none() && local.is_none()) || !l_changed && r_changed)
                && !matches!(r.snapshot.body, Body::Deleted)
            {
                download(store, dav, r)?;
                match store.access(|s| {
                    s.sync_apply(
                        &connection,
                        remote_hash.unwrap(),
                        r,
                        lhash.as_deref(),
                        catalog_etag.as_deref().unwrap_or(""),
                    )
                }) {
                    Ok(()) => stats.downloaded += 1,
                    Err(_) => {
                        preserve_conflict(
                            store,
                            (workspace, device, &connection),
                            &key,
                            local,
                            remote_hash,
                            "本地导入遇到并发修改、同名约束或原图/历史冲突；未覆盖原资料。",
                        )?;
                    }
                }
            } else {
                if let Some(r) = &remote {
                    download(store, dav, r)?;
                }
                preserve_conflict(
                    store,
                    (workspace, device, &connection),
                    &key,
                    local,
                    remote_hash,
                    if remote
                        .as_ref()
                        .is_some_and(|r| matches!(r.snapshot.body, Body::Deleted))
                    {
                        "远端删除需要逐项确认；本地资料和双方快照均保留。"
                    } else {
                        "双方已修改或远端记录缺失；请采用本地、远端或保留两者。"
                    },
                )?;
            }
        }
        stats.completed += 1;
        stats.conflicts = store.access(|s| s.sync_conflicts(&connection))?.len();
        progress(stats.clone());
    }
    stats.conflicts = store.access(|s| s.sync_conflicts(&connection))?.len();
    Ok(stats)
}
pub fn resolve(
    store: &impl Store,
    dav: &Dav,
    c: &Config,
    workspace: &str,
    device: &str,
    conflict: &Conflict,
    choice: &str,
) -> Result<()> {
    if !["local", "remote", "both"].contains(&choice) {
        return Err(invalid("未知冲突处理方式。"));
    }
    let connection = c.id();
    let catalog = dav.catalog(workspace)?.0;
    let remote_hash = catalog
        .entries
        .get(&conflict.resource_key)
        .map(|h| h.manifest.as_str());
    if remote_hash != conflict.remote_manifest.as_deref() {
        return Err(invalid("远端在确认前又发生变化，请先重新同步刷新冲突。"));
    }
    let local = store.access(|s| s.sync_snapshot(&conflict.resource_key))?;
    let remote = remote_hash
        .map(|h| manifest(store, dav, workspace, h))
        .transpose()?;
    if local
        .as_ref()
        .is_some_and(|s| !c.selected(s.project.as_deref()))
        || remote
            .as_ref()
            .is_some_and(|m| !c.selected(m.snapshot.project.as_deref()))
    {
        return Err(invalid("该资源已不在选定范围内，未处理。"));
    }
    if let (Some(current), Some(old)) = (&local, &conflict.local_manifest)
        && current.content()?
            != store
                .access(|s| s.sync_manifest(old, workspace))?
                .snapshot
                .content()?
    {
        return Err(invalid("本地资料在确认前已改变，请先重新同步刷新冲突。"));
    }
    if let Some(r) = &remote {
        download(store, dav, r)?;
    }
    if choice == "both"
        && !remote
            .as_ref()
            .is_some_and(|r| matches!(r.snapshot.body, Body::Deleted))
    {
        let source = remote
            .as_ref()
            .map(|r| &r.snapshot)
            .ok_or(invalid("远端不存在可复制资料；请选择保留本地。"))?;
        let copy = store.access(|s| {
            s.sync_clone(source, &format!("{}/{}", conflict.id, remote_hash.unwrap()))
        })?;
        let copy_key = copy.key.clone();
        let old = store.access(|s| s.sync_snapshot(&copy_key))?;
        if old.is_none() {
            let digest = store.access(|s| s.sync_archive(workspace, device, copy, vec![]))?;
            let m = store.access(|s| s.sync_manifest(&digest, workspace))?;
            store.access(|s| s.sync_apply("local-conflict-copy", &digest, &m, None, ""))?;
        }
    }
    if choice == "remote" {
        let r = remote
            .as_ref()
            .ok_or(invalid("远端索引记录不存在，不能把未知缺失当作删除。"))?;
        let expected = local.as_ref().map(Snapshot::content).transpose()?;
        store.access(|s| {
            s.sync_apply(
                &connection,
                remote_hash.unwrap(),
                r,
                expected.as_deref(),
                "",
            )
        })?;
        store.access(|s| s.sync_resolved(&connection, &conflict.resource_key))?;
    } else {
        let snapshot = local.ok_or(invalid("本地没有可采用的资料或明确删除记录。"))?;
        // User resolution is the only place that deliberately supersedes an
        // uncertain pending operation; both old immutable manifests stay on disk.
        store.access(|s| s.sync_resolved(&connection, &conflict.resource_key))?;
        let digest = store.access(|s| {
            s.sync_prepare(
                &connection,
                workspace,
                device,
                snapshot,
                remote_hash,
                conflict.local_manifest.as_deref(),
            )
        })?;
        if let Err(e) = push(store, dav, workspace, &connection, &digest, remote_hash) {
            store.access(|s| {
                s.sync_conflict(
                    &connection,
                    &conflict.resource_key,
                    Some(&digest),
                    remote_hash,
                    "冲突处理未确认；双方内容保留，请重试同步。",
                )
            })?;
            return Err(e);
        }
    }
    Ok(())
}
