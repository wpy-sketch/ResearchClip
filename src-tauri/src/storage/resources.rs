//! A catalog over canonical legacy tables plus versioned new metadata, not a second asset store.
use super::{
    Storage, StorageError, files,
    library::{check_id, now},
};
use crate::domain::research::*;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
type Result<T> = std::result::Result<T, StorageError>;
const COLUMNS: &str =
    "resource_type,id,title,format_version,revision,created_at,updated_at,is_legacy,data_json";
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSnapshot {
    pub key: ResourceKey,
    pub title: String,
    pub format_version: u32,
    pub revision: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub legacy: bool,
    pub metadata: serde_json::Value,
}
impl ResourceSnapshot {
    pub fn data(&self) -> Result<ResearchData> {
        if self.legacy || self.format_version != RESOURCE_FORMAT_VERSION {
            return Err(StorageError::Invalid(
                "该资源使用原有服务，或元数据版本不受支持。",
            ));
        }
        let data: ResearchData = serde_json::from_value(self.metadata.clone())
            .map_err(|_| StorageError::Invalid("资源元数据损坏，未覆盖原记录。"))?;
        if data.kind() != self.key.kind {
            return Err(StorageError::Invalid("资源类型与元数据不匹配。"));
        }
        data.validate().map_err(StorageError::Invalid)?;
        Ok(data)
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RelationType {
    Related,
    Source,
    Contains,
    Citation,
    Supports,
    DependsOn,
}
impl RelationType {
    fn key(self) -> &'static str {
        match self {
            Self::Related => "related",
            Self::Source => "source",
            Self::Contains => "contains",
            Self::Citation => "citation",
            Self::Supports => "supports",
            Self::DependsOn => "dependsOn",
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum RelationDirection {
    Incoming,
    Outgoing,
    Both,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRelation {
    pub source: ResourceKey,
    pub target: ResourceKey,
    pub relation_type: String,
    pub origin: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceIssue {
    pub code: String,
    pub resource: ResourceKey,
    pub target: Option<ResourceKey>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceAudit {
    pub issues: Vec<ResourceIssue>,
    pub truncated: bool,
    pub scanned_resources: usize,
}
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ResourceSnapshot> {
    let kind: String = r.get(0)?;
    let json: String = r.get(8)?;
    let bad = |message: &str| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::other(message)),
        )
    };
    Ok(ResourceSnapshot {
        key: ResourceKey {
            kind: ResourceKind::parse(&kind).map_err(bad)?,
            id: r.get(1)?,
        },
        title: r.get(2)?,
        format_version: r.get(3)?,
        revision: r.get(4)?,
        created_at: r.get(5)?,
        updated_at: r.get(6)?,
        legacy: r.get(7)?,
        metadata: serde_json::from_str(&json).map_err(|_| bad("资源元数据损坏。"))?,
    })
}
fn get(conn: &Connection, key: &ResourceKey) -> Result<ResourceSnapshot> {
    key.validate().map_err(StorageError::Invalid)?;
    conn.query_row(
        &format!(
            "SELECT {COLUMNS} FROM research_resource_catalog WHERE resource_type=?1 AND id=?2"
        ),
        params![key.kind.key(), key.id],
        row,
    )
    .optional()?
    .ok_or(StorageError::NotFound)
}
fn model(conn: &Connection, kind: ResourceKind, id: &str) -> Result<ResearchData> {
    get(
        conn,
        &ResourceKey {
            kind,
            id: id.into(),
        },
    )?
    .data()
}
fn validate_dependencies(
    conn: &Connection,
    data: &ResearchData,
    except_id: Option<&str>,
) -> Result<()> {
    data.validate().map_err(StorageError::Invalid)?;
    for key in data.dependencies() {
        get(conn, &key)?;
    }
    let hash_matches = |id: &str, hash: &str| -> Result<()> {
        let ResearchData::PdfAttachment(pdf) = model(conn, ResourceKind::PdfAttachment, id)? else {
            return Err(StorageError::Invalid("PDF附件类型无效。"));
        };
        if pdf.content_hash != hash {
            return Err(StorageError::Conflict);
        }
        Ok(())
    };
    match data {
        ResearchData::PdfAttachment(pdf) => {
            let ResearchData::Document(doc) =
                model(conn, ResourceKind::Document, &pdf.document_id)?
            else {
                return Err(StorageError::Invalid("PDF附件须关联PDF文档。"));
            };
            if doc.document_type != DocumentType::Pdf
                || doc.content_hash.as_deref() != Some(&pdf.content_hash)
            {
                return Err(StorageError::Invalid("PDF文档类型或内容摘要不匹配。"));
            }
        }
        ResearchData::PdfLocation(loc) => hash_matches(&loc.attachment_id, &loc.content_hash)?,
        ResearchData::HandwritingLayer(layer) => {
            hash_matches(&layer.attachment_id, &layer.content_hash)?
        }
        ResearchData::Stroke(stroke) => {
            let ResearchData::HandwritingLayer(layer) =
                model(conn, ResourceKind::HandwritingLayer, &stroke.layer_id)?
            else {
                return Err(StorageError::Invalid("笔迹必须属于私有手写层。"));
            };
            if stroke
                .points
                .iter()
                .any(|p| p.x > layer.page_width || p.y > layer.page_height)
            {
                return Err(StorageError::Invalid("笔迹超出所属页面。"));
            }
            let (count,points):(i64,i64)=conn.query_row("SELECT count(*),coalesce(sum(json_array_length(data_json,'$.value.points')),0) FROM research_entities WHERE resource_type='stroke' AND json_extract(data_json,'$.value.layerId')=?1 AND (?2 IS NULL OR id!=?2)",params![stroke.layer_id,except_id],|r|Ok((r.get(0)?,r.get(1)?)))?;
            if count >= 10_000 || points + stroke.points.len() as i64 > 100_000 {
                return Err(StorageError::Invalid("手写层超过笔画或10万采样点预算。"));
            }
        }
        _ => {}
    }
    Ok(())
}
fn title(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 200 || value.chars().any(char::is_control) {
        return Err(StorageError::Invalid("资源标题须为1至200个字符。"));
    }
    Ok(value)
}
fn dependencies(conn: &Connection, key: &ResourceKey, data: &ResearchData) -> Result<()> {
    conn.execute(
        "DELETE FROM research_relations WHERE source_type=?1 AND source_id=?2 AND origin='model'",
        params![key.kind.key(), key.id],
    )?;
    for target in data.dependencies() {
        conn.execute(
            "INSERT OR IGNORE INTO research_relations VALUES(?1,?2,?3,?4,'dependsOn','model')",
            params![key.kind.key(), key.id, target.kind.key(), target.id],
        )?;
    }
    Ok(())
}
impl Storage {
    pub fn research_resource(&self, key: &ResourceKey) -> Result<ResourceSnapshot> {
        let value = get(&self.connection, key)?;
        if !value.legacy {
            value.data()?;
        }
        Ok(value)
    }
    pub fn research_resources(
        &self,
        kind: Option<ResourceKind>,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<ResourceSnapshot>> {
        if limit == 0 || limit > 100 || offset > 1_000_000 {
            return Err(StorageError::Invalid("资源分页超出上限。"));
        }
        Ok(self.connection.prepare(&format!("SELECT {COLUMNS} FROM research_resource_catalog WHERE (?1 IS NULL OR resource_type=?1) ORDER BY resource_type,id LIMIT ?2 OFFSET ?3"))?.query_map(params![kind.map(ResourceKind::key),limit,offset],row)?.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn create_research_resource(
        &mut self,
        name: &str,
        data: &ResearchData,
    ) -> Result<ResourceSnapshot> {
        let name = title(name)?;
        let stamp = now()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_dependencies(&tx, data, None)?;
        let id: String = tx.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;
        let key = ResourceKey {
            kind: data.kind(),
            id,
        };
        let json =
            serde_json::to_string(data).map_err(|_| StorageError::Invalid("资源无法编码。"))?;
        tx.execute(
            "INSERT INTO research_entities VALUES(?1,?2,?3,?4,1,?5,?6,?6)",
            params![
                key.kind.key(),
                key.id,
                name,
                RESOURCE_FORMAT_VERSION,
                json,
                stamp
            ],
        )?;
        dependencies(&tx, &key, data)?;
        tx.commit()?;
        self.research_resource(&key)
    }
    pub fn update_research_resource(
        &mut self,
        key: &ResourceKey,
        expected: i64,
        name: &str,
        data: &ResearchData,
    ) -> Result<ResourceSnapshot> {
        key.validate().map_err(StorageError::Invalid)?;
        let name = title(name)?;
        if data.kind() != key.kind {
            return Err(StorageError::Invalid("资源不能改变类型。"));
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let old = get(&tx, key)?;
        if old.revision != Some(expected) {
            return Err(StorageError::Conflict);
        }
        let previous = old.data()?;
        match (&previous, data) {
            (ResearchData::PdfAttachment(a), ResearchData::PdfAttachment(b))
                if a.content_hash != b.content_hash
                    || a.document_id != b.document_id
                    || a.byte_size != b.byte_size =>
            {
                return Err(StorageError::Invalid("不可变PDF附件变更须登记新版本资源。"));
            }
            (ResearchData::Document(a), ResearchData::Document(b))
                if a.document_type != b.document_type
                    || (a.document_type == DocumentType::Pdf
                        && a.content_hash != b.content_hash) =>
            {
                return Err(StorageError::Invalid("不可改变文档类型或已有PDF内容版本。"));
            }
            (ResearchData::HandwritingLayer(a), ResearchData::HandwritingLayer(b))
                if a.attachment_id != b.attachment_id
                    || a.content_hash != b.content_hash
                    || a.page_index != b.page_index
                    || a.page_width != b.page_width
                    || a.page_height != b.page_height =>
            {
                return Err(StorageError::Invalid("私有手写层的页面绑定不可更改。"));
            }
            _ => {}
        }
        validate_dependencies(&tx, data, Some(&key.id))?;
        let json =
            serde_json::to_string(data).map_err(|_| StorageError::Invalid("资源无法编码。"))?;
        if tx.execute("UPDATE research_entities SET title=?1,data_json=?2,revision=revision+1,updated_at=?3 WHERE resource_type=?4 AND id=?5 AND revision=?6",params![name,json,now()?,key.kind.key(),key.id,expected])?!=1{return Err(StorageError::Conflict);}
        dependencies(&tx, key, data)?;
        tx.commit()?;
        self.research_resource(key)
    }
    pub fn relate_research_resources(
        &mut self,
        source: &ResourceKey,
        target: &ResourceKey,
        relation: RelationType,
    ) -> Result<()> {
        source.validate().map_err(StorageError::Invalid)?;
        target.validate().map_err(StorageError::Invalid)?;
        if source == target {
            return Err(StorageError::Invalid("不接受资源自关联。"));
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        get(&tx, source)?;
        get(&tx, target)?;
        tx.execute(
            "INSERT OR IGNORE INTO research_relations VALUES(?1,?2,?3,?4,?5,'manual')",
            params![
                source.kind.key(),
                source.id,
                target.kind.key(),
                target.id,
                relation.key()
            ],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn research_relations(
        &self,
        key: &ResourceKey,
        direction: RelationDirection,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<ResourceRelation>> {
        key.validate().map_err(StorageError::Invalid)?;
        if limit == 0 || limit > 200 || offset > 1_000_000 {
            return Err(StorageError::Invalid("关联分页超出上限。"));
        }
        let incoming = !matches!(direction, RelationDirection::Outgoing);
        let outgoing = !matches!(direction, RelationDirection::Incoming);
        let mut statement=self.connection.prepare("SELECT source_type,source_id,target_type,target_id,relation_type,origin FROM research_resource_edges WHERE (?3 AND source_type=?1 AND source_id=?2) OR (?4 AND target_type=?1 AND target_id=?2) ORDER BY source_type,source_id,target_type,target_id,relation_type,origin LIMIT ?5 OFFSET ?6")?;
        let raw = statement
            .query_map(
                params![key.kind.key(), key.id, outgoing, incoming, limit, offset],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                    ))
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        raw.into_iter()
            .map(|(a, b, c, d, e, f)| {
                Ok(ResourceRelation {
                    source: ResourceKey {
                        kind: ResourceKind::parse(&a).map_err(StorageError::Invalid)?,
                        id: b,
                    },
                    target: ResourceKey {
                        kind: ResourceKind::parse(&c).map_err(StorageError::Invalid)?,
                        id: d,
                    },
                    relation_type: e,
                    origin: f,
                })
            })
            .collect()
    }
    pub fn bind_zotero_resource(&mut self, external: &ZoteroKey, key: &ResourceKey) -> Result<()> {
        external.validate().map_err(StorageError::Invalid)?;
        key.validate().map_err(StorageError::Invalid)?;
        if !matches!(
            key.kind,
            ResourceKind::Literature | ResourceKind::PdfAttachment | ResourceKind::Annotation
        ) {
            return Err(StorageError::Invalid("该资源类型不可绑定Zotero item key。"));
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        get(&tx, key)?;
        let prior:Option<(String,String)>=tx.query_row("SELECT resource_type,resource_id FROM research_zotero_keys WHERE instance_id=?1 AND library_id=?2 AND item_key=?3",params![external.instance_id,external.library_id,external.item_key],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        if let Some((kind, id)) = prior {
            if kind != key.kind.key() || id != key.id {
                return Err(StorageError::Conflict);
            }
        } else {
            tx.execute(
                "INSERT INTO research_zotero_keys VALUES(?1,?2,?3,?4,?5)",
                params![
                    external.instance_id,
                    external.library_id,
                    external.item_key,
                    key.kind.key(),
                    key.id
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn zotero_resource(&self, external: &ZoteroKey) -> Result<Option<ResourceKey>> {
        external.validate().map_err(StorageError::Invalid)?;
        let value:Option<(String,String)>=self.connection.query_row("SELECT resource_type,resource_id FROM research_zotero_keys WHERE instance_id=?1 AND library_id=?2 AND item_key=?3",params![external.instance_id,external.library_id,external.item_key],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        value
            .map(|(kind, id)| {
                Ok(ResourceKey {
                    kind: ResourceKind::parse(&kind).map_err(StorageError::Invalid)?,
                    id,
                })
            })
            .transpose()
    }
    /// Metadata-only relink after a caller-selected destination. It never moves or overwrites a file.
    /// Callers must close the old document session first; native grants are not transferable.
    pub fn relink_markdown_resource(
        &mut self,
        id: &str,
        expected_hash: &str,
        selected: &Path,
    ) -> Result<()> {
        check_id(id)?;
        if !hash_valid(expected_hash) {
            return Err(StorageError::Invalid("文档基准摘要无效。"));
        }
        let (path, hash, size) = crate::services::markdown_document::file_fingerprint(selected)
            .map_err(|_| StorageError::Invalid("所选Markdown不可读取，原登记保留。"))?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (old,baseline):(String,String)=tx.query_row("SELECT d.path,f.content_hash FROM markdown_documents d JOIN research_document_fingerprints f ON f.document_id=d.id WHERE d.id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?.ok_or(StorageError::Invalid("旧文档尚无已验证摘要，请先成功打开或保存，不按文件名猜测重连。"))?;
        if baseline != expected_hash || hash != baseline {
            return Err(StorageError::Conflict);
        }
        match std::fs::symlink_metadata(&old) {
            Ok(_) => {
                let (_, current, _) =
                    crate::services::markdown_document::file_fingerprint(Path::new(&old))
                        .map_err(|_| StorageError::Invalid("旧文档状态不可校验。"))?;
                if current != baseline {
                    return Err(StorageError::Conflict);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let path = path
            .to_str()
            .ok_or(StorageError::Invalid("文档路径编码无效。"))?;
        let name = Path::new(path)
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or(StorageError::Invalid("文档名称无效。"))?;
        tx.execute(
            "UPDATE markdown_documents SET path=?1,name=?2,updated_at=?3 WHERE id=?4",
            params![path, name, now()?, id],
        )?;
        tx.execute(
            "UPDATE research_document_fingerprints SET byte_size=?1 WHERE document_id=?2",
            params![size, id],
        )?;
        // A path authorization must not silently follow a move.
        tx.execute(
            "DELETE FROM sync_document_grants WHERE document_id=?1 AND authorized_path!=?2",
            params![id, path],
        )?;
        tx.commit()?;
        Ok(())
    }
    /// Bounded, read-only diagnostics. No orphan is automatically deleted or repaired.
    pub fn audit_research_resources(&self, budget: u32) -> Result<ResourceAudit> {
        if budget == 0 || budget > 1000 {
            return Err(StorageError::Invalid("资源检查预算须为1至1000。"));
        }
        let resources = self.research_resources(None, 0, budget.min(100))?;
        let total: i64 = self.connection.query_row(
            "SELECT count(*) FROM research_resource_catalog",
            [],
            |r| r.get(0),
        )?;
        let mut all = resources;
        let mut offset = all.len() as u32;
        while offset < budget && (offset as i64) < total {
            let more = self.research_resources(None, offset, (budget - offset).min(100))?;
            offset += more.len() as u32;
            if more.is_empty() {
                break;
            }
            all.extend(more);
        }
        all.truncate(budget as usize);
        let mut report = ResourceAudit {
            issues: Vec::new(),
            truncated: total > budget as i64,
            scanned_resources: all.len(),
        };
        let mut add = |code: &str, key: &ResourceKey, target: Option<ResourceKey>| {
            report.issues.push(ResourceIssue {
                code: code.into(),
                resource: key.clone(),
                target,
            })
        };
        for resource in &all {
            if resource.format_version != RESOURCE_FORMAT_VERSION {
                add("unsupportedVersion", &resource.key, None);
                continue;
            }
            if !resource.legacy && resource.data().is_err() {
                add("invalidMetadata", &resource.key, None);
                continue;
            }
            if !resource.legacy {
                let data = resource.data()?;
                if validate_dependencies(&self.connection, &data, Some(&resource.key.id)).is_err() {
                    add("invalidModelBinding", &resource.key, None);
                }
            }
            let count:i64=self.connection.query_row("SELECT count(*) FROM research_resource_edges WHERE (source_type=?1 AND source_id=?2) OR (target_type=?1 AND target_id=?2)",params![resource.key.kind.key(),resource.key.id],|r|r.get(0))?;
            if count == 0 {
                add("unassociated", &resource.key, None);
            }
            let path = if resource.legacy && resource.key.kind == ResourceKind::Screenshot {
                let rel = resource.metadata["originalImagePath"]
                    .as_str()
                    .unwrap_or("");
                match files::image_path(&self.data_dir, rel) {
                    Ok(p) => Some(p),
                    Err(_) => {
                        add("missingOrUnsafeFile", &resource.key, None);
                        None
                    }
                }
            } else if resource.legacy && resource.key.kind == ResourceKind::Document {
                let fingerprint:Option<String>=self.connection.query_row("SELECT content_hash FROM research_document_fingerprints WHERE document_id=?1",[&resource.key.id],|r|r.get(0)).optional()?;
                if fingerprint.is_none() {
                    add("missingFingerprint", &resource.key, None);
                }
                Some(PathBuf::from(
                    resource.metadata["path"].as_str().unwrap_or(""),
                ))
            } else if let Ok(ResearchData::Document(doc)) = resource.data() {
                if doc.document_type == DocumentType::Pdf {
                    if let Some(relative) = doc.relative_path {
                        let mut p = self.data_dir.clone();
                        let mut safe = true;
                        for component in relative.split('/') {
                            p.push(component);
                            if files::ordinary(&p).is_err() {
                                safe = false;
                                break;
                            }
                        }
                        if safe {
                            Some(p)
                        } else {
                            add("missingOrUnsafeFile", &resource.key, None);
                            None
                        }
                    } else {
                        add("fileNotRegistered", &resource.key, None);
                        None
                    }
                } else {
                    add("fileGrantRequired", &resource.key, None);
                    None
                }
            } else {
                None
            };
            if let Some(path) = path
                && (!path.is_file() || files::ordinary(&path).is_err())
            {
                add("missingOrUnsafeFile", &resource.key, None);
            }
        }
        let mut q=self.connection.prepare("SELECT source_type,source_id,target_type,target_id FROM research_resource_edges e WHERE NOT EXISTS(SELECT 1 FROM research_resource_catalog r WHERE r.resource_type=e.source_type AND r.id=e.source_id) OR NOT EXISTS(SELECT 1 FROM research_resource_catalog r WHERE r.resource_type=e.target_type AND r.id=e.target_id) LIMIT ?1")?;
        let missing = q
            .query_map([budget + 1], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if missing.len() > budget as usize {
            report.truncated = true;
        }
        for (a, b, c, d) in missing.into_iter().take(budget as usize) {
            report.issues.push(ResourceIssue {
                code: "missingRelationEndpoint".into(),
                resource: ResourceKey {
                    kind: ResourceKind::parse(&a).map_err(StorageError::Invalid)?,
                    id: b,
                },
                target: Some(ResourceKey {
                    kind: ResourceKind::parse(&c).map_err(StorageError::Invalid)?,
                    id: d,
                }),
            });
        }
        let mut q=self.connection.prepare("SELECT resource_type,resource_id FROM research_zotero_keys k WHERE NOT EXISTS(SELECT 1 FROM research_resource_catalog r WHERE r.resource_type=k.resource_type AND r.id=k.resource_id) LIMIT ?1")?;
        let missing = q
            .query_map([budget + 1], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if missing.len() > budget as usize {
            report.truncated = true;
        }
        for (kind, id) in missing.into_iter().take(budget as usize) {
            report.issues.push(ResourceIssue {
                code: "missingExternalKeyTarget".into(),
                resource: ResourceKey {
                    kind: ResourceKind::parse(&kind).map_err(StorageError::Invalid)?,
                    id,
                },
                target: None,
            });
        }
        let mut q = self.connection.prepare("SELECT source_id,target_id FROM link_index l WHERE anchor_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM annotation_anchors a WHERE a.anchor_id=l.anchor_id AND a.screenshot_id=l.target_id) LIMIT ?1")?;
        let broken = q
            .query_map([budget + 1], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if broken.len() > budget as usize {
            report.truncated = true;
        }
        for (source, target) in broken.into_iter().take(budget as usize) {
            report.issues.push(ResourceIssue {
                code: "missingAnnotationAnchor".into(),
                resource: ResourceKey {
                    kind: ResourceKind::Document,
                    id: source,
                },
                target: Some(ResourceKey {
                    kind: ResourceKind::Screenshot,
                    id: target,
                }),
            });
        }
        Ok(report)
    }
}
/// Only used to construct historical schemas in disposable unit-test databases.
#[cfg(test)]
pub(crate) fn discard_schema12_test_fixture(conn: &Connection) {
    // Only constructs old schemas in isolated unit fixtures; never compiled into the app.
    assert_eq!(
        conn.query_row("SELECT count(*) FROM markdown_revisions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0,
        "legacy fixture cannot discard document history"
    );
    conn.execute_batch("DROP TRIGGER markdown_revision_no_update; DROP TRIGGER markdown_revision_no_delete; DROP TABLE markdown_revisions; DROP TABLE document_organization; DROP TABLE document_folders;").unwrap();

    assert_eq!(
        conn.query_row("SELECT count(*) FROM research_entities", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0,
        "historical fixture must not contain new research entities"
    );
    conn.execute_batch("DROP TRIGGER research_project_in_use; DROP TRIGGER research_document_in_use; DROP TRIGGER research_literature_in_use; DROP TRIGGER research_entity_in_use; DROP TRIGGER research_entity_no_legacy_document; DROP TRIGGER research_legacy_document_no_entity; DROP TRIGGER research_relation_valid; DROP TRIGGER research_zotero_valid; DROP VIEW research_resource_edges; DROP VIEW research_resource_catalog; DROP TABLE research_document_fingerprints; DROP TABLE research_zotero_keys; DROP TABLE research_relations; DROP TABLE research_entities;").unwrap();
}
#[cfg(test)]
mod tests;
