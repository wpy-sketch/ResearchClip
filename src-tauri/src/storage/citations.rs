//! Citation adapter over existing literature IDs; no duplicate bibliography store.
use super::{
    Storage, StorageError,
    library::check_id,
    references::{Reference, read},
};
use crate::services::references::{MAX_BIB_BYTES, valid_key};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalCitationKey {
    pub provider: String,
    pub instance_id: String,
    pub library_id: i64,
    pub external_key: String,
}
impl ExternalCitationKey {
    fn validate(&self) -> Result<(), StorageError> {
        check_id(&self.instance_id)?;
        if self.provider != "betterBibtex" || self.library_id <= 0 || !valid_key(&self.external_key)
        {
            return Err(StorageError::Invalid(
                "外部引用键必须限定提供方、实例、library ID和有效键。",
            ));
        }
        Ok(())
    }
}
impl Storage {
    pub fn citation_references(&self, ids: &[String]) -> Result<Vec<Reference>, StorageError> {
        if ids.len() > 200 {
            return Err(StorageError::Invalid("一次最多解析200篇文献。"));
        }
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for id in ids {
            check_id(id)?;
            if !seen.insert(id) {
                continue;
            }
            if let Some(r) = self
                .connection
                .query_row(
                    "SELECT * FROM literature_references WHERE id=?1",
                    [id],
                    read,
                )
                .optional()?
            {
                crate::services::references::validate(&r.data)
                    .map_err(|_| StorageError::Invalid("文献信息损坏，原记录保留。"))?;
                result.push(r);
            }
        }
        Ok(result)
    }
    pub fn citation_bibtex(&self, ids: &[String]) -> Result<String, StorageError> {
        if ids.is_empty() {
            return Err(StorageError::Invalid("请选择至少一篇文献。"));
        }
        let refs = self.citation_references(ids)?;
        let unique = ids.iter().collect::<std::collections::HashSet<_>>().len();
        if refs.len() != unique {
            return Err(StorageError::NotFound);
        }
        let mut output = String::new();
        for r in refs {
            if output.len() + r.data.bibtex_entry.len() + 2 > MAX_BIB_BYTES {
                return Err(StorageError::Invalid("BibTeX导出超过1MiB，请分批导出。"));
            }
            output.push_str(&r.data.bibtex_entry);
            output.push_str("\n\n");
        }
        Ok(output)
    }
    pub fn citation_key_mapping(
        &self,
        key: &ExternalCitationKey,
    ) -> Result<Option<String>, StorageError> {
        key.validate()?;
        Ok(self.connection.query_row("SELECT reference_id FROM citation_key_mappings WHERE provider=?1 AND instance_id=?2 AND library_id=?3 AND external_key=?4",params![key.provider,key.instance_id,key.library_id,key.external_key],|r|r.get(0)).optional()?)
    }
    pub fn bind_citation_key(
        &mut self,
        key: &ExternalCitationKey,
        id: &str,
    ) -> Result<(), StorageError> {
        key.validate()?;
        check_id(id)?;
        if let Some(prior) = self.citation_key_mapping(key)? {
            return if prior == id {
                Ok(())
            } else {
                Err(StorageError::Conflict)
            };
        }
        self.connection.execute(
            "INSERT INTO citation_key_mappings VALUES(?1,?2,?3,?4,?5)",
            params![
                key.provider,
                key.instance_id,
                key.library_id,
                key.external_key,
                id
            ],
        )?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::references::{ReferenceInput, import_bib};
    use crate::storage::references::ReferenceWrite;
    fn input(key: &str) -> ReferenceInput {
        ReferenceInput {
            title: "未核实的本地练习".into(),
            authors: vec!["练习作者".into()],
            year: None,
            journal_or_conference: "".into(),
            doi: "".into(),
            url: "".into(),
            bibtex_key: key.into(),
            bibtex_entry: "".into(),
            verified_source: "manual".into(),
            source_metadata: serde_json::json!({}),
        }
    }
    fn create(s: &mut Storage, key: &str) -> Reference {
        s.save_references(vec![ReferenceWrite {
            id: None,
            expected_revision: None,
            data: input(key),
            confirm_key_change: false,
        }])
        .unwrap()
        .remove(0)
    }
    #[test]
    fn stable_ids_key_conflicts_and_local_provenance() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("workspace");
        let mut s = Storage::open(root.clone()).unwrap();
        let r = create(&mut s, "localA");
        assert!(
            s.save_references(vec![ReferenceWrite {
                id: None,
                expected_revision: None,
                data: input("localA"),
                confirm_key_change: false
            }])
            .is_err()
        );
        let mut data = r.data;
        data.title = "改标题仍是同一文献".into();
        data.bibtex_key = "renamedKey".into();
        s.save_references(vec![ReferenceWrite {
            id: Some(r.id.clone()),
            expected_revision: Some(r.revision),
            data,
            confirm_key_change: true,
        }])
        .unwrap();
        drop(s);
        let s = Storage::open(root).unwrap();
        let refs = s.citation_references(&[r.id.clone(), r.id]).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].data.title, "改标题仍是同一文献");
        assert_eq!(refs[0].data.verified_source, "manual");
        assert!(s.citation_references(&["../bad".into()]).is_err());
        assert!(s.citation_references(&vec!["a".repeat(32); 201]).is_err());
    }
    #[test]
    fn external_mapping_is_library_scoped_and_never_reassigns() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().join("w")).unwrap();
        let a = create(&mut s, "A");
        let b = create(&mut s, "B");
        let mut k = ExternalCitationKey {
            provider: "betterBibtex".into(),
            instance_id: "a".repeat(32),
            library_id: 1,
            external_key: "sameKey".into(),
        };
        s.bind_citation_key(&k, &a.id).unwrap();
        s.bind_citation_key(&k, &a.id).unwrap();
        assert!(s.bind_citation_key(&k, &b.id).is_err());
        k.library_id = 2;
        s.bind_citation_key(&k, &b.id).unwrap();
        assert_eq!(s.citation_key_mapping(&k).unwrap(), Some(b.id));
        k.library_id = 0;
        assert!(s.citation_key_mapping(&k).is_err());
    }
    #[test]
    fn bibtex_unknown_fields_roundtrip_and_missing_export_rejected() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().join("w")).unwrap();
        let raw = "@article{Imported, title={A {Nested} Title}, author={Local Author}, custom={Preserve unknown}, year={2026}}";
        let data = import_bib(raw).unwrap().remove(0);
        assert_eq!(data.verified_source, "bibtex");
        let r = s
            .save_references(vec![ReferenceWrite {
                id: None,
                expected_revision: None,
                data,
                confirm_key_change: false,
            }])
            .unwrap()
            .remove(0);
        let bib = s.citation_bibtex(&[r.id]).unwrap();
        assert!(bib.contains("custom={Preserve unknown}"));
        assert_eq!(import_bib(&bib).unwrap()[0].verified_source, "bibtex");
        assert!(s.citation_bibtex(&["f".repeat(32)]).is_err());
        assert!(s.citation_bibtex(&[]).is_err());
        assert_eq!(
            s.reference_page("Preserve unknown", 0, None).unwrap().total,
            1
        );
    }
    #[test]
    fn migration13_preserves_reference_and_creates_verified_snapshot() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("w");
        let mut s = Storage::open(root.clone()).unwrap();
        let r = create(&mut s, "Legacy");
        s.connection
            .execute_batch("DROP TABLE citation_key_mappings; PRAGMA user_version=13;")
            .unwrap();
        drop(s);
        let s = Storage::open(root.clone()).unwrap();
        assert_eq!(
            s.citation_references(std::slice::from_ref(&r.id)).unwrap()[0].id,
            r.id
        );
        let paths = std::fs::read_dir(root.join("backups"))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(paths.len(), 1);
        let c = rusqlite::Connection::open(paths[0].path()).unwrap();
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            13
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM literature_references", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn failed14_migration_keeps13_and_local_references() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("w");
        let mut s = Storage::open(root.clone()).unwrap();
        let r = create(&mut s, "Safe");
        s.connection.execute_batch("DROP TABLE citation_key_mappings; CREATE VIEW citation_key_mappings AS SELECT id FROM literature_references; PRAGMA user_version=13;").unwrap();
        drop(s);
        assert!(Storage::open(root.clone()).is_err());
        let c = rusqlite::Connection::open(root.join("researchclip.sqlite3")).unwrap();
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            13
        );
        assert_eq!(
            c.query_row(
                "SELECT title FROM literature_references WHERE id=?1",
                [r.id],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "未核实的本地练习"
        );
    }
}
