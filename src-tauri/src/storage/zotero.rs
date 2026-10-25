//! Zotero provenance over existing references/resources, committed atomically.
use super::{
    Storage, StorageError,
    references::{Reference, ReferenceWrite, write_reference},
};
use crate::{
    domain::research::{ResourceKey, ResourceKind},
    services::zotero_local::{ZoteroCandidate, valid_item_key},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
type Result<T> = std::result::Result<T, StorageError>;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoteroSource {
    pub instance_id: String,
    pub library_type: String,
    pub library_id: u64,
    pub item_key: String,
    pub attachment_key: String,
    pub reference: Reference,
    pub snapshot: serde_json::Value,
}
fn mapping(conn: &Connection, c: &ZoteroCandidate, key: &str) -> Result<Option<(String, String)>> {
    Ok(conn.query_row("SELECT resource_type,resource_id FROM research_zotero_keys WHERE instance_id=?1 AND library_type=?2 AND library_id=?3 AND item_key=?4",params![c.instance_id,c.library_type,c.library_id,key],|r|Ok((r.get(0)?,r.get(1)?))).optional()?)
}
fn bind(conn: &Connection, c: &ZoteroCandidate, key: &str, kind: &str, id: &str) -> Result<()> {
    if let Some((old_kind, old_id)) = mapping(conn, c, key)? {
        if old_kind != kind || old_id != id {
            return Err(StorageError::Invalid(
                "此Zotero键已关联另一资源或内容版本；本阶段不覆盖历史来源。",
            ));
        }
        return Ok(());
    }
    conn.execute("INSERT INTO research_zotero_keys(instance_id,library_id,item_key,resource_type,resource_id,library_type,metadata_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![c.instance_id,c.library_id,key,kind,id,c.library_type,c.reference.source_metadata.to_string()])?;
    Ok(())
}
impl Storage {
    pub fn bind_pdf_zotero(
        &mut self,
        document_id: &str,
        content_hash: &str,
        c: &ZoteroCandidate,
    ) -> Result<String> {
        ResourceKey {
            kind: ResourceKind::Document,
            id: c.instance_id.clone(),
        }
        .validate()
        .map_err(StorageError::Invalid)?;
        if !matches!(c.library_type.as_str(), "user" | "group")
            || c.library_id > 9_007_199_254_740_991
            || (c.library_type == "group" && c.library_id == 0)
            || !valid_item_key(&c.item_key)
            || !valid_item_key(&c.attachment_key)
            || c.item_key == c.attachment_key
            || c.reference.source_metadata.to_string().len() > 65536
            || c.reference.source_metadata["provider"] != "zoteroLocal"
            || c.reference.source_metadata["status"] != "unverified"
        {
            return Err(StorageError::Invalid("Zotero来源范围或格式无效。"));
        }
        let pdf = self.pdf_record(document_id)?;
        if pdf.content_hash != content_hash {
            return Err(StorageError::Conflict);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count:i64=tx.query_row("SELECT count(*) FROM research_zotero_keys WHERE resource_type='pdfAttachment' AND resource_id=?1 AND library_type!='legacy'",[&pdf.attachment_id],|r|r.get(0))?;
        if count >= 100 && mapping(&tx, c, &c.attachment_key)?.is_none() {
            return Err(StorageError::Invalid("每份PDF最多关联100项Zotero来源。"));
        }
        let reference_id = if let Some((kind, id)) = mapping(&tx, c, &c.item_key)? {
            if kind != "literature" {
                return Err(StorageError::Conflict);
            }
            id
        } else {
            let existing: Option<String> = if c.reference.doi.is_empty() {
                None
            } else {
                tx.query_row(
                    "SELECT id FROM literature_references WHERE doi=?1",
                    [&c.reference.doi],
                    |r| r.get(0),
                )
                .optional()?
            };
            if let Some(id) = existing {
                id
            } else {
                write_reference(
                    &tx,
                    ReferenceWrite {
                        id: None,
                        expected_revision: None,
                        data: c.reference.clone(),
                        confirm_key_change: false,
                    },
                )?
            }
        };
        bind(&tx, c, &c.item_key, "literature", &reference_id)?;
        bind(
            &tx,
            c,
            &c.attachment_key,
            "pdfAttachment",
            &pdf.attachment_id,
        )?;
        tx.execute("INSERT OR IGNORE INTO research_relations VALUES('document',?1,'literature',?2,'source','manual')",params![document_id,reference_id])?;
        tx.commit()?;
        Ok(reference_id)
    }
    pub fn pdf_zotero_sources(&self, document_id: &str) -> Result<Vec<ZoteroSource>> {
        let pdf = self.pdf_record(document_id)?;
        let mut q=self.connection.prepare("SELECT a.instance_id,a.library_type,a.library_id,p.item_key,a.item_key,p.resource_id,a.metadata_json FROM research_zotero_keys a JOIN research_zotero_keys p ON p.instance_id=a.instance_id AND p.library_type=a.library_type AND p.library_id=a.library_id AND p.item_key=json_extract(a.metadata_json,'$.itemKey') AND p.resource_type='literature' WHERE a.resource_type='pdfAttachment' AND a.resource_id=?1 AND a.library_type!='legacy' ORDER BY a.instance_id,a.library_type,a.library_id,a.item_key LIMIT 101")?;
        let raw = q
            .query_map([pdf.attachment_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, u64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if raw.len() > 100 {
            return Err(StorageError::Invalid("此PDF来源超过100项保护预算。"));
        }
        raw.into_iter()
            .map(
                |(instance_id, library_type, library_id, item_key, attachment_key, id, json)| {
                    let snapshot: serde_json::Value = serde_json::from_str(&json)
                        .map_err(|_| StorageError::Invalid("Zotero来源快照损坏。"))?;
                    if snapshot["formatVersion"] != 1 {
                        return Err(StorageError::Invalid("来源快照版本不受支持。"));
                    }
                    let reference = self
                        .citation_references(&[id])?
                        .pop()
                        .ok_or(StorageError::NotFound)?;
                    Ok(ZoteroSource {
                        instance_id,
                        library_type,
                        library_id,
                        item_key,
                        attachment_key,
                        reference,
                        snapshot,
                    })
                },
            )
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::zotero_local::{
        ZoteroHealth, ZoteroScope, instance_identity, metadata_candidate,
    };
    use serde_json::json;
    fn candidate(kind: &str, id: u64, server: &str, doi: &str) -> ZoteroCandidate {
        let h = ZoteroHealth {
            api_version: 3,
            schema_version: Some(44),
            instance_id: instance_identity(server).unwrap(),
            server_id: server.into(),
            product_version: None,
        };
        let scope = ZoteroScope {
            library_type: kind.into(),
            library_id: if kind == "user" { 0 } else { id },
        };
        let library = json!({"type":kind,"id":id});
        let a = json!({"key":"ATTACH01","version":2,"library":library,"data":{"key":"ATTACH01","itemType":"attachment","contentType":"application/pdf","parentItem":"PARENT01"}});
        let p = json!({"key":"PARENT01","version":2,"library":library,"data":{"key":"PARENT01","itemType":"journalArticle","title":"测试来源（未核实）","DOI":doi}});
        metadata_candidate(&h, &scope, &a, &p).unwrap()
    }
    #[test]
    fn persists_scoped_sources_and_reuses_existing_doi_without_overwrite() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Storage::open(d.path().into()).unwrap();
        let pdf = s.register_pdf("PDF", &"a".repeat(64), 12).unwrap();
        let c = candidate("user", 0, "FixtureServer1", "10.1234/test");
        let mut manual = c.reference.clone();
        manual.title = "人工校对标题，必须保留".into();
        manual.bibtex_key = "ManualA".into();
        manual.source_metadata = json!({});
        let original = s
            .save_references(vec![ReferenceWrite {
                id: None,
                expected_revision: None,
                data: manual,
                confirm_key_change: false,
            }])
            .unwrap()
            .remove(0);
        let id = s
            .bind_pdf_zotero(&pdf.document_id, &pdf.content_hash, &c)
            .unwrap();
        assert_eq!(id, original.id);
        assert_eq!(
            s.citation_references(std::slice::from_ref(&id)).unwrap()[0]
                .data
                .title,
            "人工校对标题，必须保留"
        );
        assert_eq!(
            s.bind_pdf_zotero(&pdf.document_id, &pdf.content_hash, &c)
                .unwrap(),
            id
        );
        let group = candidate("group", 5, "FixtureServer1", "10.1234/test");
        assert_eq!(
            s.bind_pdf_zotero(&pdf.document_id, &pdf.content_hash, &group)
                .unwrap(),
            id
        );
        let other = candidate("user", 0, "FixtureServer2", "10.1234/test");
        assert_eq!(
            s.bind_pdf_zotero(&pdf.document_id, &pdf.content_hash, &other)
                .unwrap(),
            id
        );
        let sources = s.pdf_zotero_sources(&pdf.document_id).unwrap();
        assert_eq!(sources.len(), 3);
        assert_eq!(sources[0].reference.revision, 1);
        drop(s);
        let s = Storage::open(d.path().into()).unwrap();
        assert_eq!(s.pdf_zotero_sources(&pdf.document_id).unwrap().len(), 3);
        assert_eq!(
            s.pdf_zotero_sources(&pdf.document_id).unwrap()[0].snapshot["status"],
            "unverified"
        );
    }
    #[test]
    fn attachment_conflict_rolls_back_new_reference_and_keys() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Storage::open(d.path().into()).unwrap();
        let a = s.register_pdf("A", &"a".repeat(64), 12).unwrap();
        let b = s.register_pdf("B", &"b".repeat(64), 12).unwrap();
        let first = candidate("user", 0, "FixtureServer1", "");
        s.bind_pdf_zotero(&a.document_id, &a.content_hash, &first)
            .unwrap();
        let mut next = candidate("user", 0, "FixtureServer1", "");
        next.item_key = "PARENT02".into();
        next.reference.bibtex_key.push_str("_next");
        next.reference.source_metadata["itemKey"] = json!("PARENT02");
        assert!(
            s.bind_pdf_zotero(&b.document_id, &b.content_hash, &next)
                .is_err()
        );
        assert_eq!(s.reference_page("", 0, None).unwrap().total, 1);
        assert!(mapping(&s.connection, &next, "PARENT02").unwrap().is_none());
        assert!(s.pdf_zotero_sources(&b.document_id).unwrap().is_empty());
        assert_eq!(s.pdf_zotero_sources(&a.document_id).unwrap().len(), 1);
    }
    #[test]
    fn migration15_preserves_legacy_keys_and_reading_positions() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Storage::open(d.path().into()).unwrap();
        let pdf = s.register_pdf("旧PDF", &"a".repeat(64), 12).unwrap();
        let location = super::super::pdf::PdfReadingLocation { page_index: 3, rotation: 270, zoom: 1.5, zoom_mode: "width".into(), spread: true, offset_x: 0.2, offset_y: 0.6 };
        s.save_pdf_reading(&pdf.document_id, &pdf.content_hash, 0, &location).unwrap();
        let c = candidate("user", 1, "FixtureServer1", "");
        let legacy = crate::domain::research::ZoteroKey {
            instance_id: c.instance_id.clone(),
            library_id: 1,
            item_key: "ATTACH01".into(),
        };
        let target = ResourceKey {
            kind: ResourceKind::PdfAttachment,
            id: pdf.attachment_id.clone(),
        };
        s.bind_zotero_resource(&legacy, &target).unwrap();
        super::super::migrations::legacy_zotero_fixture(&s.connection);
        s.connection
            .pragma_update(None, "user_version", 15)
            .unwrap();
        drop(s);
        let s = Storage::open(d.path().into()).unwrap();
        assert_eq!(s.zotero_resource(&legacy).unwrap(), Some(target));
        let reading = s.pdf_record(&pdf.document_id).unwrap().reading.unwrap();
        assert_eq!(reading.location.page_index, 3);
        assert_eq!(reading.location.rotation, 270);
        assert_eq!(reading.location.offset_y, 0.6);
        assert_eq!(
            s.pdf_record(&pdf.document_id).unwrap().document_id,
            pdf.document_id
        );
        let backups = std::fs::read_dir(d.path().join("backups"))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(backups.len(), 1);
        let old = Connection::open_with_flags(
            backups[0].path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            old.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            15
        );
        assert_eq!(
            old.query_row("SELECT count(*) FROM research_zotero_keys", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn failed_schema16_keeps_old_schema_and_user_records() {
        let d=tempfile::tempdir().unwrap();
        let mut s=Storage::open(d.path().into()).unwrap();
        let pdf=s.register_pdf("保留旧资料", &"a".repeat(64), 12).unwrap();
        super::super::migrations::legacy_zotero_fixture(&s.connection);
        s.connection.execute_batch("PRAGMA user_version=15; CREATE TABLE research_zotero_keys_v15(guard TEXT);").unwrap();
        drop(s);
        assert!(Storage::open(d.path().into()).is_err());
        let c=Connection::open(d.path().join("researchclip.sqlite3")).unwrap();
        assert_eq!(c.query_row("PRAGMA user_version",[],|r|r.get::<_,u32>(0)).unwrap(),15);
        assert_eq!(c.query_row("SELECT count(*) FROM research_entities WHERE id=?1",[pdf.document_id],|r|r.get::<_,i64>(0)).unwrap(),1);
        assert!(c.prepare("SELECT library_type FROM research_zotero_keys").is_err());
    }

}
