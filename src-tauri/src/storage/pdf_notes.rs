//! A derived edge index over existing Markdown IDs and unified PDF locations.
use super::{Storage, StorageError, links::Backlink};
use crate::domain::research::{PdfLocation, ResearchData, ResourceKey, ResourceKind};
use rusqlite::params;
use serde::Serialize;
type Result<T> = std::result::Result<T, StorageError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::research::{PageRectangle, TextAnchor};
    use crate::storage::links::IndexedLink;
    fn location(s: &mut Storage) -> PdfLocation {
        let p = s.register_pdf("论文", &"a".repeat(64), 100).unwrap();
        PdfLocation {
            attachment_id: p.attachment_id,
            content_hash: p.content_hash,
            page_index: 2,
            page_label: None,
            page_width: 600.0,
            page_height: 800.0,
            rectangles: vec![PageRectangle {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 30.0,
            }],
            text_anchor: Some(TextAnchor {
                exact: "多行文字".into(),
                prefix: String::new(),
                suffix: String::new(),
            }),
        }
    }
    fn link(id: &str) -> (String, IndexedLink) {
        let text = format!("[来源](researchclip://pdf-location/{id})");
        let end = text.encode_utf16().count();
        (
            text,
            IndexedLink {
                target_id: id.into(),
                link_type: "pdf-location".into(),
                start: 0,
                end,
                label: "来源".into(),
                anchor_id: None,
            },
        )
    }
    #[test]
    fn migration17_preserves_provenance_and_old_notes_then_reindexes_without_changing_source() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().join("data")).unwrap();
        let l = location(&mut s);
        let id = s.create_pdf_text_location(&l).unwrap();
        let (raw, edge) = link(&id);
        let path = t.path().join("旧笔记.md");
        std::fs::write(&path, &raw).unwrap();
        let doc = s.index_document(&path, None).unwrap().id;
        super::super::migrations::discard_annotation_cache_fixture(&s.connection);
        s.connection
            .execute_batch("DROP TABLE pdf_note_links; PRAGMA user_version=17;")
            .unwrap();
        drop(s);
        let mut s = Storage::open(t.path().join("data")).unwrap();
        assert_eq!(
            s.status().unwrap().schema_version,
            super::super::migrations::SCHEMA_VERSION
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
        let source = s.link_source(&doc).unwrap();
        s.replace_links(&doc, &source.stamp, &[edge]).unwrap();
        assert_eq!(
            s.pdf_note_results(&l).unwrap().notes[0].backlink.source_id,
            doc
        );
        drop(s);
        let s = Storage::open(t.path().join("data")).unwrap();
        assert_eq!(s.pdf_note_results(&l).unwrap().location_ids, [id]);
        assert!(
            std::fs::read_dir(t.path().join("data/backups"))
                .unwrap()
                .any(|p| p
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("schema-17-"))
        );
    }
    #[test]
    fn query_uses_page_hash_and_intersection_and_handles_rename_move_external_change_and_delete() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().join("data")).unwrap();
        let l = location(&mut s);
        let id = s.create_pdf_text_location(&l).unwrap();
        let (raw, edge) = link(&id);
        let path = t.path().join("A.md");
        std::fs::write(&path, &raw).unwrap();
        let doc = s.index_document(&path, None).unwrap().id;
        let stamp = s.link_source(&doc).unwrap().stamp;
        s.replace_links(&doc, &stamp, &[edge]).unwrap();
        assert!(!s.pdf_note_results(&l).unwrap().notes[0].stale);
        for other in [
            PdfLocation {
                page_index: 3,
                ..l.clone()
            },
            PdfLocation {
                content_hash: "b".repeat(64),
                ..l.clone()
            },
            PdfLocation {
                rectangles: vec![PageRectangle {
                    x: 300.0,
                    y: 300.0,
                    width: 10.0,
                    height: 10.0,
                }],
                ..l.clone()
            },
        ] {
            assert!(s.pdf_note_results(&other).unwrap().notes.is_empty());
        }
        let moved = t.path().join("移位 B.md");
        std::fs::rename(&path, &moved).unwrap();
        assert_eq!(s.index_document(&moved, Some(&path)).unwrap().id, doc);
        assert!(!s.pdf_note_results(&l).unwrap().notes[0].stale);
        std::fs::write(&moved, format!("新增段落\n{raw}")).unwrap();
        assert!(s.pdf_note_results(&l).unwrap().notes[0].stale);
        std::fs::remove_file(&moved).unwrap();
        let result = s.pdf_note_results(&l).unwrap();
        assert!(!result.notes[0].backlink.available);
        assert!(result.notes[0].stale);
    }
    #[test]
    fn bad_stamp_span_or_failed_insert_keeps_both_prior_indexes_atomic() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().join("data")).unwrap();
        let l = location(&mut s);
        let id = s.create_pdf_text_location(&l).unwrap();
        let (raw, edge) = link(&id);
        let path = t.path().join("A.md");
        std::fs::write(&path, &raw).unwrap();
        let doc = s.index_document(&path, None).unwrap().id;
        let source = s.link_source(&doc).unwrap();
        s.replace_links(&doc, &source.stamp, std::slice::from_ref(&edge))
            .unwrap();
        assert!(matches!(
            s.replace_links(&doc, &"b".repeat(64), std::slice::from_ref(&edge)),
            Err(StorageError::Conflict)
        ));
        let mut invalid = edge.clone();
        invalid.target_id = "b".repeat(32);
        assert!(s.replace_links(&doc, &source.stamp, &[invalid]).is_err());
        s.connection.execute_batch("CREATE TRIGGER fail_pdf_index BEFORE INSERT ON pdf_note_links BEGIN SELECT RAISE(ABORT,'fixture fault'); END;").unwrap();
        assert!(s.replace_links(&doc, &source.stamp, &[edge]).is_err());
        assert_eq!(s.pdf_note_results(&l).unwrap().notes.len(), 1);
        assert_eq!(s.link_source(&doc).unwrap().stamp, source.stamp);
    }
    #[test]
    fn invalid_text_sources_never_create_locations_and_managed_copy_detects_replacement() {
        use crate::services::pdf::PdfDocumentService;
        use sha2::{Digest, Sha256};
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().join("data")).unwrap();
        let l = location(&mut s);
        for invalid in [
            PdfLocation {
                text_anchor: None,
                ..l.clone()
            },
            PdfLocation {
                text_anchor: Some(TextAnchor {
                    exact: "x".repeat(4097),
                    prefix: String::new(),
                    suffix: String::new(),
                }),
                ..l.clone()
            },
            PdfLocation {
                content_hash: "b".repeat(64),
                ..l.clone()
            },
        ] {
            assert!(s.create_pdf_text_location(&invalid).is_err());
        }
        assert_eq!(
            s.connection
                .query_row(
                    "SELECT count(*) FROM research_entities WHERE resource_type='pdfLocation'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        let bytes = b"%PDF-1.7\nowned test";
        let hash = format!("{:x}", Sha256::digest(bytes));
        let record = s.register_pdf("测试", &hash, bytes.len() as u64).unwrap();
        let path = s.pdf_directory().unwrap().join(format!("{hash}.pdf"));
        std::fs::write(&path, bytes).unwrap();
        PdfDocumentService::verify_managed(&path, &record).unwrap();
        std::fs::write(&path, b"%PDF-1.7\nreplaced!!").unwrap();
        assert!(PdfDocumentService::verify_managed(&path, &record).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"%PDF-1.7\nreplaced!!");
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfNoteResults {
    pub location_ids: Vec<String>,
    pub notes: Vec<PdfNote>,
    pub truncated: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfNote {
    #[serde(flatten)]
    pub backlink: Backlink,
    pub location_id: String,
    pub stale: bool,
}
pub fn overlap(
    a: &crate::domain::research::PageRectangle,
    b: &crate::domain::research::PageRectangle,
) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}
impl Storage {
    pub fn create_pdf_text_location(&mut self, location: &PdfLocation) -> Result<String> {
        ResearchData::PdfLocation(location.clone())
            .validate()
            .map_err(StorageError::Invalid)?;
        if location.page_index >= 2000
            || location.rectangles.len() > 100
            || location
                .text_anchor
                .as_ref()
                .is_none_or(|a| a.exact.chars().count() > 4096)
        {
            return Err(StorageError::Invalid("文字引用超出范围或缺少文字锚点。"));
        }
        let a = self.research_resource(&ResourceKey {
            kind: ResourceKind::PdfAttachment,
            id: location.attachment_id.clone(),
        })?;
        let ResearchData::PdfAttachment(attachment) = a.data()? else {
            return Err(StorageError::Invalid("附件无效。"));
        };
        if self.pdf_record(&attachment.document_id)?.content_hash != location.content_hash {
            return Err(StorageError::Conflict);
        }
        let row = self.create_research_resource(
            "PDF文字引用",
            &ResearchData::PdfLocation(location.clone()),
        )?;
        Ok(row.key.id)
    }
    pub fn pdf_note_results(&self, location: &PdfLocation) -> Result<PdfNoteResults> {
        ResearchData::PdfLocation(location.clone())
            .validate()
            .map_err(StorageError::Invalid)?;
        if location.page_index >= 2000 {
            return Err(StorageError::Invalid("页码超过支持范围。"));
        }
        let mut q=self.connection.prepare("SELECT id,data_json FROM research_entities WHERE resource_type='pdfLocation' AND json_extract(data_json,'$.value.attachmentId')=?1 AND json_extract(data_json,'$.value.contentHash')=?2 AND json_extract(data_json,'$.value.pageIndex')=?3 ORDER BY id LIMIT 1001")?;
        let rows = q
            .query_map(
                params![
                    location.attachment_id,
                    location.content_hash,
                    location.page_index
                ],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut result = PdfNoteResults {
            location_ids: Vec::new(),
            notes: Vec::new(),
            truncated: rows.len() > 1000,
        };
        for (id, json) in rows.into_iter().take(1000) {
            let data: ResearchData = serde_json::from_str(&json)
                .map_err(|_| StorageError::Invalid("来源位置数据损坏。"))?;
            let ResearchData::PdfLocation(found) = data else {
                return Err(StorageError::Invalid("来源类型无效。"));
            };
            if !location
                .rectangles
                .iter()
                .any(|a| found.rectangles.iter().any(|b| overlap(a, b)))
            {
                continue;
            }
            result.location_ids.push(id.clone());
            let mut query=self.connection.prepare("SELECT l.source_id,d.name,l.source_location,l.source_line,l.label,d.path,coalesce(s.source_stamp,'') FROM pdf_note_links l JOIN markdown_documents d ON d.id=l.source_id LEFT JOIN link_index_state s ON s.document_id=d.id WHERE l.target_id=?1 ORDER BY d.name,l.source_location LIMIT 101")?;
            let notes = query
                .query_map([&id], |r| {
                    Ok((
                        Backlink {
                            source_id: r.get(0)?,
                            title: r.get(1)?,
                            start: r.get::<_, i64>(2)? as usize,
                            line: r.get::<_, i64>(3)? as usize,
                            label: r.get(4)?,
                            available: false,
                        },
                        r.get::<_, String>(5)?,
                        r.get::<_, String>(6)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for (mut backlink, path, stamp) in notes {
                if result.notes.len() >= 100 {
                    result.truncated = true;
                    break;
                }
                let source = super::links::read_source(std::path::Path::new(&path));
                backlink.available = source.is_ok();
                let stale = source.is_err() || source.is_ok_and(|s| s.stamp != stamp);
                result.notes.push(PdfNote {
                    backlink,
                    location_id: id.clone(),
                    stale,
                });
            }
        }
        Ok(result)
    }
}
