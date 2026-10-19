use super::{Storage, StorageError, files};
use crate::services::{editor::validate_document, workflow::decode_png};
use rusqlite::{OptionalExtension, Row, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub id: String,
    pub title: String,
    pub original_image_path: String,
    pub thumbnail_path: String,
    pub annotation_data: Value,
    pub annotation_version: i64,
    pub width_px: u32,
    pub height_px: u32,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_favorite: bool,
    pub revision: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{capture::RawFrame, services::screen_capture::encode_png};
    fn png() -> Vec<u8> {
        encode_png(RawFrame {
            width_px: 32,
            height_px: 24,
            rgba: [20, 40, 80, 128].repeat(32 * 24),
        })
        .unwrap()
    }
    fn doc() -> Value {
        serde_json::json!({"schemaVersion":1,"image":{"widthPx":32,"heightPx":24},"objects":[{"id":"r","type":"rectangle","x":2,"y":3,"width":20,"height":10,"color":"#ff0000","strokeWidth":2},{"id":"e","type":"ellipse","x":2,"y":3,"width":20,"height":10,"color":"#00ff00","strokeWidth":2},{"id":"a","type":"arrow","x":2,"y":3,"points":[1,2,20,10],"color":"#00ff00","strokeWidth":2},{"id":"p","type":"pen","x":2,"y":3,"points":[0,0,10,10,20,3],"color":"#0000ff","strokeWidth":3},{"id":"t","type":"text","x":2,"y":3,"text":"研究\nA","fontSize":12,"color":"#0000ff","strokeWidth":2}]})
    }
    fn empty_patch() -> ScreenshotPatch {
        ScreenshotPatch {
            title: None,
            is_favorite: None,
            annotation_data: None,
        }
    }
    #[test]
    fn original_thumbnail_and_vectors_survive_reopen_and_revision_conflict() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let bytes = png();
        let item = s.create_screenshot("O'Brien 科研", doc(), &bytes).unwrap();
        assert_eq!(s.screenshot_image(&item.id, false).unwrap(), bytes);
        let thumb = decode_png(&s.screenshot_image(&item.id, true).unwrap()).unwrap();
        assert_eq!((thumb.width_px, thumb.height_px), (32, 24));
        assert_eq!(&thumb.rgba[..4], &[20, 40, 80, 128]);
        let updated = s
            .update_screenshot(
                &item.id,
                1,
                ScreenshotPatch {
                    title: Some("新标题".into()),
                    is_favorite: Some(true),
                    annotation_data: Some(doc()),
                },
            )
            .unwrap();
        assert_eq!(updated.revision, 2);
        assert!(matches!(
            s.update_screenshot(&item.id, 1, empty_patch()),
            Err(StorageError::Conflict)
        ));
        assert_eq!(s.get_screenshot(&item.id).unwrap().title, "新标题");
        drop(s);
        let s = Storage::open(tmp.path().into()).unwrap();
        let loaded = s.get_screenshot(&item.id).unwrap();
        assert_eq!(loaded.annotation_data, doc());
        assert_eq!(loaded.revision, 2);
        assert!(loaded.is_favorite);
        assert_eq!(s.screenshot_image(&item.id, false).unwrap(), bytes);
        assert_eq!(s.list_screenshots(0, 50, true).unwrap().len(), 1);
    }
    #[test]
    fn library_pages_sort_by_creation_and_mutations_survive_reopen() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let mut ids = Vec::new();
        for n in 0..25 {
            let shot = s
                .create_screenshot(&format!("素材 {n}"), doc(), &png())
                .unwrap();
            s.connection
                .execute(
                    "UPDATE screenshots SET created_at=?1 WHERE id=?2",
                    params![n + 1, shot.id],
                )
                .unwrap();
            ids.push(shot.id);
        }
        let p = s
            .screenshot_page(0, 24, false, LibrarySort::CreatedDesc)
            .unwrap();
        assert_eq!(p.total, 25);
        assert_eq!(p.items.len(), 24);
        assert_eq!(p.items[0].id, ids[24]);
        let oldest = s
            .update_screenshot(
                &ids[0],
                1,
                ScreenshotPatch {
                    title: Some("O'Brien 科研".into()),
                    is_favorite: Some(true),
                    annotation_data: None,
                },
            )
            .unwrap();
        assert_eq!(
            s.screenshot_page(0, 24, false, LibrarySort::CreatedDesc)
                .unwrap()
                .items[0]
                .id,
            ids[24]
        );
        let last = s
            .screenshot_page(1, 24, false, LibrarySort::CreatedDesc)
            .unwrap();
        assert_eq!(last.items.len(), 1);
        assert_eq!(last.items[0].title, "O'Brien 科研");
        assert!(last.items[0].is_favorite);
        assert_eq!(last.items[0].revision, 2);
        assert_eq!(
            s.screenshot_page(0, 24, true, LibrarySort::CreatedAsc)
                .unwrap()
                .total,
            1
        );
        drop(s);
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let favorite = s
            .screenshot_page(0, 24, true, LibrarySort::CreatedAsc)
            .unwrap();
        assert_eq!(favorite.items[0].id, oldest.id);
        assert_eq!(favorite.items[0].title, "O'Brien 科研");
        s.delete_screenshot(&oldest.id, oldest.revision).unwrap();
        let clamped = s
            .screenshot_page(1, 24, false, LibrarySort::CreatedDesc)
            .unwrap();
        assert_eq!(
            (clamped.total, clamped.page, clamped.items.len()),
            (24, 0, 24)
        );
        assert!(!clamped.items.iter().any(|i| i.id == oldest.id));
        assert_eq!(
            s.screenshot_page(5, 24, true, LibrarySort::CreatedDesc)
                .unwrap()
                .total,
            0
        );
        assert!(!tmp.path().join(oldest.original_image_path).exists());
        assert!(!tmp.path().join(oldest.thumbnail_path).exists());
    }
    #[test]
    fn equal_creation_times_have_stable_reverse_order_across_pages() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let mut ids = Vec::new();
        for _ in 0..5 {
            ids.push(s.create_screenshot("A", doc(), &png()).unwrap().id);
        }
        s.connection
            .execute("UPDATE screenshots SET created_at=10", [])
            .unwrap();
        ids.sort();
        let ascending = s
            .screenshot_page(0, 3, false, LibrarySort::CreatedAsc)
            .unwrap();
        let tail = s
            .screenshot_page(1, 3, false, LibrarySort::CreatedAsc)
            .unwrap();
        assert_eq!(
            ascending
                .items
                .iter()
                .chain(tail.items.iter())
                .map(|i| &i.id)
                .collect::<Vec<_>>(),
            ids.iter().collect::<Vec<_>>()
        );
        assert_eq!(
            s.screenshot_page(0, 3, false, LibrarySort::CreatedDesc)
                .unwrap()
                .items[0]
                .id,
            ids[4]
        );
    }
    #[test]
    fn pagination_rejects_invalid_bounds_and_unknown_sort() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Storage::open(tmp.path().into()).unwrap();
        for (page, limit) in [(0, 0), (0, 51), (1_000_001, 24)] {
            assert!(
                s.screenshot_page(page, limit, false, LibrarySort::CreatedDesc)
                    .is_err()
            );
        }
        assert!(
            serde_json::from_str::<LibrarySort>("\"created_at; DELETE FROM screenshots\"").is_err()
        );
        let empty = s
            .screenshot_page(100, 24, false, LibrarySort::CreatedDesc)
            .unwrap();
        assert_eq!((empty.total, empty.page, empty.items.len()), (0, 0, 0));
    }
    #[test]
    fn damaged_images_and_future_vectors_do_not_break_other_library_previews() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = s.create_screenshot("损坏", doc(), &png()).unwrap();
        let b = s.create_screenshot("完好", doc(), &png()).unwrap();
        std::fs::write(tmp.path().join(&a.thumbnail_path), b"broken PNG").unwrap();
        std::fs::remove_file(tmp.path().join(&a.original_image_path)).unwrap();
        s.connection
            .execute(
                "UPDATE screenshots SET annotation_data=?1 WHERE id=?2",
                params!["{\"schemaVersion\":999}", b.id],
            )
            .unwrap();
        let page = s
            .screenshot_page(0, 24, false, LibrarySort::CreatedDesc)
            .unwrap();
        assert_eq!(page.total, 2);
        assert!(
            !page
                .items
                .iter()
                .find(|i| i.id == a.id)
                .unwrap()
                .image_available
        );
        assert!(s.screenshot_image(&a.id, true).is_err());
        assert!(s.screenshot_image(&b.id, true).is_ok());
        assert!(s.get_screenshot(&b.id).is_err());
        assert!(tmp.path().join(a.thumbnail_path).exists());
        assert_eq!(
            s.screenshot_page(0, 24, false, LibrarySort::CreatedDesc)
                .unwrap()
                .total,
            2
        );
    }
    #[test]
    fn notes_tags_parameter_binding_foreign_keys_and_delete_cascade() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = s.create_screenshot("A", doc(), &png()).unwrap();
        let b = s.create_screenshot("B", doc(), &png()).unwrap();
        let body = "'); DROP TABLE screenshots; --\n研究";
        let note = s
            .save_note(&a.id, None, None, body, Some("https://example.org/paper"))
            .unwrap();
        assert_eq!(note.content, body);
        let note2 = s
            .save_note(&a.id, Some(&note.id), Some(1), "更新", None)
            .unwrap();
        assert_eq!(note2.revision, 2);
        assert!(
            s.save_note(&a.id, Some(&note.id), Some(1), "旧", None)
                .is_err()
        );
        assert!(
            s.save_note(&a.id, None, None, "", Some("javascript:alert(1)"))
                .is_err()
        );
        let tag = s.save_tag(None, " O'Brien ").unwrap();
        assert!(s.save_tag(None, "o'brien").is_err());
        let a = s
            .set_screenshot_tags(&a.id, a.revision, std::slice::from_ref(&tag.id))
            .unwrap();
        assert_eq!(s.screenshot_tags(&a.id).unwrap().len(), 1);
        assert!(
            s.set_screenshot_tags(&a.id, a.revision, &["0".repeat(32)])
                .is_err()
        );
        assert_eq!(s.get_screenshot(&a.id).unwrap().revision, a.revision);
        assert_eq!(s.screenshot_tags(&a.id).unwrap()[0].tag_id, tag.id);
        s.delete_screenshot(&a.id, a.revision).unwrap();
        assert!(matches!(
            s.get_screenshot(&a.id),
            Err(StorageError::NotFound)
        ));
        assert_eq!(
            s.connection
                .query_row("SELECT count(*) FROM notes", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            s.connection
                .query_row("SELECT count(*) FROM screenshot_tags", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(!tmp.path().join(a.original_image_path).exists());
        assert!(!tmp.path().join(a.thumbnail_path).exists());
        assert_eq!(s.list_tags().unwrap().len(), 1);
        assert_eq!(s.screenshot_image(&b.id, false).unwrap(), png());
        s.delete_tag(&tag.id).unwrap();
        assert!(s.list_tags().unwrap().is_empty());
    }
    #[test]
    fn failed_database_commit_preserves_owned_files_and_recovers_on_restart() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let id = "a".repeat(32);
        s.connection.execute_batch("CREATE TRIGGER fail_create BEFORE INSERT ON screenshots BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        assert!(s.create_with_id(&id, "恢复", doc(), &png()).is_err());
        assert!(s.list_screenshots(0, 50, false).unwrap().is_empty());
        assert_eq!(s.library_status().unwrap().pending_writes, 1);
        assert_eq!(
            files::read(&s.data_dir, &format!("originals/{id}.png")).unwrap(),
            png()
        );
        s.connection
            .execute_batch("DROP TRIGGER fail_create;")
            .unwrap();
        drop(s);
        let s = Storage::open(tmp.path().into()).unwrap();
        assert_eq!(s.get_screenshot(&id).unwrap().annotation_data, doc());
        assert_eq!(s.library_status().unwrap().pending_writes, 0);
        assert_eq!(s.screenshot_image(&id, false).unwrap(), png());
    }
    #[test]
    fn crash_after_original_rebuilds_thumbnail_but_missing_or_future_data_is_retained() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Storage::open(tmp.path().into()).unwrap();
        let id = "b".repeat(32);
        let absent = "c".repeat(32);
        let future = "d".repeat(32);
        for (id, doc) in [
            (&id, doc()),
            (&absent, doc()),
            (&future, serde_json::json!({"schemaVersion":99})),
        ] {
            s.connection
                .execute(
                    "INSERT INTO pending_screenshots VALUES (?1,'恢复',?2,32,24,1)",
                    params![id, doc.to_string()],
                )
                .unwrap();
        }
        files::write(&s.data_dir, &format!("originals/{id}.png"), &png()).unwrap();
        files::write(&s.data_dir, &format!("originals/{future}.png"), &png()).unwrap();
        drop(s);
        let s = Storage::open(tmp.path().into()).unwrap();
        assert_eq!(s.get_screenshot(&id).unwrap().annotation_data, doc());
        assert!(s.screenshot_image(&id, true).is_ok());
        let status = s.library_status().unwrap();
        assert_eq!(status.pending_writes, 2);
        assert_eq!(status.warnings.len(), 2);
        assert_eq!(
            files::read(&s.data_dir, &format!("originals/{future}.png")).unwrap(),
            png()
        );
    }
    #[test]
    fn invalid_png_dimensions_version_and_file_conflicts_do_not_create_records() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let mut bad = doc();
        bad["schemaVersion"] = 99.into();
        assert!(s.create_screenshot("A", bad, &png()).is_err());
        let mut bad = doc();
        bad["image"]["widthPx"] = 33.into();
        assert!(s.create_screenshot("A", bad, &png()).is_err());
        assert!(s.create_screenshot("A", doc(), b"bad PNG").is_err());
        let id = "e".repeat(32);
        let path = tmp.path().join(format!("originals/{id}.png"));
        std::fs::write(&path, b"existing user data").unwrap();
        assert!(s.create_with_id(&id, "A", doc(), &png()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"existing user data");
        assert_eq!(s.library_status().unwrap().pending_writes, 0);
        assert!(s.list_screenshots(0, 50, false).unwrap().is_empty());
    }
    #[test]
    fn file_write_failure_is_reported_without_resetting_other_assets() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = s.create_screenshot("A", doc(), &png()).unwrap();
        std::fs::rename(
            tmp.path().join("originals"),
            tmp.path().join("held-originals"),
        )
        .unwrap();
        std::fs::write(tmp.path().join("originals"), b"blocked").unwrap();
        assert!(s.create_screenshot("B", doc(), &png()).is_err());
        assert_eq!(s.get_screenshot(&a.id).unwrap().title, "A");
        assert_eq!(
            std::fs::read(
                tmp.path()
                    .join("held-originals")
                    .join(format!("{}.png", a.id))
            )
            .unwrap(),
            png()
        );
    }
    #[test]
    fn cleanup_never_deletes_linked_assets_or_follows_untrusted_references() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = s.create_screenshot("A", doc(), &png()).unwrap();
        s.connection
            .execute(
                "INSERT INTO pending_file_deletions VALUES (?1,?2)",
                params![a.original_image_path, a.id],
            )
            .unwrap();
        let status = s.reconcile().unwrap();
        assert_eq!(status.pending_cleanup, 1);
        assert_eq!(s.screenshot_image(&a.id, false).unwrap(), png());
        let external = tmp.path().join("private.png");
        std::fs::write(&external, b"keep").unwrap();
        s.connection
            .execute(
                "UPDATE screenshots SET original_image_path='../private.png' WHERE id=?1",
                [&a.id],
            )
            .unwrap();
        assert!(s.screenshot_image(&a.id, false).is_err());
        assert!(s.delete_screenshot(&a.id, a.revision).is_err());
        assert_eq!(std::fs::read(external).unwrap(), b"keep");
    }
    #[test]
    fn deletion_cleanup_failure_remains_durable_and_is_retried_after_restart() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = s.create_screenshot("A", doc(), &png()).unwrap();
        let path = tmp.path().join(&a.original_image_path);
        // Only this disposable fixture is changed into a directory to provoke remove_file failure.
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        let result = s.delete_screenshot(&a.id, a.revision).unwrap();
        assert_eq!(result.pending_cleanup, 1);
        assert!(s.get_screenshot(&a.id).is_err());
        drop(s);
        let s = Storage::open(tmp.path().into()).unwrap();
        assert_eq!(s.library_status().unwrap().pending_cleanup, 1);
        assert!(path.is_dir());
        drop(s);
        std::fs::remove_dir(&path).unwrap();
        let s = Storage::open(tmp.path().into()).unwrap();
        assert_eq!(s.library_status().unwrap().pending_cleanup, 0);
    }
    #[test]
    fn thumbnail_bounds_and_transparency_are_preserved() {
        let frame = RawFrame {
            width_px: 1024,
            height_px: 8,
            rgba: [30, 90, 150, 64].repeat(1024 * 8),
        };
        let thumb = files::thumbnail_frame(&frame);
        assert_eq!((thumb.width_px, thumb.height_px), (512, 4));
        assert!(
            thumb
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| *p == [30, 90, 150, 64])
        );
        let transparent = files::thumbnail_frame(&RawFrame {
            width_px: 1,
            height_px: 1,
            rgba: vec![255, 200, 100, 0],
        });
        assert_eq!(transparent.rgba, [0, 0, 0, 0]);
    }
    #[test]
    fn cleanup_ack_failure_reports_committed_deletion_and_retains_recovery_queue() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = s.create_screenshot("A", doc(), &png()).unwrap();
        s.connection.execute_batch("CREATE TRIGGER fail_ack BEFORE DELETE ON pending_file_deletions BEGIN SELECT RAISE(ABORT,'test acknowledgement failure'); END;").unwrap();
        let deleted = s.delete_screenshot(&a.id, a.revision).unwrap();
        assert!(deleted.deleted);
        assert_eq!(deleted.pending_cleanup, 2);
        assert!(!s.library_status().unwrap().warnings.is_empty());
        assert!(s.get_screenshot(&a.id).is_err());
        s.connection
            .execute_batch("DROP TRIGGER fail_ack;")
            .unwrap();
        drop(s);
        let s = Storage::open(tmp.path().into()).unwrap();
        assert_eq!(s.library_status().unwrap().pending_cleanup, 0);
        assert!(!tmp.path().join(a.original_image_path).exists());
        assert!(!tmp.path().join(a.thumbnail_path).exists());
    }
    #[test]
    fn aborting_a_new_empty_save_never_deletes_a_committed_image_or_its_intent() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let id = "f".repeat(32);
        s.connection
            .execute(
                "INSERT INTO pending_screenshots VALUES (?1,'A',?2,32,24,1)",
                params![id, doc().to_string()],
            )
            .unwrap();
        s.clear_empty_pending(&id).unwrap();
        assert_eq!(s.library_status().unwrap().pending_writes, 0);
        s.connection
            .execute(
                "INSERT INTO pending_screenshots VALUES (?1,'A',?2,32,24,1)",
                params![id, doc().to_string()],
            )
            .unwrap();
        files::write(&s.data_dir, &format!("originals/{id}.png"), &png()).unwrap();
        s.clear_empty_pending(&id).unwrap();
        assert_eq!(s.library_status().unwrap().pending_writes, 1);
        assert_eq!(
            files::read(&s.data_dir, &format!("originals/{id}.png")).unwrap(),
            png()
        );
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotSummary {
    pub id: String,
    pub title: String,
    pub original_image_path: String,
    pub thumbnail_path: String,
    pub width_px: u32,
    pub height_px: u32,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_favorite: bool,
    pub revision: i64,
    pub image_available: bool,
    pub thumbnail_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_excerpt: Option<super::search::MatchExcerpt>,
}
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LibrarySort {
    CreatedDesc,
    CreatedAsc,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotPage {
    pub items: Vec<ScreenshotSummary>,
    pub total: u64,
    pub page: u32,
    pub limit: u32,
    pub sort: LibrarySort,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatus {
    pub schema_version: u32,
    pub pending_writes: i64,
    pub pending_cleanup: i64,
    pub warnings: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenshotPatch {
    pub title: Option<String>,
    pub is_favorite: Option<bool>,
    pub annotation_data: Option<Value>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub screenshot_id: String,
    pub content: String,
    pub source_url: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub revision: i64,
}
#[derive(Serialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotTag {
    pub screenshot_id: String,
    pub tag_id: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    pub deleted: bool,
    pub pending_cleanup: i64,
}
const COLUMNS: &str = "id,title,original_image_path,thumbnail_path,annotation_data,width_px,height_px,created_at,updated_at,is_favorite,revision";
pub(super) fn now() -> Result<i64, StorageError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StorageError::Invalid("系统时间无效。"))?
        .as_millis()
        .try_into()
        .map_err(|_| StorageError::Invalid("系统时间超出范围。"))
}
pub(super) fn check_id(id: &str) -> Result<(), StorageError> {
    if files::id_valid(id) {
        Ok(())
    } else {
        Err(StorageError::Invalid("资料标识无效。"))
    }
}
pub(super) fn title(value: &str) -> Result<String, StorageError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 200 || value.chars().any(char::is_control) {
        return Err(StorageError::Invalid(
            "标题须为1至200个字符，不能包含控制字符。",
        ));
    }
    Ok(value.into())
}
fn document(doc: &Value, width: u32, height: u32) -> Result<String, StorageError> {
    validate_document(doc)
        .map_err(|_| StorageError::Invalid("标注版本、图元或数据无效，现有资料未改动。"))?;
    if doc["image"]["widthPx"].as_u64() != Some(width.into())
        || doc["image"]["heightPx"].as_u64() != Some(height.into())
    {
        return Err(StorageError::Invalid("标注与原图尺寸不一致。"));
    }
    serde_json::to_string(doc).map_err(|_| StorageError::Invalid("标注JSON无法序列化。"))
}
fn row(r: &Row<'_>) -> rusqlite::Result<Screenshot> {
    let json: String = r.get(4)?;
    let annotation_data = serde_json::from_str(&json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(Screenshot {
        id: r.get(0)?,
        title: r.get(1)?,
        original_image_path: r.get(2)?,
        thumbnail_path: r.get(3)?,
        annotation_data,
        annotation_version: 0,
        width_px: r.get(5)?,
        height_px: r.get(6)?,
        created_at: r.get(7)?,
        updated_at: r.get(8)?,
        is_favorite: r.get(9)?,
        revision: r.get(10)?,
    })
}
impl Storage {
    pub(super) fn new_id(&self) -> Result<String, StorageError> {
        Ok(self
            .connection
            .query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?)
    }
    pub fn library_status(&self) -> Result<LibraryStatus, StorageError> {
        Ok(LibraryStatus {
            schema_version: 5,
            pending_writes: self.connection.query_row(
                "SELECT count(*) FROM pending_screenshots",
                [],
                |r| r.get(0),
            )?,
            pending_cleanup: self.connection.query_row(
                "SELECT count(*) FROM pending_file_deletions",
                [],
                |r| r.get(0),
            )?,
            warnings: self.recovery_warnings.clone(),
        })
    }
    pub fn get_screenshot(&self, id: &str) -> Result<Screenshot, StorageError> {
        check_id(id)?;
        let mut item = self
            .connection
            .query_row(
                &format!("SELECT {COLUMNS} FROM screenshots WHERE id=?1"),
                [id],
                row,
            )
            .optional()?
            .ok_or(StorageError::NotFound)?;
        item.annotation_version=self.connection.query_row("SELECT coalesce(max(version_number),0) FROM annotation_revisions WHERE screenshot_id=?1",[id],|r|r.get(0))?;
        self.validate_references(&item)?;
        document(&item.annotation_data, item.width_px, item.height_px)?;
        Ok(item)
    }
    fn validate_references(&self, item: &Screenshot) -> Result<(), StorageError> {
        check_id(&item.id)?;
        if item.original_image_path != format!("originals/{}.png", item.id)
            || item.thumbnail_path != format!("thumbnails/{}.png", item.id)
        {
            return Err(StorageError::Invalid("图片引用不属于该资料，已拒绝读写。"));
        }
        Ok(())
    }
    pub fn list_screenshots(
        &self,
        offset: u32,
        limit: u32,
        favorites: bool,
    ) -> Result<Vec<ScreenshotSummary>, StorageError> {
        if limit == 0 || limit > 50 || offset > 1_000_000 {
            return Err(StorageError::Invalid("分页参数无效。"));
        }
        self.list_summaries(
            &self.connection,
            offset,
            limit,
            favorites,
            "updated_at DESC,id",
        )
    }
    pub fn screenshot_page(
        &self,
        page: u32,
        limit: u32,
        favorites: bool,
        sort: LibrarySort,
    ) -> Result<ScreenshotPage, StorageError> {
        self.search_screenshot_page(
            page,
            limit,
            favorites,
            sort,
            &super::search::LibraryFilter::default(),
        )
    }
    fn list_summaries(
        &self,
        connection: &rusqlite::Connection,
        offset: u32,
        limit: u32,
        favorites: bool,
        order: &str,
    ) -> Result<Vec<ScreenshotSummary>, StorageError> {
        // Listing never loads annotation bodies; a damaged/future document remains visible.
        // order is an internal allowlist literal; all user values remain bound parameters.
        let mut stmt=connection.prepare(&format!("SELECT id,title,original_image_path,thumbnail_path,width_px,height_px,created_at,updated_at,is_favorite,revision FROM screenshots WHERE (?1=0 OR is_favorite=1) ORDER BY {order} LIMIT ?2 OFFSET ?3"))?;
        Ok(stmt
            .query_map(params![favorites, limit, offset], |r| self.summary_row(r))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn screenshot_image(&self, id: &str, thumbnail: bool) -> Result<Vec<u8>, StorageError> {
        if thumbnail {
            // Library previews need no annotation body or original pixels.
            check_id(id)?;
            let path: String = self
                .connection
                .query_row(
                    "SELECT thumbnail_path FROM screenshots WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or(StorageError::NotFound)?;
            if path != format!("thumbnails/{id}.png") {
                return Err(StorageError::Invalid("缩略图引用不属于该资料。"));
            }
            let bytes = files::read(&self.data_dir, &path)?;
            let frame = decode_png(&bytes)
                .map_err(|_| StorageError::Invalid("缩略图损坏，资料已保留。"))?;
            if frame.width_px.max(frame.height_px) > 512 {
                return Err(StorageError::Invalid("缩略图尺寸无效。"));
            }
            return Ok(bytes);
        }
        let item = self.get_screenshot(id)?;
        let bytes = files::read(
            &self.data_dir,
            if thumbnail {
                &item.thumbnail_path
            } else {
                &item.original_image_path
            },
        )?;
        let frame = decode_png(&bytes)
            .map_err(|_| StorageError::Invalid("图片损坏，资料未删除或覆盖。"))?;
        if (!thumbnail && (frame.width_px != item.width_px || frame.height_px != item.height_px))
            || (thumbnail && frame.width_px.max(frame.height_px) > 512)
        {
            return Err(StorageError::Invalid("图片实际尺寸与资料不一致。"));
        }
        Ok(bytes)
    }
    pub fn create_screenshot(
        &mut self,
        name: &str,
        doc: Value,
        png: &[u8],
    ) -> Result<Screenshot, StorageError> {
        let id = self.new_id()?;
        self.create_with_id(&id, name, doc, png)
    }
    fn create_with_id(
        &mut self,
        id: &str,
        name: &str,
        doc: Value,
        png: &[u8],
    ) -> Result<Screenshot, StorageError> {
        check_id(id)?;
        let name = title(name)?;
        if self.library_status()?.pending_writes >= 100 {
            return Err(StorageError::Invalid(
                "未完成保存过多，请先处理工作区恢复提示。",
            ));
        }
        let frame =
            decode_png(png).map_err(|_| StorageError::Invalid("原始PNG无效，资料尚未写入。"))?;
        let json = document(&doc, frame.width_px, frame.height_px)?;
        let thumbnail = crate::services::screen_capture::encode_png(files::thumbnail_frame(&frame))
            .map_err(|_| StorageError::Invalid("缩略图生成失败，资料尚未写入。"))?;
        let original = format!("originals/{id}.png");
        let thumb = format!("thumbnails/{id}.png");
        if files::image_path(&self.data_dir, &original)?.try_exists()?
            || files::image_path(&self.data_dir, &thumb)?.try_exists()?
        {
            return Err(StorageError::Invalid(
                "唯一图片文件名冲突，已有文件未覆盖。",
            ));
        }
        let stamp = now()?;
        self.connection.execute(
            "INSERT INTO pending_screenshots VALUES (?1,?2,?3,?4,?5,?6)",
            params![id, name, json, frame.width_px, frame.height_px, stamp],
        )?;
        if let Err(error) = files::write(&self.data_dir, &original, png) {
            self.clear_empty_pending(id)?;
            return Err(error);
        }
        files::write(&self.data_dir, &thumb, &thumbnail)?;
        self.commit_pending(id)?;
        self.get_screenshot(id)
    }
    fn commit_pending(&mut self, id: &str) -> Result<(), StorageError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted=tx.execute("INSERT INTO screenshots (id,title,original_image_path,thumbnail_path,annotation_data,width_px,height_px,created_at,updated_at) SELECT id,title,'originals/'||id||'.png','thumbnails/'||id||'.png',annotation_data,width_px,height_px,created_at,created_at FROM pending_screenshots WHERE id=?1",[id])?;
        if inserted != 1 {
            return Err(StorageError::NotFound);
        }
        tx.execute("DELETE FROM pending_screenshots WHERE id=?1", [id])?;
        tx.commit()?;
        Ok(())
    }
    fn clear_empty_pending(&mut self, id: &str) -> Result<(), StorageError> {
        // Only abort this newly requested save if neither image was ever committed.
        let empty = [
            format!("originals/{id}.png"),
            format!("thumbnails/{id}.png"),
        ]
        .iter()
        .all(|rel| {
            files::image_path(&self.data_dir, rel)
                .is_ok_and(|p| p.try_exists().is_ok_and(|exists| !exists))
        });
        if empty {
            self.connection
                .execute("DELETE FROM pending_screenshots WHERE id=?1", [id])?;
        }
        Ok(())
    }
    fn recover_pending(&mut self, id: &str) -> Result<(), StorageError> {
        check_id(id)?;
        let (json, w, h): (String, u32, u32) = self.connection.query_row(
            "SELECT annotation_data,width_px,height_px FROM pending_screenshots WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let doc: Value = serde_json::from_str(&json)
            .map_err(|_| StorageError::Invalid("未完成保存的标注无效，文件保留。"))?;
        document(&doc, w, h)?;
        let png = files::read(&self.data_dir, &format!("originals/{id}.png"))?;
        let frame =
            decode_png(&png).map_err(|_| StorageError::Invalid("待恢复原图损坏，文件保留。"))?;
        if frame.width_px != w || frame.height_px != h {
            return Err(StorageError::Invalid("待恢复原图尺寸不符，文件保留。"));
        }
        let expected = files::thumbnail_frame(&frame);
        let relative = format!("thumbnails/{id}.png");
        if files::image_path(&self.data_dir, &relative)?.try_exists()? {
            let actual = decode_png(&files::read(&self.data_dir, &relative)?)
                .map_err(|_| StorageError::Invalid("待恢复缩略图损坏，文件保留。"))?;
            if actual.width_px != expected.width_px
                || actual.height_px != expected.height_px
                || actual.rgba != expected.rgba
            {
                return Err(StorageError::Invalid("待恢复缩略图与原图不符，文件保留。"));
            }
        } else {
            files::write(&self.data_dir, &relative, &files::thumbnail(&png)?)?;
        }
        self.commit_pending(id)
    }
    pub fn reconcile(&mut self) -> Result<LibraryStatus, StorageError> {
        self.recovery_warnings.clear();
        let ids = self
            .connection
            .prepare("SELECT id FROM pending_screenshots ORDER BY created_at,id LIMIT 100")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for id in ids {
            if self.recover_pending(&id).is_err() {
                self.recovery_warnings.push("有未完成的截图保存无法自动恢复；原图与待处理记录已保留，请检查工作区权限、文件完整性与剩余空间。".into());
            }
        }
        self.cleanup()?;
        self.library_status()
    }
    pub fn update_screenshot(
        &mut self,
        id: &str,
        revision: i64,
        patch: ScreenshotPatch,
    ) -> Result<Screenshot, StorageError> {
        let old = self.get_screenshot(id)?;
        if revision != old.revision {
            return Err(StorageError::Conflict);
        }
        let name = patch
            .title
            .as_deref()
            .map(title)
            .transpose()?
            .unwrap_or(old.title);
        let has_annotation = patch.annotation_data.is_some();
        let doc = patch.annotation_data.unwrap_or(old.annotation_data);
        let json = document(&doc, old.width_px, old.height_px)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if has_annotation {
            super::revisions::record(&tx, id, &doc, "保存标注")?;
        }
        let n=tx.execute("UPDATE screenshots SET title=?1,is_favorite=?2,annotation_data=?3,updated_at=?4,revision=revision+1 WHERE id=?5 AND revision=?6",params![name,patch.is_favorite.unwrap_or(old.is_favorite),json,now()?,id,revision])?;
        if n != 1 {
            return Err(StorageError::Conflict);
        }
        tx.commit()?;
        self.get_screenshot(id)
    }
    pub fn delete_screenshot(
        &mut self,
        id: &str,
        revision: i64,
    ) -> Result<DeleteResult, StorageError> {
        let old = self.get_screenshot(id)?;
        if old.revision != revision {
            return Err(StorageError::Conflict);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let n = tx.execute(
            "DELETE FROM screenshots WHERE id=?1 AND revision=?2",
            params![id, revision],
        )?;
        if n != 1 {
            return Err(StorageError::Conflict);
        }
        for path in [&old.original_image_path, &old.thumbnail_path] {
            tx.execute(
                "INSERT INTO pending_file_deletions VALUES (?1,?2)",
                params![path, id],
            )?;
        }
        tx.commit()?;
        if self.cleanup().is_err() {
            self.recovery_warnings.push(
                "删除已提交，但文件清理或清理队列更新失败；剩余任务已保留，刷新或重启后可重试。"
                    .into(),
            );
        }
        Ok(DeleteResult {
            deleted: true,
            pending_cleanup: self.library_status()?.pending_cleanup,
        })
    }
    pub(super) fn cleanup(&mut self) -> Result<(), StorageError> {
        let jobs = self
            .connection
            .prepare("SELECT relative_path,screenshot_id FROM pending_file_deletions LIMIT 200")?
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (relative, id) in jobs {
            let linked:i64=self.connection.query_row("SELECT count(*) FROM screenshots WHERE id=?1 OR original_image_path=?2 OR thumbnail_path=?2",params![id,relative],|r|r.get(0))?;
            let safe = files::id_valid(&id)
                && [
                    format!("originals/{id}.png"),
                    format!("thumbnails/{id}.png"),
                ]
                .contains(&relative)
                && linked == 0;
            let result = if safe {
                files::image_path(&self.data_dir, &relative).and_then(
                    |p| match std::fs::remove_file(p) {
                        Ok(()) => Ok(()),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                        Err(e) => Err(e.into()),
                    },
                )
            } else {
                Err(StorageError::Invalid("清理引用无效。"))
            };
            if result.is_ok() {
                self.connection.execute(
                    "DELETE FROM pending_file_deletions WHERE relative_path=?1",
                    [relative],
                )?;
            } else {
                self.recovery_warnings.push(
                    "部分已删除资料的图片清理失败；清理任务已保留，可重试或在重启后继续。".into(),
                );
            }
        }
        Ok(())
    }
    pub fn list_notes(&self, screenshot_id: &str) -> Result<Vec<Note>, StorageError> {
        self.get_screenshot(screenshot_id)?;
        Ok(self.connection.prepare("SELECT id,screenshot_id,content,source_url,created_at,updated_at,revision FROM notes WHERE screenshot_id=?1 ORDER BY created_at,id LIMIT 100")?.query_map([screenshot_id],|r|Ok(Note{id:r.get(0)?,screenshot_id:r.get(1)?,content:r.get(2)?,source_url:r.get(3)?,created_at:r.get(4)?,updated_at:r.get(5)?,revision:r.get(6)?}))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn save_note(
        &mut self,
        screenshot_id: &str,
        id: Option<&str>,
        revision: Option<i64>,
        content: &str,
        source_url: Option<&str>,
    ) -> Result<Note, StorageError> {
        self.get_screenshot(screenshot_id)?;
        if content.len() > 1024 * 1024
            || source_url.is_some_and(|s| {
                s.len() > 2048
                    || !["https://", "http://"]
                        .iter()
                        .any(|p| s.starts_with(p) && s.len() > p.len())
                    || s.chars().any(char::is_whitespace)
            })
        {
            return Err(StorageError::Invalid(
                "笔记超过1 MiB或来源URL无效（仅支持http/https）。",
            ));
        }
        let stamp = now()?;
        let note_id = if let Some(id) = id {
            check_id(id)?;
            let n=self.connection.execute("UPDATE notes SET content=?1,source_url=?2,updated_at=?3,revision=revision+1 WHERE id=?4 AND screenshot_id=?5 AND revision=?6",params![content,source_url,stamp,id,screenshot_id,revision.ok_or(StorageError::Conflict)?])?;
            if n != 1 {
                return Err(StorageError::Conflict);
            }
            id.into()
        } else {
            if self.list_notes(screenshot_id)?.len() >= 100 {
                return Err(StorageError::Invalid("每张截图最多100条笔记。"));
            }
            let id = self.new_id()?;
            self.connection.execute(
                "INSERT INTO notes VALUES (?1,?2,?3,?4,?5,?5,1)",
                params![id, screenshot_id, content, source_url, stamp],
            )?;
            id
        };
        self.list_notes(screenshot_id)?
            .into_iter()
            .find(|n| n.id == note_id)
            .ok_or(StorageError::NotFound)
    }
    pub fn delete_note(&mut self, id: &str, revision: i64) -> Result<(), StorageError> {
        check_id(id)?;
        if self.connection.execute(
            "DELETE FROM notes WHERE id=?1 AND revision=?2",
            params![id, revision],
        )? != 1
        {
            return Err(StorageError::Conflict);
        }
        Ok(())
    }
    pub fn list_tags(&self) -> Result<Vec<Tag>, StorageError> {
        Ok(self
            .connection
            .prepare("SELECT id,name FROM tags ORDER BY normalized_name,id LIMIT 1000")?
            .query_map([], |r| {
                Ok(Tag {
                    id: r.get(0)?,
                    name: r.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn save_tag(&mut self, id: Option<&str>, name: &str) -> Result<Tag, StorageError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 50 || name.chars().any(char::is_control) {
            return Err(StorageError::Invalid("标签须为1至50个字符。"));
        }
        let normalized = name.to_ascii_lowercase();
        let id = if let Some(id) = id {
            check_id(id)?;
            if self.connection.execute(
                "UPDATE tags SET name=?1,normalized_name=?2 WHERE id=?3",
                params![name, normalized, id],
            )? != 1
            {
                return Err(StorageError::NotFound);
            }
            id.into()
        } else {
            if self.list_tags()?.len() >= 1000 {
                return Err(StorageError::Invalid("当前最多1000个标签。"));
            }
            let id = self.new_id()?;
            self.connection.execute(
                "INSERT INTO tags VALUES (?1,?2,?3)",
                params![id, name, normalized],
            )?;
            id
        };
        Ok(Tag {
            id,
            name: name.into(),
        })
    }
    pub fn delete_tag(&mut self, id: &str) -> Result<(), StorageError> {
        check_id(id)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("UPDATE screenshots SET revision=revision+1,updated_at=?1 WHERE id IN (SELECT screenshot_id FROM screenshot_tags WHERE tag_id=?2)",params![now()?,id])?;
        if tx.execute("DELETE FROM tags WHERE id=?1", [id])? != 1 {
            return Err(StorageError::NotFound);
        }
        tx.commit()?;
        Ok(())
    }
    pub fn screenshot_tags(&self, id: &str) -> Result<Vec<ScreenshotTag>, StorageError> {
        self.get_screenshot(id)?;
        Ok(self.connection.prepare("SELECT screenshot_id,tag_id FROM screenshot_tags WHERE screenshot_id=?1 ORDER BY tag_id")?.query_map([id],|r|Ok(ScreenshotTag{screenshot_id:r.get(0)?,tag_id:r.get(1)?}))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn set_screenshot_tags(
        &mut self,
        id: &str,
        revision: i64,
        tags: &[String],
    ) -> Result<Screenshot, StorageError> {
        self.get_screenshot(id)?;
        if tags.len() > 100 {
            return Err(StorageError::Invalid("每张截图最多100个标签。"));
        }
        let mut unique = std::collections::HashSet::new();
        for tag in tags {
            check_id(tag)?;
            if !unique.insert(tag) {
                return Err(StorageError::Invalid("标签关联重复。"));
            }
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute(
            "UPDATE screenshots SET revision=revision+1,updated_at=?1 WHERE id=?2 AND revision=?3",
            params![now()?, id, revision],
        )? != 1
        {
            return Err(StorageError::Conflict);
        }
        tx.execute("DELETE FROM screenshot_tags WHERE screenshot_id=?1", [id])?;
        for tag in tags {
            tx.execute(
                "INSERT INTO screenshot_tags VALUES (?1,?2)",
                params![id, tag],
            )?;
        }
        tx.commit()?;
        self.get_screenshot(id)
    }
}
