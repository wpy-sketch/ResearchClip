//! Durable, scoped standard-annotation state. No keys, URLs, SQL or private strokes.
use super::{
    zotero_annotations::{
        AnnotationPosition, AnnotationReadCache, AnnotationSource, ZoteroAnnotation,
        ZoteroAnnotationIdentity,
    },
    zotero_local::valid_item_key,
    zotero_write_model::{AnnotationWriteInput, WRITE_TYPES},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnnotationSyncState {
    Synced,
    LocalDirty,
    RemoteChanged,
    Conflict,
    Failed,
    ReadOnly,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnotationSyncEntry {
    pub format_version: u32,
    pub id: String,
    pub document_id: String,
    pub source: AnnotationSource,
    pub pdf_content_hash: String,
    pub annotation_key: String,
    pub revision: i64,
    pub status: AnnotationSyncState,
    pub base: Option<ZoteroAnnotation>,
    pub remote: Option<ZoteroAnnotation>,
    pub local: Option<AnnotationWriteInput>,
    pub needs_verification: bool,
    pub message: String,
    pub error_code: Option<String>,
    pub local_updated_at: Option<i64>,
    pub last_synced_at: Option<i64>,
    pub updated_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnotationSyncScope {
    pub refresh_interval: u32,
    pub last_refreshed_at: Option<i64>,
    pub last_attempt_at: Option<i64>,
    pub last_synced_at: Option<i64>,
    pub message: String,
}
impl Default for AnnotationSyncScope {
    fn default() -> Self {
        Self {
            refresh_interval: 0,
            last_refreshed_at: None,
            last_attempt_at: None,
            last_synced_at: None,
            message: "自动刷新关闭；本地修改在显式同步前不会写入Zotero。".into(),
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationSyncSnapshot {
    pub entries: Vec<AnnotationSyncEntry>,
    pub scope: AnnotationSyncScope,
}
fn hex(value: &str, n: usize) -> bool {
    value.len() == n
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl AnnotationSyncEntry {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.format_version != 1
            || !hex(&self.id, 32)
            || !hex(&self.document_id, 32)
            || !hex(&self.pdf_content_hash, 64)
            || !valid_item_key(&self.annotation_key)
            || self.revision <= 0
            || self.revision > 9_007_199_254_740_991
            || self.source.validate().is_err()
            || self.message.len() > 4000
            || self.message.contains('\0')
            || self.error_code.as_ref().is_some_and(|v| {
                v.len() > 96 || !v.bytes().all(|b| b.is_ascii_uppercase() || b == b'_')
            })
        {
            return Err("批注同步身份、版本或状态数据无效。");
        }
        for t in [
            Some(self.updated_at),
            self.local_updated_at,
            self.last_synced_at,
        ]
        .into_iter()
        .flatten()
        {
            if !(1..=8_640_000_000_000_000).contains(&t) {
                return Err("批注同步时间无效。");
            }
        }
        for a in [&self.base, &self.remote].into_iter().flatten() {
            if a.identity.source != self.source || a.identity.annotation_key != self.annotation_key
            {
                return Err("批注同步快照越过当前来源。");
            }
            AnnotationReadCache {
                format_version: 1,
                source: self.source.clone(),
                pdf_content_hash: self.pdf_content_hash.clone(),
                fetched_at: 1,
                product_version: None,
                library_version: None,
                annotations: vec![a.clone()],
                truncated: false,
            }
            .validate()
            .map_err(|_| "批注同步快照无效。")?;
        }
        if let Some(local) = &self.local {
            local.validate_draft()?;
            match local {
                AnnotationWriteInput::Create { .. } => {}
                AnnotationWriteInput::Update { annotation_key, .. }
                | AnnotationWriteInput::Delete { annotation_key, .. } => {
                    if annotation_key != &self.annotation_key {
                        return Err("待处理修改指向不同批注。");
                    }
                }
            }
        }
        if self.status == AnnotationSyncState::Synced
            && (self.local.is_some() || self.needs_verification)
            || self.needs_verification && self.local.is_none()
        {
            return Err("批注已同步标记与待处理意图矛盾。");
        }
        Ok(())
    }
    pub fn writable_type(&self) -> bool {
        self.remote.as_ref().or(self.base.as_ref()).is_none_or(|a| {
            WRITE_TYPES.contains(&a.annotation_type.as_str()) && a.position.is_some()
        })
    }
    pub fn desired(&self) -> Result<Option<ZoteroAnnotation>, &'static str> {
        let Some(input) = &self.local else {
            return Ok(self.remote.clone());
        };
        input.validate()?;
        match input {
            AnnotationWriteInput::Delete { .. } => Ok(None),
            AnnotationWriteInput::Create {
                annotation_type,
                color,
                text,
                comment,
                geometry,
            } => Ok(Some(ZoteroAnnotation {
                identity: ZoteroAnnotationIdentity {
                    source: self.source.clone(),
                    annotation_key: self.annotation_key.clone(),
                },
                version: 0,
                annotation_type: annotation_type.clone(),
                color: color.to_lowercase(),
                text: text.clone(),
                comment: comment.clone(),
                page_label: (geometry.page_index + 1).to_string(),
                modified_at: None,
                position: Some(AnnotationPosition {
                    page_index: geometry.page_index,
                    rects: geometry.raw_rectangles()?,
                    next_page_rects: vec![],
                }),
                unsupported_reason: None,
            })),
            AnnotationWriteInput::Update {
                annotation_type,
                color,
                text,
                comment,
                ..
            } => {
                let mut a = self.base.clone().ok_or("原版本快照不可用，请先解决冲突。");
                if let Ok(a) = a.as_mut() {
                    if a.annotation_type != *annotation_type {
                        return Err("不能用同步改变批注类型。");
                    }
                    a.color = color.to_lowercase();
                    a.text = text.clone();
                    a.comment = comment.clone();
                }
                a.map(Some)
            }
        }
    }
    pub fn matches_remote(&self, remote: &Option<ZoteroAnnotation>) -> bool {
        match (self.desired(), remote) {
            (Ok(None), None) => true,
            (Ok(Some(a)), Some(b)) => {
                a.identity == b.identity
                    && a.annotation_type == b.annotation_type
                    && a.color == b.color
                    && a.text == b.text
                    && a.comment == b.comment
                    && a.position == b.position
            }
            _ => false,
        }
    }
    pub fn observe(&mut self, remote: Option<ZoteroAnnotation>, key_read: bool, now: i64) {
        let changed = self.remote != remote;
        self.remote = remote;
        if self.local.is_some() {
            let deletion = matches!(self.local, Some(AnnotationWriteInput::Delete { .. }));
            if self.matches_remote(&self.remote) && (!deletion || key_read) {
                self.accept_remote(now, "已复读确认本地目标与Zotero一致；未重复发送。");
            } else if self.base.as_ref().map(|a| a.version)
                != self.remote.as_ref().map(|a| a.version)
            {
                self.status = AnnotationSyncState::Conflict;
                self.error_code = Some("ZOTERO_WRITE_CONFLICT".into());
                self.message =
                    "两端同时变化或远端已删除；基线、本地输入和远端快照均已保留。".into();
            } else if self.needs_verification {
                self.status = AnnotationSyncState::Failed;
                self.message = "提交结果待复核；不能自动重发。".into();
            }
        } else if changed {
            self.status = if self.writable_type() {
                AnnotationSyncState::RemoteChanged
            } else {
                AnnotationSyncState::ReadOnly
            };
            self.message = "发现Zotero远端变化；尚未提交任何本地修改。".into();
        }
        self.updated_at = now;
    }
    pub fn accept_remote(&mut self, now: i64, message: &str) {
        self.base = self.remote.clone();
        self.local = None;
        self.needs_verification = false;
        self.status = if self.writable_type() {
            AnnotationSyncState::Synced
        } else {
            AnnotationSyncState::ReadOnly
        };
        self.error_code = None;
        self.last_synced_at = Some(now);
        self.updated_at = now;
        self.message = message.into();
    }
    pub fn fail(&mut self, code: &str, message: &str, now: i64) {
        self.status = match code {
            "ZOTERO_WRITE_CONFLICT" | "ZOTERO_WRITE_NOT_FOUND" => AnnotationSyncState::Conflict,
            "ZOTERO_WRITE_FORBIDDEN"
            | "ZOTERO_WRITE_AUTH_REQUIRED"
            | "ZOTERO_WRITE_UNSUPPORTED"
            | "ZOTERO_WRITE_SOURCE_MISMATCH" => AnnotationSyncState::ReadOnly,
            _ => AnnotationSyncState::Failed,
        };
        self.error_code = Some(code.into());
        self.message = message.into();
        self.updated_at = now;
    }
}
