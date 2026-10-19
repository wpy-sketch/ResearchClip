use super::{
    Storage, StorageError,
    library::{check_id, now},
};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;

pub const ENGINE: &str = "Windows.Media.Ocr (system-managed; version not exposed)";
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrContent {
    pub screenshot_id: String,
    pub recognized_text: String,
    pub language: String,
    pub recognition_status: String,
    pub ocr_engine_version: String,
    pub updated_at: i64,
    pub source_image_revision: String,
    pub revision: i64,
    pub error: Option<String>,
    pub corrected: bool,
}
impl Storage {
    pub(super) fn recover_ocr(&self) -> Result<(), StorageError> {
        self.connection.execute("UPDATE ocr_content SET recognition_status='failed',error='上次识别被中断，请重试。',revision=revision+1 WHERE recognition_status='running'",[])?;
        Ok(())
    }
    pub fn ocr_content(&self, id: &str) -> Result<Option<OcrContent>, StorageError> {
        check_id(id)?;
        self.get_screenshot(id)?;
        Ok(self.connection.query_row("SELECT screenshot_id,recognized_text,language,recognition_status,ocr_engine_version,updated_at,source_image_revision,revision,error,corrected FROM ocr_content WHERE screenshot_id=?1",[id],|r|Ok(OcrContent{screenshot_id:r.get(0)?,recognized_text:r.get(1)?,language:r.get(2)?,recognition_status:r.get(3)?,ocr_engine_version:r.get(4)?,updated_at:r.get(5)?,source_image_revision:r.get(6)?,revision:r.get(7)?,error:r.get(8)?,corrected:r.get(9)?})).optional()?)
    }
    pub fn start_ocr(&self, id: &str, language: &str) -> Result<i64, StorageError> {
        check_id(id)?;
        self.get_screenshot(id)?;
        if language.is_empty()
            || language.len() > 64
            || !language
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return Err(StorageError::Invalid("OCR语言无效。"));
        }
        self.connection.execute("INSERT INTO ocr_content VALUES (?1,'',?2,'running',?3,?4,?5,1,NULL,0) ON CONFLICT(screenshot_id) DO UPDATE SET recognition_status='running',language=?2,updated_at=?4,error=NULL,revision=revision+1",params![id,language,ENGINE,now()?,format!("original-v1:{id}")])?;
        Ok(self
            .ocr_content(id)?
            .ok_or(StorageError::NotFound)?
            .revision)
    }
    pub fn finish_ocr(
        &self,
        id: &str,
        revision: i64,
        result: Result<&str, &str>,
    ) -> Result<(), StorageError> {
        let count = match result {
            Ok(text) => {
                validate_text(text)?;
                self.connection.execute("UPDATE ocr_content SET recognized_text=?1,recognition_status='completed',updated_at=?2,revision=revision+1,error=NULL,corrected=0 WHERE screenshot_id=?3 AND revision=?4 AND recognition_status='running'",params![text,now()?,id,revision])?
            }
            Err(message) => {
                if message.len() > 512 {
                    return Err(StorageError::Invalid("OCR错误提示过长。"));
                }
                self.connection.execute("UPDATE ocr_content SET recognition_status='failed',updated_at=?1,revision=revision+1,error=?2 WHERE screenshot_id=?3 AND revision=?4 AND recognition_status='running'",params![now()?,message,id,revision])?
            }
        };
        if count != 1 {
            return Err(StorageError::Conflict);
        }
        Ok(())
    }
    pub fn correct_ocr(
        &self,
        id: &str,
        revision: i64,
        text: &str,
    ) -> Result<OcrContent, StorageError> {
        check_id(id)?;
        validate_text(text)?;
        if self.connection.execute("UPDATE ocr_content SET recognized_text=?1,recognition_status='completed',updated_at=?2,revision=revision+1,error=NULL,corrected=1 WHERE screenshot_id=?3 AND revision=?4 AND recognition_status!='running'",params![text,now()?,id,revision])?!=1{return Err(StorageError::Conflict);}
        self.ocr_content(id)?.ok_or(StorageError::NotFound)
    }
    pub fn pending_ocr(&self, failed_only: bool) -> Result<Vec<String>, StorageError> {
        let mut s=self.connection.prepare("SELECT s.id FROM screenshots s LEFT JOIN ocr_content o ON o.screenshot_id=s.id WHERE (?1=0 AND o.screenshot_id IS NULL) OR o.recognition_status='failed' ORDER BY s.created_at,s.id LIMIT 5001")?;
        let ids = s
            .query_map([failed_only], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        if ids.len() > 5000 {
            return Err(StorageError::Invalid("一次OCR任务最多5000张，请分批处理。"));
        }
        Ok(ids)
    }
    pub fn rebuild_search(&mut self) -> Result<(), StorageError> {
        let tx = self.connection.transaction()?;
        tx.execute("DELETE FROM screenshot_search", [])?;
        tx.execute("INSERT INTO screenshot_search(rowid,screenshot_id,title,notes,tags,ocr) SELECT * FROM screenshot_search_source",[])?;
        tx.execute(
            "INSERT INTO screenshot_search(screenshot_search) VALUES('integrity-check')",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }
}
fn validate_text(text: &str) -> Result<(), StorageError> {
    if text.len() > 1_048_576 || text.contains('\0') {
        Err(StorageError::Invalid(
            "OCR文本须为不含空字符的文本，最多1 MiB。",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        capture::RawFrame,
        services::screen_capture::encode_png,
        storage::{
            library::LibrarySort,
            search::{LibraryFilter, SearchSource},
        },
    };
    fn add(s: &mut Storage) -> String {
        let png = encode_png(RawFrame {
            width_px: 2,
            height_px: 2,
            rgba: vec![255; 16],
        })
        .unwrap();
        s.create_screenshot(
            "截图",
            serde_json::json!({"schemaVersion":1,"image":{"widthPx":2,"heightPx":2},"objects":[]}),
            &png,
        )
        .unwrap()
        .id
    }
    fn count(s: &Storage, q: &str, source: SearchSource) -> u64 {
        s.search_screenshot_page(
            0,
            24,
            false,
            LibrarySort::CreatedDesc,
            &LibraryFilter {
                query: q.into(),
                source,
                ..Default::default()
            },
        )
        .unwrap()
        .total
    }
    #[test]
    fn actual_tokenizer_short_chinese_literal_fts_source_filter_and_rebuild() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().into()).unwrap();
        let id = add(&mut s);
        let enabled: i64 = s
            .connection
            .query_row("SELECT sqlite_compileoption_used('ENABLE_FTS5')", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(enabled, 1);
        let revision = s.start_ocr(&id, "zh-Hans-CN").unwrap();
        s.finish_ocr(
            &id,
            revision,
            Ok("科研控制理论 local Stability 中文English %_\\ \"quote\" OR NEAR(a)"),
        )
        .unwrap();
        for q in [
            "科研",
            "控",
            "控制理论",
            "stability",
            "中文English",
            "%_\\",
            "\"quote\"",
            "OR NEAR(a)",
        ] {
            assert_eq!(count(&s, q, SearchSource::Ocr), 1, "query {q}");
            assert_eq!(count(&s, q, SearchSource::Notes), 0);
        }
        assert_eq!(count(&s, "\" OR * - : (", SearchSource::All), 0);
        let page = s
            .search_screenshot_page(
                0,
                24,
                false,
                LibrarySort::CreatedDesc,
                &LibraryFilter {
                    query: "科研".into(),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(page.items[0].match_excerpt.as_ref().unwrap().source, "ocr");
        assert_eq!(
            s.select_screenshots(
                false,
                LibrarySort::CreatedDesc,
                &LibraryFilter {
                    query: "科研".into(),
                    source: SearchSource::Ocr,
                    ..Default::default()
                }
            )
            .unwrap()
            .len(),
            1
        );
        s.connection
            .execute("DELETE FROM screenshot_search", [])
            .unwrap();
        assert_eq!(count(&s, "Stability", SearchSource::All), 0);
        s.rebuild_search().unwrap();
        assert_eq!(count(&s, "Stability", SearchSource::All), 1);
    }
    #[test]
    fn corrections_are_separate_cas_durable_and_delete_cascades() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().into()).unwrap();
        let id = add(&mut s);
        let rev = s.start_ocr(&id, "zh-Hans-CN").unwrap();
        s.finish_ocr(&id, rev, Ok("识别旧文")).unwrap();
        let old = s.ocr_content(&id).unwrap().unwrap();
        let corrected = s
            .correct_ocr(&id, old.revision, "人工修正 Lyapunov")
            .unwrap();
        assert!(corrected.corrected);
        assert!(s.correct_ocr(&id, old.revision, "stale").is_err());
        assert_eq!(count(&s, "识别旧文", SearchSource::All), 0);
        assert_eq!(count(&s, "Lyapunov", SearchSource::All), 1);
        assert!(
            s.connection
                .query_row(
                    "SELECT count(*) FROM notes WHERE screenshot_id=?1",
                    [&id],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap()
                == 0
        );
        drop(s);
        let mut s = Storage::open(t.path().into()).unwrap();
        assert_eq!(
            s.ocr_content(&id).unwrap().unwrap().recognized_text,
            "人工修正 Lyapunov"
        );
        assert_eq!(count(&s, "人工", SearchSource::All), 1);
        let shot = s.get_screenshot(&id).unwrap();
        s.delete_screenshot(&id, shot.revision).unwrap();
        assert_eq!(count(&s, "Lyapunov", SearchSource::All), 0);
        assert_eq!(
            s.connection
                .query_row("SELECT count(*) FROM ocr_content", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn failed_and_interrupted_reruns_preserve_corrections_and_resume_missing_only() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().into()).unwrap();
        let id = add(&mut s);
        let other = add(&mut s);
        let rev = s.start_ocr(&id, "zh-Hans-CN").unwrap();
        s.finish_ocr(&id, rev, Ok("已保存的科研")).unwrap();
        assert_eq!(s.pending_ocr(false).unwrap(), vec![other.clone()]);
        let rev = s.start_ocr(&id, "zh-Hans-CN").unwrap();
        assert!(s.correct_ocr(&id, rev, "racing").is_err());
        s.finish_ocr(&id, rev, Err("取消")).unwrap();
        assert_eq!(
            s.ocr_content(&id).unwrap().unwrap().recognized_text,
            "已保存的科研"
        );
        s.start_ocr(&id, "zh-Hans-CN").unwrap();
        drop(s);
        let s = Storage::open(t.path().into()).unwrap();
        let r = s.ocr_content(&id).unwrap().unwrap();
        assert_eq!(r.recognition_status, "failed");
        assert_eq!(r.recognized_text, "已保存的科研");
        assert_eq!(s.pending_ocr(true).unwrap(), vec![id]);
        assert_eq!(s.pending_ocr(false).unwrap().len(), 2);
    }
    #[test]
    fn v3_upgrade_is_backed_up_and_keeps_original_rows() {
        let t = tempfile::tempdir().unwrap();
        let db = t.path().join("researchclip.sqlite3");
        let conn = rusqlite::Connection::open(&db).unwrap();
        for sql in [
            include_str!("../../migrations/001_workspace.sql"),
            include_str!("../../migrations/002_library.sql"),
            include_str!("../../migrations/003_projects.sql"),
        ] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 3).unwrap();
        conn.execute("INSERT INTO app_metadata VALUES ('keep','科研')", [])
            .unwrap();
        drop(conn);
        let s = Storage::open(t.path().into()).unwrap();
        assert_eq!(s.status().unwrap().schema_version, 6);
        assert_eq!(
            std::fs::read_dir(t.path().join("backups")).unwrap().count(),
            1
        );
        assert_eq!(
            s.connection
                .query_row("SELECT value FROM app_metadata WHERE key='keep'", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "科研"
        );
    }
}
