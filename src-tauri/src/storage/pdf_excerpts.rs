//! Provenance over the existing screenshot and research-location stores.
use super::{
    Storage, StorageError,
    library::{check_id, now},
    resources::insert_resource,
};
use crate::domain::research::{PdfLocation, ResearchData, ResourceKey, ResourceKind};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
type Result<T> = std::result::Result<T, StorageError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExcerptDraft {
    pub location: PdfLocation,
    pub literature_id: Option<String>,
    pub dpi: u32,
    pub rotation: u32,
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        capture::RawFrame, domain::research::PageRectangle, services::screen_capture::encode_png,
    };
    fn png() -> Vec<u8> {
        encode_png(RawFrame {
            width_px: 20,
            height_px: 10,
            rgba: vec![190; 20 * 10 * 4],
        })
        .unwrap()
    }
    fn fixture() -> (tempfile::TempDir, Storage, ExcerptDraft, String) {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        // Metadata-only PDF fixture: actual parsing/rendering is exercised separately in Windows.
        let pdf = s.register_pdf("unit PDF", &"a".repeat(64), 4).unwrap();
        let d = ExcerptDraft {
            location: PdfLocation {
                attachment_id: pdf.attachment_id,
                content_hash: pdf.content_hash,
                page_index: 0,
                page_label: None,
                page_width: 600.0,
                page_height: 800.0,
                rectangles: vec![PageRectangle {
                    x: 10.0,
                    y: 20.0,
                    width: 10.0,
                    height: 5.0,
                }],
                text_anchor: None,
            },
            literature_id: None,
            dpi: 144,
            rotation: 0,
        };
        (tmp, s, d, pdf.document_id)
    }
    #[test]
    fn migration16_preserves_legacy_bytes_and_restart_retains_multiple_page_sources() {
        let (tmp, mut s, d, doc) = fixture();
        let raw = png();
        let old=s.create_screenshot("old",serde_json::json!({"schemaVersion":1,"image":{"widthPx":20,"heightPx":10},"objects":[]}),&raw).unwrap();
        s.connection
            .execute_batch(
                "DROP TABLE pdf_excerpts; DROP TABLE pending_pdf_excerpts; PRAGMA user_version=16;",
            )
            .unwrap();
        drop(s);
        let mut s = Storage::open(tmp.path().into()).unwrap();
        assert_eq!(s.status().unwrap().schema_version, 17);
        assert_eq!(s.screenshot_image(&old.id, false).unwrap(), raw);
        assert_eq!(
            s.pdf_record(&doc).unwrap().content_hash,
            d.location.content_hash
        );
        let a = s.create_pdf_excerpt("first", &d, &png()).unwrap();
        let mut second = d.clone();
        second.location.page_index = 1;
        let b = s.create_pdf_excerpt("second", &second, &png()).unwrap();
        assert_ne!(a.screenshot_id, b.screenshot_id);
        assert_eq!(a.pdf_attachment_id, b.pdf_attachment_id);
        assert!(a.literature_id.is_none());
        assert_eq!(s.pdf_excerpts(&doc).unwrap().len(), 2);
        drop(s);
        let s = Storage::open(tmp.path().into()).unwrap();
        let got = s.pdf_excerpt(&a.screenshot_id).unwrap().unwrap();
        assert_eq!(got.page_rectangle.x, 10.0);
        assert_eq!(got.location_id, a.location_id);
        assert_eq!(
            got.image_asset_path,
            format!("originals/{}.png", a.screenshot_id)
        );
        assert_eq!(s.screenshot_image(&old.id, false).unwrap(), png());
        assert_eq!(s.screenshot_image(&a.screenshot_id, false).unwrap(), png());
        let note: String = s
            .connection
            .query_row(
                "SELECT content FROM notes WHERE screenshot_id=?1",
                [a.screenshot_id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(note.contains(&format!("researchclip://pdf-location/{}", a.location_id)));
        assert!(
            std::fs::read_dir(tmp.path().join("backups"))
                .unwrap()
                .any(|f| f
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("schema-16-"))
        );
    }
    #[test]
    fn failed_provenance_transaction_keeps_intent_and_files_then_recovers_atomically() {
        let (tmp, mut s, d, _) = fixture();
        s.connection.execute_batch("CREATE TRIGGER fail_excerpt BEFORE INSERT ON pdf_excerpts BEGIN SELECT RAISE(ABORT,'fixture failure'); END;").unwrap();
        assert!(s.create_pdf_excerpt("pending", &d, &png()).is_err());
        assert_eq!(s.library_status().unwrap().pending_writes, 1);
        assert!(s.list_screenshots(0, 50, false).unwrap().is_empty());
        let n: i64 = s
            .connection
            .query_row(
                "SELECT count(*) FROM research_entities WHERE resource_type='pdfLocation'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0);
        let id: String = s
            .connection
            .query_row("SELECT screenshot_id FROM pending_pdf_excerpts", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            std::fs::read(tmp.path().join(format!("originals/{id}.png"))).unwrap(),
            png()
        );
        s.connection
            .execute_batch("DROP TRIGGER fail_excerpt;")
            .unwrap();
        drop(s);
        let s = Storage::open(tmp.path().into()).unwrap();
        assert_eq!(s.library_status().unwrap().pending_writes, 0);
        let e = s.pdf_excerpt(&id).unwrap().unwrap();
        assert_eq!(e.pdf_content_hash, d.location.content_hash);
        assert!(s.get_screenshot(&id).is_ok());
        let count:i64=s.connection.query_row("SELECT count(*) FROM research_relations WHERE source_type='screenshot' AND source_id=?1 AND target_id=?2",params![id,e.location_id],|r|r.get(0)).unwrap();
        assert_eq!(count, 1);
    }
    #[test]
    fn malformed_bounds_hash_reference_quality_and_pixels_do_not_create_images() {
        let (_tmp, mut s, d, _) = fixture();
        let mut bad = d.clone();
        bad.location.rectangles[0].x = 900.0;
        assert!(s.create_pdf_excerpt("bad", &bad, &png()).is_err());
        bad = d.clone();
        bad.location.content_hash = "b".repeat(64);
        assert!(s.create_pdf_excerpt("bad", &bad, &png()).is_err());
        bad = d.clone();
        bad.literature_id = Some("f".repeat(32));
        assert!(s.create_pdf_excerpt("bad", &bad, &png()).is_err());
        bad = d.clone();
        bad.dpi = 600;
        assert!(s.create_pdf_excerpt("bad", &bad, &png()).is_err());
        bad = d.clone();
        bad.rotation = 45;
        assert!(s.create_pdf_excerpt("bad", &bad, &png()).is_err());
        bad = d.clone();
        bad.location.rectangles[0].width = 100.0;
        assert!(s.create_pdf_excerpt("bad", &bad, &png()).is_err());
        assert!(s.create_pdf_excerpt("bad", &d, b"not PNG").is_err());
        assert_eq!(s.library_status().unwrap().pending_writes, 0);
        assert!(s.list_screenshots(0, 50, false).unwrap().is_empty());
    }
    #[test]
    fn future_database_version_refuses_writes_without_reset() {
        let (tmp, s, _, _) = fixture();
        s.connection
            .pragma_update(None, "user_version", 18)
            .unwrap();
        drop(s);
        assert!(matches!(
            Storage::open(tmp.path().into()),
            Err(StorageError::NewerSchema(18))
        ));
        let c = Connection::open(tmp.path().join("researchclip.sqlite3")).unwrap();
        assert_eq!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            18
        );
    }
}
impl ExcerptDraft {
    pub fn validate(&self) -> Result<()> {
        ResearchData::PdfLocation(self.location.clone())
            .validate()
            .map_err(StorageError::Invalid)?;
        if self.location.rectangles.len() != 1
            || self.location.page_index >= 2000
            || ![144, 216, 288].contains(&self.dpi)
            || ![0, 90, 180, 270].contains(&self.rotation)
            || self.location.page_width > 20000.0
            || self.location.page_height > 20000.0
        {
            return Err(StorageError::Invalid("PDF摘录位置、清晰度或旋转无效。"));
        }
        if let Some(id) = &self.literature_id {
            check_id(id)?;
        }
        let r = &self.location.rectangles[0];
        let scale = self.dpi as f64 / 72.0;
        let (w, h) = if self.rotation.is_multiple_of(180) {
            (r.width * scale, r.height * scale)
        } else {
            (r.height * scale, r.width * scale)
        };
        if w < 1.0
            || h < 1.0
            || w.ceil() > 8192.0
            || h.ceil() > 8192.0
            || w.ceil() * h.ceil() > 16_777_216.0
        {
            return Err(StorageError::Invalid(
                "摘录超过图片预算，请降低DPI或缩小选区。",
            ));
        }
        Ok(())
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfExcerpt {
    pub screenshot_id: String,
    pub literature_id: Option<String>,
    pub pdf_attachment_id: String,
    pub pdf_content_hash: String,
    pub page_index: u32,
    pub page_rectangle: crate::domain::research::PageRectangle,
    pub image_asset_path: String,
    pub created_at: i64,
    pub location_id: String,
    pub document_id: String,
    pub dpi: u32,
    pub rotation: u32,
}
pub(super) fn commit_extra(conn: &Connection, id: &str) -> Result<()> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT draft_json FROM pending_pdf_excerpts WHERE screenshot_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(raw) = raw else { return Ok(()) };
    let draft: ExcerptDraft = serde_json::from_str(&raw)
        .map_err(|_| StorageError::Invalid("待保存PDF摘录元数据损坏，文件保留。"))?;
    draft.validate()?;
    let key = insert_resource(
        conn,
        "PDF摘录位置",
        &ResearchData::PdfLocation(draft.location.clone()),
    )?;
    conn.execute("INSERT INTO pdf_excerpts(screenshot_id,location_id,literature_id,dpi,rotation) VALUES(?1,?2,?3,?4,?5)",params![id,key.id,draft.literature_id,draft.dpi,draft.rotation])?;
    conn.execute(
        "INSERT INTO research_relations VALUES('screenshot',?1,'pdfLocation',?2,'source','manual')",
        params![id, key.id],
    )?;
    if let Some(lit) = &draft.literature_id {
        conn.execute(
            "INSERT INTO screenshot_references(screenshot_id,reference_id) VALUES(?1,?2)",
            params![id, lit],
        )?;
    }
    let stamp = now()?;
    let note_id: String = conn.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;
    let link = format!("researchclip://pdf-location/{}", key.id);
    let attribution = draft
        .literature_id
        .as_ref()
        .map(|id| format!("[查看来源文献](researchclip://literature/{id})（待人工核对）"))
        .unwrap_or_else(|| "尚未关联文献".into());
    let content = format!(
        "注释：\n\n[返回 PDF 第{}页摘录位置]({link})\n\n来源：{} · {} DPI",
        draft.location.page_index + 1,
        attribution,
        draft.dpi
    );
    conn.execute("INSERT INTO notes(id,screenshot_id,content,source_url,created_at,updated_at) VALUES(?1,?2,?3,NULL,?4,?4)",params![note_id,id,content,stamp])?;
    Ok(())
}
impl Storage {
    pub fn create_pdf_excerpt(
        &mut self,
        title: &str,
        draft: &ExcerptDraft,
        png: &[u8],
    ) -> Result<PdfExcerpt> {
        draft.validate()?;
        let attachment = self.research_resource(&ResourceKey {
            kind: ResourceKind::PdfAttachment,
            id: draft.location.attachment_id.clone(),
        })?;
        let ResearchData::PdfAttachment(a) = attachment.data()? else {
            return Err(StorageError::Invalid("附件类型无效。"));
        };
        if a.content_hash != draft.location.content_hash {
            return Err(StorageError::Conflict);
        }
        if let Some(lit) = &draft.literature_id {
            self.research_resource(&ResourceKey {
                kind: ResourceKind::Literature,
                id: lit.clone(),
            })?;
        }
        let frame = crate::services::workflow::decode_png(png)
            .map_err(|_| StorageError::Invalid("摘录PNG无效。"))?;
        let r = &draft.location.rectangles[0];
        let scale = draft.dpi as f64 / 72.0;
        let (w, h) = if draft.rotation.is_multiple_of(180) {
            (r.width * scale, r.height * scale)
        } else {
            (r.height * scale, r.width * scale)
        };
        if frame.width_px > 8192
            || frame.height_px > 8192
            || (frame.width_px as f64 - w).abs() > 2.0
            || (frame.height_px as f64 - h).abs() > 2.0
        {
            return Err(StorageError::Invalid("摘录像素尺寸与页面选区不符。"));
        }
        let doc = serde_json::json!({"schemaVersion":1,"image":{"widthPx":frame.width_px,"heightPx":frame.height_px},"objects":[]});
        let id = self.new_id()?;
        self.create_with_extra(&id, title, doc, png, Some(draft))?;
        self.pdf_excerpt(&id)?.ok_or(StorageError::NotFound)
    }
    pub fn pdf_excerpt(&self, id: &str) -> Result<Option<PdfExcerpt>> {
        check_id(id)?;
        let row:Option<(String,Option<String>,u32,u32)>=self.connection.query_row("SELECT location_id,literature_id,dpi,rotation FROM pdf_excerpts WHERE screenshot_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
        let Some((location_id, literature_id, dpi, rotation)) = row else {
            return Ok(None);
        };
        let shot = self.get_screenshot(id)?;
        let location = self.research_resource(&ResourceKey {
            kind: ResourceKind::PdfLocation,
            id: location_id.clone(),
        })?;
        let ResearchData::PdfLocation(loc) = location.data()? else {
            return Err(StorageError::Invalid("摘录位置损坏。"));
        };
        let attachment = self.research_resource(&ResourceKey {
            kind: ResourceKind::PdfAttachment,
            id: loc.attachment_id.clone(),
        })?;
        let ResearchData::PdfAttachment(a) = attachment.data()? else {
            return Err(StorageError::Invalid("摘录附件损坏。"));
        };
        Ok(Some(PdfExcerpt {
            screenshot_id: id.into(),
            literature_id,
            pdf_attachment_id: loc.attachment_id,
            pdf_content_hash: loc.content_hash,
            page_index: loc.page_index,
            page_rectangle: loc
                .rectangles
                .into_iter()
                .next()
                .ok_or(StorageError::Invalid("摘录区域缺失。"))?,
            image_asset_path: shot.original_image_path,
            created_at: shot.created_at,
            location_id,
            document_id: a.document_id,
            dpi,
            rotation,
        }))
    }
    pub fn pdf_excerpts(&self, document_id: &str) -> Result<Vec<PdfExcerpt>> {
        let record = self.pdf_record(document_id)?;
        let mut query=self.connection.prepare("SELECT e.screenshot_id FROM pdf_excerpts e JOIN research_entities r ON r.resource_type='pdfLocation' AND r.id=e.location_id JOIN screenshots s ON s.id=e.screenshot_id WHERE json_extract(r.data_json,'$.value.attachmentId')=?1 ORDER BY s.created_at DESC,s.id DESC LIMIT 100")?;
        let ids = query
            .query_map([record.attachment_id], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| self.pdf_excerpt(id)?.ok_or(StorageError::NotFound))
            .collect()
    }
}
