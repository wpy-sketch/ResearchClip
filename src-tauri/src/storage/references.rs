use super::{
    Storage, StorageError,
    library::{check_id, now},
};
use crate::services::references::{ReferenceInput, bibtex, normalize_doi, validate};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
type Result<T> = std::result::Result<T, StorageError>;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReferenceTarget {
    pub kind: String,
    pub id: String,
}
impl ReferenceTarget {
    fn table(&self) -> Result<(&'static str, &'static str)> {
        check_id(&self.id)?;
        match self.kind.as_str() {
            "screenshot" => Ok(("screenshot_references", "screenshot_id")),
            "document" => Ok(("document_references", "document_id")),
            _ => Err(StorageError::Invalid("文献关联目标无效。")),
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub id: String,
    #[serde(flatten)]
    pub data: ReferenceInput,
    pub updated_at: i64,
    pub revision: i64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReferenceWrite {
    pub id: Option<String>,
    pub expected_revision: Option<i64>,
    pub data: ReferenceInput,
    pub confirm_key_change: bool,
}
#[derive(Serialize)]
pub struct ReferencePage {
    pub items: Vec<Reference>,
    pub total: i64,
}
fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Reference> {
    let authors: String = row.get(2)?;
    let metadata: String = row.get(10)?;
    let json_error =
        |e| rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e));
    Ok(Reference {
        id: row.get(0)?,
        data: ReferenceInput {
            title: row.get(1)?,
            authors: serde_json::from_str(&authors).map_err(json_error)?,
            year: row.get(3)?,
            journal_or_conference: row.get(4)?,
            doi: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
            url: row.get(6)?,
            bibtex_key: row.get(7)?,
            bibtex_entry: row.get(8)?,
            verified_source: row.get(9)?,
            source_metadata: serde_json::from_str(&metadata).map_err(json_error)?,
        },
        updated_at: row.get(11)?,
        revision: row.get(12)?,
    })
}
impl Storage {
    pub fn reference_doi(&self, id: &str) -> Result<String> {
        check_id(id)?;
        let doi: Option<String> = self
            .connection
            .query_row(
                "SELECT doi FROM literature_references WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(StorageError::NotFound)?;
        doi.ok_or(StorageError::Invalid("此文献尚未填写 DOI。"))
    }
    pub fn reference_page(
        &self,
        query: &str,
        page: u32,
        target: Option<&ReferenceTarget>,
    ) -> Result<ReferencePage> {
        if query.chars().count() > 200 || page > 1000 {
            return Err(StorageError::Invalid("文献查询超出长度或页码限制。"));
        }
        let predicate = if let Some(t) = target {
            let (table, column) = t.table()?;
            format!(" AND id IN (SELECT reference_id FROM {table} WHERE {column}=?2)")
        } else {
            String::new()
        };
        // instr treats %, _, quotes and FTS metacharacters literally; never interpolates user text.
        let filter = format!(
            "WHERE (instr(lower(title),lower(?1))>0 OR instr(lower(authors_json),lower(?1))>0 OR instr(lower(coalesce(doi,'')),lower(?1))>0 OR instr(lower(bibtex_key),lower(?1))>0){predicate}"
        );
        let target_id = target.map_or("", |t| t.id.as_str());
        let total = if target.is_some() {
            self.connection.query_row(
                &format!("SELECT count(*) FROM literature_references {filter}"),
                params![query.trim(), target_id],
                |r| r.get(0),
            )?
        } else {
            self.connection.query_row(
                &format!("SELECT count(*) FROM literature_references {filter}"),
                [query.trim()],
                |r| r.get(0),
            )?
        };
        // Explicit parameter numbering keeps query and target independent of pagination.
        let sql = format!(
            "SELECT * FROM literature_references {filter} ORDER BY updated_at DESC,id LIMIT 25 OFFSET ?{}",
            if target.is_some() { 3 } else { 2 }
        );
        let mut stmt = self.connection.prepare(&sql)?;
        let items = if target.is_some() {
            stmt.query_map(params![query.trim(), target_id, page * 25], read)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map(params![query.trim(), page * 25], read)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        Ok(ReferencePage { items, total })
    }
    pub fn save_references(&mut self, items: Vec<ReferenceWrite>) -> Result<Vec<Reference>> {
        if items.is_empty() || items.len() > 200 {
            return Err(StorageError::Invalid("一次保存1至200条文献。"));
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut ids = vec![];
        for mut item in items {
            validate(&item.data).map_err(|_| {
                StorageError::Invalid("文献字段无效；请检查标题、作者、年份、URL 与引用键。")
            })?;
            item.data.doi = normalize_doi(&item.data.doi)
                .map_err(|_| StorageError::Invalid("DOI 格式无效。"))?;
            let id = if let Some(id) = item.id {
                check_id(&id)?;
                let (revision,key,source,metadata):(i64,String,String,String)=tx.query_row("SELECT revision,bibtex_key,verified_source,source_metadata FROM literature_references WHERE id=?1",[&id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?.ok_or(StorageError::NotFound)?;
                if Some(revision) != item.expected_revision {
                    return Err(StorageError::Conflict);
                }
                if key != item.data.bibtex_key && !item.confirm_key_change {
                    return Err(StorageError::Invalid(
                        "修改引用键可能使已有 Markdown 引用失效，请明确确认后保存。",
                    ));
                }
                if source != "manual" {
                    item.data.verified_source = "edited".into();
                    item.data.source_metadata = serde_json::from_str(&metadata)
                        .map_err(|_| StorageError::Invalid("原文献来源记录损坏；原资料保留。"))?;
                }
                id
            } else {
                tx.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?
            };
            let duplicate:bool=tx.query_row("SELECT exists(SELECT 1 FROM literature_references WHERE id!=?1 AND (bibtex_key=?2 OR (?3!='' AND doi=?3)))",params![id,item.data.bibtex_key,item.data.doi],|r|r.get(0))?;
            if duplicate {
                return Err(StorageError::Invalid(
                    "DOI 或引用键已存在；请查找并关联已有文献，或手动选择不冲突的引用键。整批未保存。",
                ));
            }
            let bib = bibtex(&item.data)
                .map_err(|_| StorageError::Invalid("BibTeX 记录或字段无效，请检查导入语法。"))?;
            let d = item.data;
            tx.execute("INSERT INTO literature_references VALUES(?1,?2,?3,?4,?5,nullif(?6,''),?7,?8,?9,?10,?11,?12,1) ON CONFLICT(id) DO UPDATE SET title=excluded.title,authors_json=excluded.authors_json,year=excluded.year,venue=excluded.venue,doi=excluded.doi,url=excluded.url,bibtex_key=excluded.bibtex_key,bibtex_entry=excluded.bibtex_entry,verified_source=excluded.verified_source,source_metadata=excluded.source_metadata,updated_at=excluded.updated_at,revision=literature_references.revision+1",params![id,d.title,serde_json::to_string(&d.authors).map_err(|_|StorageError::Invalid("作者格式无效。"))?,d.year,d.journal_or_conference,d.doi,d.url,d.bibtex_key,bib,d.verified_source,d.source_metadata.to_string(),now()?])?;
            ids.push(id);
        }
        tx.commit()?;
        ids.iter()
            .map(|id| {
                Ok(self.connection.query_row(
                    "SELECT * FROM literature_references WHERE id=?1",
                    [id],
                    read,
                )?)
            })
            .collect()
    }
    pub fn associate_reference(
        &mut self,
        target: &ReferenceTarget,
        id: &str,
        linked: bool,
    ) -> Result<()> {
        let (table, column) = target.table()?;
        check_id(id)?;
        if linked {
            let exists: bool = self.connection.query_row(
                &format!(
                    "SELECT exists(SELECT 1 FROM {table} WHERE {column}=?1 AND reference_id=?2)"
                ),
                params![target.id, id],
                |r| r.get(0),
            )?;
            if exists {
                return Ok(());
            }
            let count: i64 = self.connection.query_row(
                &format!("SELECT count(*) FROM {table} WHERE {column}=?1"),
                [&target.id],
                |r| r.get(0),
            )?;
            if count >= 200 {
                return Err(StorageError::Invalid("每张卡片或文档最多关联200条文献。"));
            }
            self.connection.execute(
                &format!("INSERT OR IGNORE INTO {table}({column},reference_id) VALUES(?1,?2)"),
                params![target.id, id],
            )?;
        } else {
            self.connection.execute(
                &format!("DELETE FROM {table} WHERE {column}=?1 AND reference_id=?2"),
                params![target.id, id],
            )?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn item(key: &str, doi: &str) -> ReferenceWrite {
        ReferenceWrite {
            id: None,
            expected_revision: None,
            confirm_key_change: false,
            data: ReferenceInput {
                title: "中文文献".into(),
                authors: vec!["Li, Wang".into()],
                year: Some(2026),
                journal_or_conference: "Research".into(),
                doi: doi.into(),
                url: String::new(),
                bibtex_key: key.into(),
                bibtex_entry: String::new(),
                verified_source: "manual".into(),
                source_metadata: serde_json::json!({}),
            },
        }
    }
    #[test]
    fn manual_dedup_key_conflict_batch_atomic_and_reopen() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let one = s
            .save_references(vec![item("Li2026", "10.1234/ABC")])
            .unwrap()
            .remove(0);
        assert!(
            s.save_references(vec![
                item("new", ""),
                item("other", "https://doi.org/10.1234/abc")
            ])
            .is_err()
        );
        assert_eq!(s.reference_page("", 0, None).unwrap().total, 1);
        assert!(s.save_references(vec![item("Li2026", "")]).is_err());
        let mut update = item("changed", "");
        update.id = Some(one.id);
        update.expected_revision = Some(one.revision);
        assert!(s.save_references(vec![update]).is_err());
        drop(s);
        let s = Storage::open(tmp.path().into()).unwrap();
        assert_eq!(
            s.reference_page("中文", 0, None).unwrap().items[0]
                .data
                .bibtex_key,
            "Li2026"
        );
        assert_eq!(s.reference_page("%_", 0, None).unwrap().total, 0);
    }
    #[test]
    fn updates_use_revision_provenance_and_explicit_key_confirmation() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let mut request = item("Stable", "");
        request.data.verified_source = "crossref".into();
        request.data.source_metadata =
            serde_json::json!({"provider":"Crossref","originalTitle":"Original"});
        let r = s.save_references(vec![request]).unwrap().remove(0);
        let mut request = item("Changed", "");
        request.id = Some(r.id.clone());
        request.expected_revision = Some(r.revision);
        request.confirm_key_change = true;
        let updated = s.save_references(vec![request]).unwrap().remove(0);
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.data.verified_source, "edited");
        assert_eq!(updated.data.source_metadata["originalTitle"], "Original");
        let mut stale = item("Changed", "");
        stale.id = Some(r.id);
        stale.expected_revision = Some(1);
        assert!(matches!(
            s.save_references(vec![stale]),
            Err(StorageError::Conflict)
        ));
    }
    #[test]
    fn v7_upgrade_snapshot_preserves_original_records_and_corrupt_authors_are_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("researchclip.sqlite3");
        let conn = rusqlite::Connection::open(&path).unwrap();
        for sql in [
            include_str!("../../migrations/001_workspace.sql"),
            include_str!("../../migrations/002_library.sql"),
            include_str!("../../migrations/003_projects.sql"),
            include_str!("../../migrations/004_ocr_index.sql"),
            include_str!("../../migrations/005_annotation_revisions.sql"),
            include_str!("../../migrations/006_sync.sql"),
            include_str!("../../migrations/007_image_features.sql"),
        ] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 7).unwrap();
        conn.execute("INSERT INTO app_metadata VALUES('keep','Original')", [])
            .unwrap();
        drop(conn);
        let mut s = Storage::open(tmp.path().into()).unwrap();
        assert_eq!(s.status().unwrap().schema_version, 10);
        let snapshots = std::fs::read_dir(tmp.path().join("backups"))
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(snapshots.len(), 1);
        let backup = rusqlite::Connection::open(snapshots[0].path()).unwrap();
        assert_eq!(
            backup
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            7
        );
        assert_eq!(
            backup
                .query_row("SELECT value FROM app_metadata WHERE key='keep'", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "Original"
        );
        assert!(
            backup
                .prepare("SELECT * FROM literature_references")
                .is_err()
        );
        s.save_references(vec![item("A", "")]).unwrap();
        s.connection
            .execute("UPDATE literature_references SET authors_json='{}'", [])
            .unwrap();
        assert!(s.reference_page("", 0, None).is_err());
    }
    #[test]
    fn screenshot_document_associations_and_unlink_preserve_data() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let r = s.save_references(vec![item("A", "")]).unwrap().remove(0);
        let shot = "a".repeat(32);
        s.connection.execute("INSERT INTO screenshots(id,title,original_image_path,thumbnail_path,annotation_data,width_px,height_px,created_at,updated_at,is_favorite,revision) VALUES(?1,'t','originals/test.png','thumbnails/test.png',?2,1,1,1,1,0,1)",params![shot,serde_json::json!({"schemaVersion":1,"image":{"widthPx":1,"heightPx":1},"objects":[]}).to_string()]).unwrap();
        let d = "b".repeat(32);
        s.connection
            .execute(
                "INSERT INTO markdown_documents VALUES(?1,'C:/notes.md','notes.md',NULL,1,1)",
                [&d],
            )
            .unwrap();
        for (kind, id) in [("screenshot", shot.clone()), ("document", d)] {
            let target = ReferenceTarget {
                kind: kind.into(),
                id,
            };
            s.associate_reference(&target, &r.id, true).unwrap();
            s.associate_reference(&target, &r.id, true).unwrap();
            assert_eq!(s.reference_page("", 0, Some(&target)).unwrap().total, 1);
            s.associate_reference(&target, &r.id, false).unwrap();
            assert_eq!(s.reference_page("", 0, Some(&target)).unwrap().total, 0);
        }
        assert_eq!(s.reference_page("", 0, None).unwrap().total, 1);
        assert_eq!(
            s.connection
                .query_row("SELECT count(*) FROM screenshots", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
