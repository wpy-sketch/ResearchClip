//! Typed durable standard-annotation intent and conflict sidecars, using existing IDs.
use super::{
    Storage, StorageError,
    library::{check_id, now},
    revision_meta::RevisionMetadata,
};
use crate::services::{
    zotero_annotations::{AnnotationReadCache, AnnotationSource},
    zotero_sync_model::{
        AnnotationSyncEntry, AnnotationSyncScope, AnnotationSyncSnapshot, AnnotationSyncState,
    },
    zotero_write_model::{AnnotationWriteInput, new_foreign_annotation_key},
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
type Result<T> = std::result::Result<T, StorageError>;
const BUDGET: i64 = 64 * 1024 * 1024;
fn invalid(_: impl std::fmt::Display) -> StorageError {
    StorageError::Invalid("批注同步记录损坏或未来格式，未重置数据。")
}
fn state_text(state: AnnotationSyncState) -> Result<String> {
    serde_json::to_value(state)
        .map_err(invalid)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid("state"))
}
impl Storage {
    pub fn annotation_sync_snapshot(
        &self,
        document_id: &str,
        source: &AnnotationSource,
    ) -> Result<AnnotationSyncSnapshot> {
        self.annotation_source(document_id, source)?;
        let record = self.pdf_record(document_id)?;
        let mut query=self.connection.prepare("SELECT id,revision,status,entry_json FROM zotero_annotation_sync WHERE document_id=?1 AND instance_id=?2 AND library_type=?3 AND library_id=?4 AND item_key=?5 AND attachment_key=?6 AND content_hash=?7 ORDER BY id LIMIT 1001")?;
        let rows = query.query_map(
            params![
                document_id,
                source.instance_id,
                source.library_type,
                source.library_id,
                source.item_key,
                source.attachment_key,
                record.content_hash
            ],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            },
        )?;
        let mut entries = vec![];
        for row in rows {
            let (id, revision, status, json) = row?;
            if json.len() > 1024 * 1024 {
                return Err(invalid("budget"));
            }
            let e: AnnotationSyncEntry = serde_json::from_str(&json).map_err(invalid)?;
            e.validate().map_err(invalid)?;
            if e.id != id
                || e.revision != revision
                || state_text(e.status)? != status
                || e.document_id != document_id
                || e.source != *source
                || e.pdf_content_hash != record.content_hash
            {
                return Err(StorageError::Conflict);
            }
            entries.push(e);
        }
        if entries.len() > 1000 {
            return Err(StorageError::Invalid(
                "当前来源超过1000条同步记录，未丢弃记录。",
            ));
        }
        let raw:Option<String>=self.connection.query_row("SELECT scope_json FROM zotero_annotation_sync_scopes WHERE document_id=?1 AND instance_id=?2 AND library_type=?3 AND library_id=?4 AND item_key=?5 AND attachment_key=?6 AND content_hash=?7",params![document_id,source.instance_id,source.library_type,source.library_id,source.item_key,source.attachment_key,record.content_hash],|r|r.get(0)).optional()?;
        let scope: AnnotationSyncScope = raw
            .map(|s| {
                if s.len() > 16384 {
                    return Err(invalid("scope"));
                }
                serde_json::from_str(&s).map_err(invalid)
            })
            .transpose()?
            .unwrap_or_default();
        validate_scope(&scope)?;
        Ok(AnnotationSyncSnapshot { entries, scope })
    }
    pub fn annotation_sync_entry(
        &self,
        document_id: &str,
        source: &AnnotationSource,
        id: &str,
    ) -> Result<AnnotationSyncEntry> {
        check_id(id)?;
        self.annotation_sync_snapshot(document_id, source)?
            .entries
            .into_iter()
            .find(|e| e.id == id)
            .ok_or(StorageError::NotFound)
    }
    pub fn save_annotation_sync_scope(
        &mut self,
        document_id: &str,
        source: &AnnotationSource,
        scope: &AnnotationSyncScope,
    ) -> Result<()> {
        self.annotation_source(document_id, source)?;
        validate_scope(scope)?;
        let hash = self.pdf_record(document_id)?.content_hash;
        let json = serde_json::to_string(scope).map_err(invalid)?;
        self.connection.execute("INSERT INTO zotero_annotation_sync_scopes(document_id,instance_id,library_type,library_id,item_key,attachment_key,content_hash,scope_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(document_id,instance_id,library_type,library_id,item_key,attachment_key,content_hash) DO UPDATE SET scope_json=excluded.scope_json",params![document_id,source.instance_id,source.library_type,source.library_id,source.item_key,source.attachment_key,hash,json])?;
        Ok(())
    }
    pub fn save_annotation_sync_entry(
        &mut self,
        mut entry: AnnotationSyncEntry,
        expected: i64,
        archive: bool,
    ) -> Result<AnnotationSyncEntry> {
        self.annotation_source(&entry.document_id, &entry.source)?;
        entry.validate().map_err(invalid)?;
        if self.pdf_record(&entry.document_id)?.content_hash != entry.pdf_content_hash {
            return Err(StorageError::Conflict);
        }
        let existing: Option<(i64, String)> = self
            .connection
            .query_row(
                "SELECT revision,entry_json FROM zotero_annotation_sync WHERE id=?1",
                [&entry.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if existing.as_ref().map_or(0, |e| e.0) != expected {
            return Err(StorageError::Conflict);
        }
        if let Some((_, json)) = &existing {
            let old: AnnotationSyncEntry = serde_json::from_str(json).map_err(invalid)?;
            old.validate().map_err(invalid)?;
            if old.document_id != entry.document_id
                || old.source != entry.source
                || old.pdf_content_hash != entry.pdf_content_hash
                || old.annotation_key != entry.annotation_key
            {
                return Err(StorageError::Conflict);
            }
        }
        entry.revision = expected.checked_add(1).ok_or(StorageError::Conflict)?;
        let json = serde_json::to_string(&entry).map_err(invalid)?;
        if json.len() > 1024 * 1024 {
            return Err(StorageError::Invalid(
                "单条批注同步记录超过1MiB，旧记录保留。",
            ));
        }
        let history_id = self.new_id()?;
        let status = state_text(entry.status)?;
        let timestamp = now()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT revision FROM zotero_annotation_sync WHERE id=?1",
                [&entry.id],
                |r| r.get(0),
            )
            .optional()?;
        if current.unwrap_or(0) != expected {
            return Err(StorageError::Conflict);
        }
        let bytes: i64 = tx.query_row(
            "SELECT coalesce(sum(length(cast(entry_json AS BLOB))),0) FROM zotero_annotation_sync",
            [],
            |r| r.get(0),
        )?;
        let old_len = existing.as_ref().map_or(0, |e| e.1.len() as i64);
        if bytes - old_len + json.len() as i64 > BUDGET {
            return Err(StorageError::Invalid(
                "批注待处理队列达到64MiB保护预算，旧记录和当前输入须保留。",
            ));
        }
        if archive && let Some((version, old)) = &existing {
            let history_bytes:i64=tx.query_row("SELECT coalesce(sum(length(cast(snapshot_json AS BLOB))),0) FROM zotero_annotation_sync_revisions",[],|r|r.get(0))?;
            if history_bytes + old.len() as i64 > BUDGET {
                return Err(StorageError::Invalid(
                    "批注处理修订达到64MiB保护预算，未删除任何历史。",
                ));
            }
            let parent:Option<String>=tx.query_row("SELECT revision_id FROM zotero_annotation_sync_revisions WHERE change_id=?1 ORDER BY version_number DESC LIMIT 1",[&entry.id],|r|r.get(0)).optional()?;
            tx.execute("INSERT INTO zotero_annotation_sync_revisions(revision_id,change_id,version_number,parent_revision_id,created_at,description,snapshot_json) VALUES(?1,?2,?3,?4,?5,'保留提交、失败或冲突解决前的完整状态',?6)",params![history_id,entry.id,version,parent,timestamp,old])?;
        }
        tx.execute("INSERT INTO zotero_annotation_sync(id,document_id,instance_id,library_type,library_id,item_key,attachment_key,content_hash,annotation_key,revision,status,entry_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,status=excluded.status,entry_json=excluded.entry_json",params![entry.id,entry.document_id,entry.source.instance_id,entry.source.library_type,entry.source.library_id,entry.source.item_key,entry.source.attachment_key,entry.pdf_content_hash,entry.annotation_key,entry.revision,status,json])?;
        tx.commit()?;
        Ok(entry)
    }
    pub fn stage_annotation_change(
        &mut self,
        document_id: &str,
        source: &AnnotationSource,
        id: Option<&str>,
        revision: Option<i64>,
        input: AnnotationWriteInput,
    ) -> Result<AnnotationSyncEntry> {
        input.validate_draft().map_err(StorageError::Invalid)?;
        let snapshot = self.annotation_sync_snapshot(document_id, source)?;
        let record = self.pdf_record(document_id)?;
        let timestamp = now()?;
        let key = match &input {
            AnnotationWriteInput::Create { .. } => None,
            AnnotationWriteInput::Update { annotation_key, .. }
            | AnnotationWriteInput::Delete { annotation_key, .. } => Some(annotation_key.as_str()),
        };
        let mut entry = if let Some(id) = id {
            let e = snapshot
                .entries
                .iter()
                .find(|e| e.id == id)
                .cloned()
                .ok_or(StorageError::NotFound)?;
            if revision != Some(e.revision) {
                return Err(StorageError::Conflict);
            }
            if e.needs_verification {
                return Err(StorageError::Invalid("上次写入仍待复核，不能覆盖该意图。"));
            }
            e
        } else if let Some(e) = snapshot
            .entries
            .iter()
            .find(|e| Some(e.annotation_key.as_str()) == key)
        {
            if e.local.as_ref().is_some_and(|old| old != &input) || e.needs_verification {
                return Err(StorageError::Conflict);
            }
            e.clone()
        } else {
            if snapshot.entries.len() >= 1000 {
                return Err(StorageError::Invalid(
                    "当前来源同步记录达到1000条，未删除旧记录。",
                ));
            }
            let remote = if let Some(key) = key {
                self.annotation_cache(document_id, source)?.and_then(|c| {
                    c.annotations
                        .into_iter()
                        .find(|a| a.identity.annotation_key == key)
                })
            } else {
                None
            };
            if key.is_some() && remote.is_none() {
                return Err(StorageError::Invalid("请先读取要修改的当前来源标准批注。"));
            }
            AnnotationSyncEntry {
                format_version: 1,
                id: self.new_id()?,
                document_id: document_id.into(),
                source: source.clone(),
                pdf_content_hash: record.content_hash,
                annotation_key: match key {
                    Some(key) => key.into(),
                    None => new_foreign_annotation_key().map_err(StorageError::Invalid)?,
                },
                revision: 1,
                status: AnnotationSyncState::LocalDirty,
                base: remote.clone(),
                remote,
                local: None,
                needs_verification: false,
                message: String::new(),
                error_code: None,
                local_updated_at: None,
                last_synced_at: None,
                updated_at: timestamp,
            }
        };
        if !entry.writable_type() {
            return Err(StorageError::Invalid(
                "本版不修改未验证类型或不支持的位置。",
            ));
        }
        if key.is_some_and(|key| key != entry.annotation_key)
            || matches!(input, AnnotationWriteInput::Create { .. }) && entry.base.is_some()
        {
            return Err(StorageError::Conflict);
        }
        let expected = if snapshot.entries.iter().any(|e| e.id == entry.id) {
            entry.revision
        } else {
            0
        };
        if entry.local.is_none() && !matches!(input, AnnotationWriteInput::Create { .. }) {
            entry.base = entry.remote.clone();
        }
        let preserve_failure = matches!(
            entry.status,
            AnnotationSyncState::Conflict
                | AnnotationSyncState::Failed
                | AnnotationSyncState::ReadOnly
        ) && entry.local.as_ref().is_some_and(|old| old != &input);
        entry.local = Some(input);
        entry.local_updated_at = Some(timestamp);
        entry.updated_at = timestamp;
        entry.error_code = None;
        entry.message = "本地待处理修改已落盘，尚未写入Zotero。".into();
        entry.status = AnnotationSyncState::LocalDirty;
        if let Some(
            AnnotationWriteInput::Update {
                expected_version, ..
            }
            | AnnotationWriteInput::Delete {
                expected_version, ..
            },
        ) = &entry.local
            && entry.base.as_ref().map(|a| a.version) != Some(*expected_version)
        {
            entry.status = AnnotationSyncState::Conflict;
            entry.message = "草稿基线版本已变化；输入保留，请核对双方后解决。".into();
        }
        self.save_annotation_sync_entry(entry, expected, preserve_failure)
    }
    pub fn observe_annotation_sync(
        &mut self,
        document_id: &str,
        cache: &AnnotationReadCache,
    ) -> Result<AnnotationSyncSnapshot> {
        self.annotation_source(document_id, &cache.source)?;
        cache.validate().map_err(invalid)?;
        if self.pdf_record(document_id)?.content_hash != cache.pdf_content_hash {
            return Err(StorageError::Conflict);
        }
        let snapshot = self.annotation_sync_snapshot(document_id, &cache.source)?;
        let timestamp = now()?;
        let mut existing = snapshot.entries;
        for remote in &cache.annotations {
            if let Some(e) = existing
                .iter_mut()
                .find(|e| e.annotation_key == remote.identity.annotation_key)
            {
                let previous = e.clone();
                e.observe(Some(remote.clone()), false, timestamp);
                let mut comparable = e.clone();
                comparable.updated_at = previous.updated_at;
                if comparable != previous {
                    let saved =
                        self.save_annotation_sync_entry(e.clone(), previous.revision, true)?;
                    *e = saved;
                }
            } else {
                let e = AnnotationSyncEntry {
                    format_version: 1,
                    id: self.new_id()?,
                    document_id: document_id.into(),
                    source: cache.source.clone(),
                    pdf_content_hash: cache.pdf_content_hash.clone(),
                    annotation_key: remote.identity.annotation_key.clone(),
                    revision: 1,
                    status: AnnotationSyncState::RemoteChanged,
                    base: None,
                    remote: Some(remote.clone()),
                    local: None,
                    needs_verification: false,
                    message: "首次读取到Zotero标准批注。".into(),
                    error_code: None,
                    local_updated_at: None,
                    last_synced_at: None,
                    updated_at: timestamp,
                };
                let mut e = e;
                if !e.writable_type() {
                    e.status = AnnotationSyncState::ReadOnly;
                }
                self.save_annotation_sync_entry(e, 0, false)?;
            }
        }
        if !cache.truncated {
            for mut e in existing.into_iter().filter(|e| {
                e.remote.is_some()
                    && !cache
                        .annotations
                        .iter()
                        .any(|a| a.identity.annotation_key == e.annotation_key)
            }) {
                let rev = e.revision;
                e.observe(None, false, timestamp);
                self.save_annotation_sync_entry(e, rev, true)?;
            }
        }
        let mut scope = snapshot.scope;
        scope.last_refreshed_at = Some(cache.fetched_at);
        scope.message = if cache.truncated {
            "读取列表达到保护上限，不能据此推断未出现的批注已删除。"
        } else {
            "已读取当前来源变化；未自动提交本地修改。"
        }
        .into();
        self.save_annotation_sync_scope(document_id, &cache.source, &scope)?;
        self.annotation_sync_snapshot(document_id, &cache.source)
    }
    pub fn annotation_sync_history(
        &self,
        document_id: &str,
        source: &AnnotationSource,
        id: &str,
    ) -> Result<Vec<RevisionMetadata>> {
        self.annotation_sync_entry(document_id, source, id)?;
        let mut q=self.connection.prepare("SELECT revision_id,version_number,parent_revision_id,created_at,description FROM zotero_annotation_sync_revisions WHERE change_id=?1 ORDER BY version_number DESC LIMIT 1000")?;
        Ok(q.query_map([id], |r| {
            Ok(RevisionMetadata {
                revision_id: r.get(0)?,
                version_number: r.get(1)?,
                parent_revision_id: r.get(2)?,
                created_at: r.get(3)?,
                description: r.get(4)?,
                schema_version: 1,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn annotation_sync_revision(
        &self,
        document_id: &str,
        source: &AnnotationSource,
        id: &str,
        revision_id: &str,
    ) -> Result<AnnotationSyncEntry> {
        self.annotation_sync_entry(document_id, source, id)?;
        check_id(revision_id)?;
        let raw:String=self.connection.query_row("SELECT snapshot_json FROM zotero_annotation_sync_revisions WHERE change_id=?1 AND revision_id=?2",params![id,revision_id],|r|r.get(0)).optional()?.ok_or(StorageError::NotFound)?;
        if raw.len() > 1024 * 1024 {
            return Err(invalid("history"));
        }
        let e: AnnotationSyncEntry = serde_json::from_str(&raw).map_err(invalid)?;
        e.validate().map_err(invalid)?;
        if e.id != id || e.document_id != document_id || e.source != *source {
            return Err(StorageError::Conflict);
        }
        Ok(e)
    }
}
fn validate_scope(s: &AnnotationSyncScope) -> Result<()> {
    if ![0, 30, 60, 300].contains(&s.refresh_interval)
        || s.message.len() > 4000
        || s.message.contains('\0')
        || [s.last_refreshed_at, s.last_attempt_at, s.last_synced_at]
            .into_iter()
            .flatten()
            .any(|t| !(1..=8_640_000_000_000_000).contains(&t))
    {
        return Err(invalid("scope"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "zotero_sync_tests.rs"]
mod tests;
