use super::{
    Storage, StorageError,
    library::{check_id, now},
};
use crate::services::editor::{MAX_DOCUMENT_BYTES, validate_document};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationRevision {
    pub revision_id: String,
    pub screenshot_id: String,
    pub version_number: i64,
    pub parent_revision_id: Option<String>,
    pub created_at: i64,
    pub description: String,
    pub schema_version: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionDocument {
    pub revision: AnnotationRevision,
    pub annotation_json: Value,
}
const COLUMNS: &str = "revision_id,screenshot_id,version_number,parent_revision_id,created_at,description,schema_version";
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<AnnotationRevision> {
    Ok(AnnotationRevision {
        revision_id: r.get(0)?,
        screenshot_id: r.get(1)?,
        version_number: r.get(2)?,
        parent_revision_id: r.get(3)?,
        created_at: r.get(4)?,
        description: r.get(5)?,
        schema_version: r.get(6)?,
    })
}
pub(crate) fn same_content(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_content(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| same_content(value, other)))
        }
        _ => a == b,
    }
}
pub(super) fn record(
    connection: &Connection,
    id: &str,
    doc: &Value,
    description: &str,
) -> Result<bool, StorageError> {
    super::revision_meta::validate_description(description)?;
    let latest=connection.query_row("SELECT revision_id,version_number,CASE WHEN length(CAST(annotation_json AS BLOB))<=4194304 THEN annotation_json ELSE '' END FROM annotation_revisions WHERE revision_id=(SELECT revision_id FROM annotation_heads WHERE screenshot_id=?1 AND needs_review=0)",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?))).optional()?;
    if latest.as_ref().is_some_and(|(_, _, json)| {
        serde_json::from_str::<Value>(json)
            .ok()
            .is_some_and(|old| same_content(&old, doc))
    }) {
        return Ok(false);
    }
    let parent = latest.map(|(id, _, _)| id);
    let number: i64 = connection.query_row(
        "SELECT coalesce(max(version_number),0)+1 FROM annotation_revisions WHERE screenshot_id=?1",
        [id],
        |r| r.get(0),
    )?;
    connection.execute(
        "INSERT INTO annotation_revisions VALUES (lower(hex(randomblob(16))),?1,?2,?3,?4,?5,?6,1)",
        params![
            id,
            number,
            parent,
            doc.to_string(),
            now()?,
            description.trim()
        ],
    )?;
    Ok(true)
}
impl Storage {
    pub fn annotation_history(
        &self,
        id: &str,
        before: i64,
        limit: u32,
    ) -> Result<Vec<AnnotationRevision>, StorageError> {
        check_id(id)?;
        if before < 1 || !(1..=50).contains(&limit) {
            return Err(StorageError::Invalid("版本分页参数无效。"));
        }
        // Metadata can still be listed when a single annotation JSON is damaged.
        if !self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM screenshots WHERE id=?1)",
            [id],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(StorageError::NotFound);
        }
        Ok(self.connection.prepare(&format!("SELECT {COLUMNS} FROM annotation_revisions WHERE screenshot_id=?1 AND version_number<?2 ORDER BY version_number DESC LIMIT ?3"))?.query_map(params![id,before,limit],row)?.collect::<rusqlite::Result<_>>()?)
    }
    pub fn annotation_revision(
        &self,
        id: &str,
        revision_id: &str,
    ) -> Result<RevisionDocument, StorageError> {
        check_id(id)?;
        check_id(revision_id)?;
        let (revision,json):(AnnotationRevision,String)=self.connection.query_row(&format!("SELECT {COLUMNS},CASE WHEN length(CAST(annotation_json AS BLOB))<=4194304 THEN annotation_json ELSE '' END FROM annotation_revisions WHERE screenshot_id=?1 AND revision_id=?2"),params![id,revision_id],|r|Ok((row(r)?,r.get(7)?))).optional()?.ok_or(StorageError::NotFound)?;
        if revision.schema_version != 1 || json.len() > MAX_DOCUMENT_BYTES {
            return Err(StorageError::Invalid(
                "该历史版本格式不支持或超过上限；原记录已保留。",
            ));
        }
        let doc: Value = serde_json::from_str(&json)
            .map_err(|_| StorageError::Invalid("该历史版本JSON损坏；其他版本和当前草稿未改动。"))?;
        validate_document(&doc)
            .map_err(|_| StorageError::Invalid("该历史版本标注校验失败；原记录已保留。"))?;
        let (w, h): (u32, u32) = self.connection.query_row(
            "SELECT width_px,height_px FROM screenshots WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if doc["image"]["widthPx"] != w || doc["image"]["heightPx"] != h {
            return Err(StorageError::Invalid("历史版本与原图尺寸不一致。"));
        }
        Ok(RevisionDocument {
            revision,
            annotation_json: doc,
        })
    }
    pub fn save_annotation_revision(
        &mut self,
        id: &str,
        expected: i64,
        doc: Value,
        description: &str,
    ) -> Result<super::library::Screenshot, StorageError> {
        let old = self.get_screenshot(id)?;
        if old.revision != expected {
            return Err(StorageError::Conflict);
        }
        validate_document(&doc).map_err(|_| StorageError::Invalid("标注数据无效，保存未执行。"))?;
        if doc["image"]["widthPx"] != old.width_px || doc["image"]["heightPx"] != old.height_px {
            return Err(StorageError::Invalid("标注尺寸与原图不一致。"));
        }
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let created = record(&tx, id, &doc, description)?;
        if created || !same_content(&doc, &old.annotation_data) {
            let n=tx.execute("UPDATE screenshots SET annotation_data=?1,updated_at=?2,revision=revision+1 WHERE id=?3 AND revision=?4",params![doc.to_string(),now()?,id,expected])?;
            if n != 1 {
                return Err(StorageError::Conflict);
            }
        }
        tx.commit()?;
        self.get_screenshot(id)
    }
    pub fn restore_annotation_revision(
        &mut self,
        id: &str,
        expected: i64,
        revision_id: &str,
        description: &str,
    ) -> Result<super::library::Screenshot, StorageError> {
        let historical = self.annotation_revision(id, revision_id)?;
        self.save_annotation_revision(id, expected, historical.annotation_json, description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{capture::RawFrame, services::screen_capture::encode_png};
    fn png() -> Vec<u8> {
        encode_png(RawFrame {
            width_px: 32,
            height_px: 24,
            rgba: [20, 40, 60, 128].repeat(32 * 24),
        })
        .unwrap()
    }
    fn doc(text: &str) -> Value {
        serde_json::json!({"schemaVersion":1,"image":{"widthPx":32,"heightPx":24},"objects":[{"id":"text","type":"text","x":2,"y":3,"text":text,"fontSize":12,"color":"#abcdef","strokeWidth":2}]})
    }
    #[test]
    fn numeric_spelling_dedup_and_advanced_objects_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let initial = doc("数学");
        let a = s.create_screenshot("A", initial.clone(), &png()).unwrap();
        let mut equivalent = initial.clone();
        equivalent["objects"][0]["x"] = serde_json::json!(2.0);
        assert_eq!(
            s.save_annotation_revision(&a.id, 1, equivalent, "")
                .unwrap()
                .revision,
            1
        );
        let mut advanced = initial;
        advanced["objects"].as_array_mut().unwrap().extend([
            serde_json::json!({"id":"m","type":"mosaic","x":0,"y":0,"width":10,"height":10,"strength":8,"color":"#ffffff","strokeWidth":2}),
            serde_json::json!({"id":"b","type":"blur","x":10,"y":10,"width":10,"height":10,"strength":4,"color":"#ffffff","strokeWidth":2}),
            serde_json::json!({"id":"n","type":"number","x":1,"y":1,"value":3,"size":20,"color":"#ff0000","strokeWidth":2}),
            serde_json::json!({"id":"a","type":"arrow","x":1,"y":1,"points":[0,0,10,10],"color":"#ff0000","strokeWidth":2})]);
        s.save_annotation_revision(&a.id, 1, advanced.clone(), "高级工具")
            .unwrap();
        let head = s.annotation_history(&a.id, i64::MAX, 1).unwrap();
        assert_eq!(
            s.annotation_revision(&a.id, &head[0].revision_id)
                .unwrap()
                .annotation_json,
            advanced
        );
    }
    #[test]
    fn explicit_versions_preview_restore_continue_reopen_and_original_integrity() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let original = png();
        let a = s
            .create_screenshot("版本测试", doc("初始"), &original)
            .unwrap();
        assert_eq!(a.annotation_version, 1);
        let v1 = s.annotation_history(&a.id, i64::MAX, 50).unwrap()[0].clone();
        let b = s
            .save_annotation_revision(&a.id, a.revision, doc("修改"), "说明中文")
            .unwrap();
        assert_eq!(b.annotation_version, 2);
        assert_eq!(
            s.annotation_revision(&a.id, &v1.revision_id)
                .unwrap()
                .annotation_json,
            doc("初始")
        );
        assert_eq!(
            s.get_screenshot(&a.id).unwrap().annotation_data,
            doc("修改")
        );
        let restored = s
            .restore_annotation_revision(&a.id, b.revision, &v1.revision_id, "恢复")
            .unwrap();
        assert_eq!(restored.annotation_version, 3);
        let versions = s.annotation_history(&a.id, i64::MAX, 50).unwrap();
        assert_eq!(
            versions[0].parent_revision_id.as_deref(),
            Some(versions[1].revision_id.as_str())
        );
        let final_item = s
            .save_annotation_revision(&a.id, restored.revision, doc("继续编辑"), "继续")
            .unwrap();
        assert_eq!(final_item.annotation_version, 4);
        drop(s);
        let s = Storage::open(tmp.path().into()).unwrap();
        assert_eq!(s.annotation_history(&a.id, i64::MAX, 50).unwrap().len(), 4);
        assert_eq!(
            s.get_screenshot(&a.id).unwrap().annotation_data,
            doc("继续编辑")
        );
        assert_eq!(s.screenshot_image(&a.id, false).unwrap(), original);
        assert_eq!(
            std::fs::read_dir(tmp.path().join("originals"))
                .unwrap()
                .count(),
            1
        );
    }
    #[test]
    fn identical_content_dedup_metadata_changes_and_stale_writes() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = s.create_screenshot("A", doc("A"), &png()).unwrap();
        let same = s
            .save_annotation_revision(&a.id, 1, doc("A"), "不创建冗余版本")
            .unwrap();
        assert_eq!(same.revision, 1);
        let b = s
            .update_screenshot(
                &a.id,
                1,
                super::super::library::ScreenshotPatch {
                    title: Some("B".into()),
                    is_favorite: Some(true),
                    annotation_data: None,
                },
            )
            .unwrap();
        assert_eq!(b.annotation_version, 1);
        assert!(matches!(
            s.save_annotation_revision(&a.id, 1, doc("B"), "冲突"),
            Err(StorageError::Conflict)
        ));
        assert!(
            s.save_annotation_revision(&a.id, b.revision, doc("B"), &"长".repeat(501))
                .is_err()
        );
        assert_eq!(s.annotation_history(&a.id, i64::MAX, 50).unwrap().len(), 1);
        let b = s
            .update_screenshot(
                &a.id,
                b.revision,
                super::super::library::ScreenshotPatch {
                    title: None,
                    is_favorite: None,
                    annotation_data: Some(doc("B")),
                },
            )
            .unwrap();
        assert_eq!(b.annotation_version, 2);
    }
    #[test]
    fn corrupted_future_cross_image_and_dimension_mismatch_are_isolated() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = s.create_screenshot("A", doc("A"), &png()).unwrap();
        let b = s.create_screenshot("B", doc("B"), &png()).unwrap();
        let first = s.annotation_history(&a.id, i64::MAX, 50).unwrap()[0].clone();
        let current = s.save_annotation_revision(&a.id, 1, doc("新"), "").unwrap();
        for json in ["{broken", r#"{"schemaVersion":999}"#] {
            s.connection
                .execute(
                    "UPDATE annotation_revisions SET annotation_json=?1 WHERE revision_id=?2",
                    params![json, first.revision_id],
                )
                .unwrap();
            assert!(s.annotation_revision(&a.id, &first.revision_id).is_err());
            assert!(
                s.restore_annotation_revision(&a.id, current.revision, &first.revision_id, "")
                    .is_err()
            );
        }
        assert!(s.annotation_revision(&b.id, &first.revision_id).is_err());
        assert_eq!(s.annotation_history(&a.id, i64::MAX, 50).unwrap().len(), 2);
        assert_eq!(s.get_screenshot(&a.id).unwrap().annotation_data, doc("新"));
        let mut bad = doc("A");
        bad["image"]["widthPx"] = 33.into();
        assert!(
            s.save_annotation_revision(&a.id, current.revision, bad, "")
                .is_err()
        );
        assert!(s.annotation_history(&a.id, 0, 50).is_err());
        assert!(s.annotation_history(&a.id, i64::MAX, 51).is_err());
        assert_eq!(
            s.annotation_history(&a.id, 2, 50).unwrap()[0].version_number,
            1
        );
    }
    #[test]
    fn failed_update_rolls_back_new_revision_and_delete_cascades() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = s.create_screenshot("A", doc("A"), &png()).unwrap();
        s.connection.execute_batch("CREATE TRIGGER fail_revision BEFORE UPDATE ON screenshots BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        assert!(s.save_annotation_revision(&a.id, 1, doc("B"), "").is_err());
        assert_eq!(s.annotation_history(&a.id, i64::MAX, 50).unwrap().len(), 1);
        s.connection
            .execute_batch("DROP TRIGGER fail_revision;")
            .unwrap();
        let a = s.save_annotation_revision(&a.id, 1, doc("B"), "").unwrap();
        s.delete_screenshot(&a.id, a.revision).unwrap();
        assert_eq!(
            s.connection
                .query_row("SELECT count(*) FROM annotation_revisions", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn v4_upgrade_preserves_legacy_v1_annotations_and_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("researchclip.sqlite3");
        let mut conn = Connection::open(path).unwrap();
        for sql in [
            include_str!("../../migrations/001_workspace.sql"),
            include_str!("../../migrations/002_library.sql"),
            include_str!("../../migrations/003_projects.sql"),
            include_str!("../../migrations/004_ocr_index.sql"),
        ] {
            conn.execute_batch(sql).unwrap();
        }
        let id = "a".repeat(32);
        conn.execute("INSERT INTO screenshots(id,title,original_image_path,thumbnail_path,annotation_data,width_px,height_px,created_at,updated_at) VALUES(?1,'old',?2,?3,?4,32,24,1,2)",params![id,format!("originals/{id}.png"),format!("thumbnails/{id}.png"),doc("旧格式").to_string()]).unwrap();
        conn.pragma_update(None, "user_version", 4).unwrap();
        super::super::migrations::migrate(&mut conn, Some(tmp.path())).unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT annotation_json FROM annotation_revisions",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            doc("旧格式").to_string()
        );
        let backup = std::fs::read_dir(tmp.path().join("backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let old = Connection::open(backup).unwrap();
        assert_eq!(
            old.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            4
        );
        assert!(old.prepare("SELECT * FROM annotation_revisions").is_err());
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            crate::storage::migrations::SCHEMA_VERSION as i64
        );
    }
}
