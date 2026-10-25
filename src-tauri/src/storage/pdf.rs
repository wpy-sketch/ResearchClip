//! Reuses the research resource catalog; no second PDF identity or document store.
use super::{Storage, StorageError, files, library::now, resources::insert_resource};
use crate::domain::research::*;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
type Result<T> = std::result::Result<T, StorageError>;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PdfReadingLocation {
    pub page_index: u32,
    pub rotation: u32,
    pub zoom: f64,
    pub zoom_mode: String,
    pub spread: bool,
    pub offset_x: f64,
    pub offset_y: f64,
}
impl PdfReadingLocation {
    pub fn validate(&self) -> Result<()> {
        if self.page_index >= 10_000
            || ![0, 90, 180, 270].contains(&self.rotation)
            || !self.zoom.is_finite()
            || !(0.25..=4.0).contains(&self.zoom)
            || !["custom", "width", "page"].contains(&self.zoom_mode.as_str())
            || !self.offset_x.is_finite()
            || !(0.0..=1.0).contains(&self.offset_x)
            || !self.offset_y.is_finite()
            || !(0.0..=1.0).contains(&self.offset_y)
        {
            return Err(StorageError::Invalid("PDF阅读位置或缩放无效。"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfReadingState {
    pub revision: i64,
    pub location: PdfReadingLocation,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfRecord {
    pub document_id: String,
    pub attachment_id: String,
    pub title: String,
    pub content_hash: String,
    pub byte_size: u64,
    pub reading: Option<PdfReadingState>,
}
impl Storage {
    pub fn pdf_directory(&self) -> Result<PathBuf> {
        let path = self.data_dir.join("pdfs");
        files::ordinary(&self.data_dir)?;
        files::directory(&path)?;
        Ok(path)
    }
    pub fn register_pdf(&mut self, name: &str, hash: &str, bytes: u64) -> Result<PdfRecord> {
        if !hash_valid(hash) || bytes == 0 || bytes > 128 * 1024 * 1024 {
            return Err(StorageError::Invalid("PDF摘要或大小无效。"));
        }
        let relative = format!("pdfs/{hash}.pdf");
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing:Option<String>=tx.query_row("SELECT id FROM research_entities WHERE resource_type='document' AND json_extract(data_json,'$.value.documentType')='pdf' AND json_extract(data_json,'$.value.contentHash')=?1 ORDER BY id LIMIT 1",[hash],|r|r.get(0)).optional()?;
        let id = if let Some(id) = existing {
            let count:i64=tx.query_row("SELECT count(*) FROM research_entities WHERE resource_type='pdfAttachment' AND json_extract(data_json,'$.value.documentId')=?1",[&id],|r|r.get(0))?;
            if count == 0 {
                insert_resource(
                    &tx,
                    name,
                    &ResearchData::PdfAttachment(PdfAttachment {
                        document_id: id.clone(),
                        reference_id: None,
                        content_hash: hash.into(),
                        byte_size: bytes,
                    }),
                )?;
            }
            id
        } else {
            let document = insert_resource(
                &tx,
                name,
                &ResearchData::Document(DocumentResource {
                    document_type: DocumentType::Pdf,
                    project_id: None,
                    relative_path: Some(relative),
                    code_workspace_id: None,
                    content_hash: Some(hash.into()),
                }),
            )?;
            insert_resource(
                &tx,
                name,
                &ResearchData::PdfAttachment(PdfAttachment {
                    document_id: document.id.clone(),
                    reference_id: None,
                    content_hash: hash.into(),
                    byte_size: bytes,
                }),
            )?;
            document.id
        };
        tx.commit()?;
        let record = self.pdf_record(&id)?;
        if record.content_hash != hash || record.byte_size != bytes {
            return Err(StorageError::Conflict);
        }
        Ok(record)
    }
    pub fn pdf_record(&self, id: &str) -> Result<PdfRecord> {
        let doc = self.research_resource(&ResourceKey {
            kind: ResourceKind::Document,
            id: id.into(),
        })?;
        let ResearchData::Document(data) = doc.data()? else {
            return Err(StorageError::Invalid("资源不是PDF。"));
        };
        if data.document_type != DocumentType::Pdf {
            return Err(StorageError::Invalid("资源不是PDF。"));
        }
        let hash = data
            .content_hash
            .ok_or(StorageError::Invalid("PDF缺少摘要。"))?;
        let (attachment_id,json):(String,String)=self.connection.query_row("SELECT id,data_json FROM research_entities WHERE resource_type='pdfAttachment' AND json_extract(data_json,'$.value.documentId')=?1 ORDER BY id LIMIT 1",[id],|r|Ok((r.get(0)?,r.get(1)?)))?;
        let ResearchData::PdfAttachment(attachment) =
            serde_json::from_str(&json).map_err(|_| StorageError::Invalid("PDF附件损坏。"))?
        else {
            return Err(StorageError::Invalid("PDF附件无效。"));
        };
        if attachment.content_hash != hash {
            return Err(StorageError::Conflict);
        }
        let saved:Option<(String,i64,String,i64)>=self.connection.query_row("SELECT content_hash,format_version,location_json,revision FROM pdf_reading_positions WHERE document_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
        let reading = match saved {
            None => None,
            Some((h, v, json, revision)) => {
                if h != hash || v != 1 {
                    return Err(StorageError::Invalid(
                        "PDF阅读状态版本或内容不匹配，未覆盖记录。",
                    ));
                }
                let location: PdfReadingLocation = serde_json::from_str(&json)
                    .map_err(|_| StorageError::Invalid("PDF阅读位置损坏。"))?;
                location.validate()?;
                Some(PdfReadingState { revision, location })
            }
        };
        Ok(PdfRecord {
            document_id: id.into(),
            attachment_id,
            title: doc.title,
            content_hash: hash,
            byte_size: attachment.byte_size,
            reading,
        })
    }
    pub fn pdf_file_path(&self, id: &str) -> Result<PathBuf> {
        let record = self.pdf_record(id)?;
        let resource = self.research_resource(&ResourceKey {
            kind: ResourceKind::Document,
            id: id.into(),
        })?;
        let ResearchData::Document(data) = resource.data()? else {
            return Err(StorageError::Invalid("不是PDF文档。"));
        };
        let fallback = self
            .pdf_directory()?
            .join(format!("{}.pdf", record.content_hash));
        if let Some(relative) = data.relative_path {
            if !relative.starts_with("pdfs/") || !relative_path_valid(&relative) {
                return Err(StorageError::Invalid(
                    "PDF路径不是受管工作区路径，请重新选择原PDF导入。",
                ));
            }
            let mut path = self.data_dir.clone();
            for part in relative.split('/') {
                path.push(part);
                files::existing_ordinary(&path)?;
            }
            if path.try_exists()? {
                if !path
                    .canonicalize()?
                    .starts_with(self.data_dir.join("pdfs").canonicalize()?)
                {
                    return Err(StorageError::Invalid("PDF路径越出受管目录。"));
                }
                return Ok(path);
            }
        }
        files::existing_ordinary(&fallback)?;
        Ok(fallback)
    }
    pub fn recent_pdfs(&self) -> Result<Vec<PdfRecord>> {
        let mut q=self.connection.prepare("SELECT e.id FROM research_entities e LEFT JOIN pdf_reading_positions p ON p.document_id=e.id WHERE e.resource_type='document' AND json_extract(e.data_json,'$.value.documentType')='pdf' ORDER BY coalesce(p.updated_at,e.updated_at) DESC LIMIT 50")?;
        let ids = q
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.into_iter().map(|id| self.pdf_record(&id)).collect()
    }
    pub fn save_pdf_reading(
        &mut self,
        id: &str,
        hash: &str,
        expected: i64,
        location: &PdfReadingLocation,
    ) -> Result<i64> {
        location.validate()?;
        if expected < 0 || expected == i64::MAX {
            return Err(StorageError::Invalid("PDF阅读版本无效。"));
        }
        let record = self.pdf_record(id)?;
        if record.content_hash != hash
            || record.reading.as_ref().map_or(0, |r| r.revision) != expected
        {
            return Err(StorageError::Conflict);
        }
        let json = serde_json::to_string(location)
            .map_err(|_| StorageError::Invalid("阅读位置无法编码。"))?;
        let changed = if expected == 0 {
            self.connection.execute(
                "INSERT INTO pdf_reading_positions VALUES('document',?1,?2,1,1,?3,?4)",
                params![id, hash, json, now()?],
            )?
        } else {
            self.connection.execute("UPDATE pdf_reading_positions SET revision=revision+1,location_json=?1,updated_at=?2 WHERE document_id=?3 AND revision=?4 AND content_hash=?5",params![json,now()?,id,expected,hash])?
        };
        if changed != 1 {
            return Err(StorageError::Conflict);
        }
        Ok(expected + 1)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_pdf_locator_and_content_identity_are_reused_without_moving_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Storage::open(dir.path().into()).unwrap();
        let hash = "a".repeat(64);
        let first = s.register_pdf("旧论文", &hash, 12).unwrap();
        let key = ResourceKey {
            kind: ResourceKind::Document,
            id: first.document_id.clone(),
        };
        let original = s.research_resource(&key).unwrap();
        let ResearchData::Document(mut data) = original.data().unwrap() else {
            panic!()
        };
        data.relative_path = Some("pdfs/原有 中文论文.pdf".into());
        s.update_research_resource(
            &key,
            original.revision.unwrap(),
            "旧论文",
            &ResearchData::Document(data),
        )
        .unwrap();
        let root = s.pdf_directory().unwrap();
        let legacy = root.join("原有 中文论文.pdf");
        std::fs::write(&legacy, b"%PDF-1.7\nabc").unwrap();
        let reopened = s.register_pdf("同内容新名字", &hash, 12).unwrap();
        assert_eq!(reopened.document_id, first.document_id);
        assert_eq!(reopened.attachment_id, first.attachment_id);
        assert_eq!(s.pdf_file_path(&first.document_id).unwrap(), legacy);
        assert_eq!(std::fs::read(&legacy).unwrap(), b"%PDF-1.7\nabc");
    }
    #[test]
    fn schema14_upgrade_preserves_catalog_and_verifies_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("隔离升级");
        let mut s = Storage::open(root.clone()).unwrap();
        let p = s
            .register_pdf("原有PDF元数据", &"a".repeat(64), 12)
            .unwrap();
        let before: String = s
            .connection
            .query_row(
                "SELECT group_concat(data_json,'|') FROM research_entities",
                [],
                |r| r.get(0),
            )
            .unwrap();
        super::super::migrations::legacy_zotero_fixture(&s.connection);
        s.connection
            .execute_batch("DROP TABLE pdf_reading_positions; PRAGMA user_version=14;")
            .unwrap();
        drop(s);
        let s = Storage::open(root.clone()).unwrap();
        assert_eq!(
            s.pdf_record(&p.document_id).unwrap().attachment_id,
            p.attachment_id
        );
        let after: String = s
            .connection
            .query_row(
                "SELECT group_concat(data_json,'|') FROM research_entities",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(before, after);
        assert_eq!(
            s.connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            crate::storage::migrations::SCHEMA_VERSION
        );
        let backups = std::fs::read_dir(root.join("backups"))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(backups.len(), 1);
        let copy = rusqlite::Connection::open_with_flags(
            backups[0].path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            copy.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            14
        );
        assert_eq!(
            copy.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        assert_eq!(
            copy.query_row(
                "SELECT group_concat(data_json,'|') FROM research_entities",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            before
        );
    }
    #[test]
    fn failed_schema15_rolls_back_without_resetting_schema14() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("w");
        let mut s = Storage::open(root.clone()).unwrap();
        let p = s.register_pdf("旧目录", &"a".repeat(64), 12).unwrap();
        super::super::migrations::legacy_zotero_fixture(&s.connection);
        s.connection
            .execute_batch("PRAGMA user_version=14;")
            .unwrap();
        drop(s);
        assert!(Storage::open(root.clone()).is_err());
        let c = rusqlite::Connection::open(root.join("researchclip.sqlite3")).unwrap();
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            14
        );
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM research_entities WHERE id=?1",
                [p.document_id],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM pdf_reading_positions", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn registration_identity_and_reading_restart() {
        let dir = tempfile::tempdir().unwrap();
        let hash = "a".repeat(64);
        let mut s = Storage::open(dir.path().into()).unwrap();
        let p = s.register_pdf("练习.pdf", &hash, 100).unwrap();
        assert_eq!(
            s.register_pdf("改名.pdf", &hash, 100).unwrap().document_id,
            p.document_id
        );
        let location = PdfReadingLocation {
            page_index: 4,
            rotation: 90,
            zoom: 1.25,
            zoom_mode: "width".into(),
            spread: true,
            offset_x: 0.1,
            offset_y: 0.4,
        };
        assert_eq!(
            s.save_pdf_reading(&p.document_id, &hash, 0, &location)
                .unwrap(),
            1
        );
        assert!(
            s.save_pdf_reading(&p.document_id, &hash, 0, &location)
                .is_err()
        );
        assert!(
            s.save_pdf_reading(&p.document_id, &"b".repeat(64), 1, &location)
                .is_err()
        );
        drop(s);
        let s = Storage::open(dir.path().into()).unwrap();
        let state = s.pdf_record(&p.document_id).unwrap().reading.unwrap();
        assert_eq!(state.revision, 1);
        assert_eq!(state.location.page_index, 4);
        assert_eq!(state.location.rotation, 90);
        assert_eq!(s.recent_pdfs().unwrap().len(), 1);
    }
    #[test]
    fn bounds_and_atomic_failure() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Storage::open(dir.path().into()).unwrap();
        assert!(s.register_pdf("", &"a".repeat(64), 100).is_err());
        assert_eq!(s.research_resources(None, 0, 100).unwrap().len(), 0);
        assert!(s.register_pdf("PDF", "x", 100).is_err());
        let mut l = PdfReadingLocation {
            page_index: 0,
            rotation: 0,
            zoom: 1.,
            zoom_mode: "custom".into(),
            spread: false,
            offset_x: 0.,
            offset_y: 0.,
        };
        assert!(l.validate().is_ok());
        l.zoom = f64::NAN;
        assert!(l.validate().is_err());
        l.zoom = 1.;
        l.rotation = 45;
        assert!(l.validate().is_err());
        l.rotation = 0;
        l.offset_y = 1.01;
        assert!(l.validate().is_err());
    }
}
