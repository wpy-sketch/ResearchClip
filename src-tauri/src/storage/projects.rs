use super::{
    Storage, StorageError,
    library::LibrarySort,
    library::{check_id, now},
    search::{FILTER, LibraryFilter, fts_query, pattern},
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
pub const MAX_SELECTION: usize = 5000;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub image_count: i64,
    pub document_count: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectedScreenshot {
    pub id: String,
    pub revision: i64,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum BatchAction {
    Move {
        #[serde(rename = "projectId")]
        project_id: Option<String>,
    },
    Tags {
        names: Vec<String>,
    },
    Favorite {
        value: bool,
    },
    Delete {
        #[serde(rename = "confirmedCount")]
        confirmed_count: usize,
    },
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    pub affected: usize,
    pub pending_cleanup: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDocument {
    pub id: String,
    pub name: String,
    pub project_id: Option<String>,
    pub updated_at: i64,
    pub available: bool,
}
fn name(input: &str) -> Result<String, StorageError> {
    let n = input.trim();
    if n.is_empty() || n.chars().count() > 80 || n.chars().any(char::is_control) {
        return Err(StorageError::Invalid("项目名称须为1至80个字符。"));
    }
    Ok(n.into())
}
impl Storage {
    pub fn projects(&self) -> Result<Vec<Project>, StorageError> {
        Ok(self.connection.prepare("SELECT p.id,p.name,(SELECT count(*) FROM screenshot_projects s WHERE s.project_id=p.id),(SELECT count(*) FROM markdown_documents d WHERE d.project_id=p.id) FROM projects p ORDER BY p.created_at,p.id")?.query_map([],|r|Ok(Project{id:r.get(0)?,name:r.get(1)?,image_count:r.get(2)?,document_count:r.get(3)?}))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn save_project(&mut self, id: Option<&str>, input: &str) -> Result<(), StorageError> {
        let n = name(input)?;
        let time = now()?;
        if let Some(id) = id {
            check_id(id)?;
            if self.connection.execute(
                "UPDATE projects SET name=?1,normalized_name=?2,updated_at=?3 WHERE id=?4",
                params![n, n.to_ascii_lowercase(), time, id],
            )? != 1
            {
                return Err(StorageError::NotFound);
            }
        } else {
            let tx = self
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let count: i64 = tx.query_row("SELECT count(*) FROM projects", [], |r| r.get(0))?;
            if count >= 500 {
                return Err(StorageError::Invalid("当前最多500个项目。"));
            }
            tx.execute(
                "INSERT INTO projects VALUES(lower(hex(randomblob(16))),?1,?2,?3,?3)",
                params![n, n.to_ascii_lowercase(), time],
            )?;
            tx.commit()?;
        }
        Ok(())
    }
    pub fn delete_empty_project(&mut self, id: &str) -> Result<(), StorageError> {
        check_id(id)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count:i64=tx.query_row("SELECT (SELECT count(*) FROM screenshot_projects WHERE project_id=?1)+(SELECT count(*) FROM markdown_documents WHERE project_id=?1)",[id],|r|r.get(0))?;
        if count != 0 {
            return Err(StorageError::Invalid(
                "只能删除空项目；请先移出图片和文档。此操作不删除项目内资料。",
            ));
        }
        if tx.execute("DELETE FROM projects WHERE id=?1", [id])? != 1 {
            return Err(StorageError::NotFound);
        }
        tx.commit()?;
        Ok(())
    }
    pub fn select_screenshots(
        &self,
        favorites: bool,
        sort: LibrarySort,
        filter: &LibraryFilter,
    ) -> Result<Vec<SelectedScreenshot>, StorageError> {
        // Reuse validation and the exact same predicate as the paged list; never decode original images.
        self.search_screenshot_page(0, 24, favorites, sort, filter)?;
        let order = match sort {
            LibrarySort::CreatedAsc => "created_at,id",
            LibrarySort::CreatedDesc => "created_at DESC,id DESC",
        };
        let tags = serde_json::to_string(&filter.tag_ids)
            .map_err(|_| StorageError::Invalid("标签无效。"))?;
        let rows = self
            .connection
            .prepare(&format!(
                "SELECT id,revision FROM screenshots WHERE {FILTER} ORDER BY {order} LIMIT 5001"
            ))?
            .query_map(
                params![
                    favorites,
                    filter.favorite,
                    filter.query.trim(),
                    pattern(filter.query.trim()),
                    tags,
                    rusqlite::types::Null,
                    rusqlite::types::Null,
                    filter.project_id,
                    filter.unclassified,
                    filter.source.key(),
                    fts_query(filter.query.trim(), filter.source)
                ],
                |r| {
                    Ok(SelectedScreenshot {
                        id: r.get(0)?,
                        revision: r.get(1)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if rows.len() > MAX_SELECTION {
            return Err(StorageError::Invalid(
                "筛选结果超过5000张，请缩小筛选范围；没有进行部分全选。",
            ));
        }
        Ok(rows)
    }
    pub fn batch_screenshots(
        &mut self,
        items: &[SelectedScreenshot],
        action: BatchAction,
    ) -> Result<BatchResult, StorageError> {
        if items.is_empty() || items.len() > MAX_SELECTION {
            return Err(StorageError::Invalid("请选择1至5000张图片。"));
        }
        let mut seen = std::collections::HashSet::new();
        for item in items {
            check_id(&item.id)?;
            if item.revision < 1 || !seen.insert(&item.id) {
                return Err(StorageError::Invalid("选择重复或版本无效。"));
            }
        }
        if let BatchAction::Delete { confirmed_count } = action
            && confirmed_count != items.len()
        {
            return Err(StorageError::Invalid(
                "删除确认数量与当前选择不一致，未删除资料。",
            ));
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        for item in items {
            let current: Option<i64> = tx
                .query_row(
                    "SELECT revision FROM screenshots WHERE id=?1",
                    [&item.id],
                    |r| r.get(0),
                )
                .optional()?;
            if current != Some(item.revision) {
                return Err(StorageError::Conflict);
            }
        }
        let mut tag_ids = Vec::new();
        match &action {
            BatchAction::Move {
                project_id: Some(id),
            } => {
                check_id(id)?;
                if !tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM projects WHERE id=?1)",
                    [id],
                    |r| r.get::<_, bool>(0),
                )? {
                    return Err(StorageError::NotFound);
                }
            }
            BatchAction::Tags { names } => {
                if names.is_empty() || names.len() > 100 {
                    return Err(StorageError::Invalid("一次添加1至100个标签。"));
                }
                let mut unique = std::collections::HashSet::new();
                for n in names {
                    let n = n.trim();
                    if n.is_empty()
                        || n.chars().count() > 50
                        || n.chars().any(char::is_control)
                        || !unique.insert(n.to_ascii_lowercase())
                    {
                        return Err(StorageError::Invalid("标签无效、重复或超过50字符。"));
                    }
                    tx.execute("INSERT INTO tags VALUES(lower(hex(randomblob(16))),?1,?2) ON CONFLICT(normalized_name) DO NOTHING",params![n,n.to_ascii_lowercase()])?;
                    tag_ids.push(tx.query_row(
                        "SELECT id FROM tags WHERE normalized_name=?1",
                        [n.to_ascii_lowercase()],
                        |r| r.get::<_, String>(0),
                    )?);
                }
                if tx.query_row("SELECT count(*) FROM tags", [], |r| r.get::<_, i64>(0))? > 1000 {
                    return Err(StorageError::Invalid("全局标签超过1000个。"));
                }
            }
            _ => {}
        }
        let time = now()?;
        for item in items {
            match &action {
                BatchAction::Move { project_id } => {
                    tx.execute(
                        "DELETE FROM screenshot_projects WHERE screenshot_id=?1",
                        [&item.id],
                    )?;
                    if let Some(p) = project_id {
                        tx.execute(
                            "INSERT INTO screenshot_projects VALUES(?1,?2)",
                            params![item.id, p],
                        )?;
                    }
                }
                BatchAction::Tags { .. } => {
                    for tag in &tag_ids {
                        tx.execute(
                            "INSERT OR IGNORE INTO screenshot_tags VALUES(?1,?2)",
                            params![item.id, tag],
                        )?;
                    }
                    if tx.query_row(
                        "SELECT count(*) FROM screenshot_tags WHERE screenshot_id=?1",
                        [&item.id],
                        |r| r.get::<_, i64>(0),
                    )? > 100
                    {
                        return Err(StorageError::Invalid(
                            "某张图片合并后超过100个标签，整批未提交。",
                        ));
                    }
                }
                BatchAction::Favorite { value } => {
                    tx.execute(
                        "UPDATE screenshots SET is_favorite=?1 WHERE id=?2",
                        params![value, item.id],
                    )?;
                }
                BatchAction::Delete { .. } => {
                    let (a, b): (String, String) = tx.query_row(
                        "SELECT original_image_path,thumbnail_path FROM screenshots WHERE id=?1",
                        [&item.id],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )?;
                    if a != format!("originals/{}.png", item.id)
                        || b != format!("thumbnails/{}.png", item.id)
                    {
                        return Err(StorageError::Invalid("受管图片引用异常，整批删除已取消。"));
                    }
                    for path in [a, b] {
                        tx.execute(
                            "INSERT INTO pending_file_deletions VALUES(?1,?2)",
                            params![path, item.id],
                        )?;
                    }
                    tx.execute("DELETE FROM screenshots WHERE id=?1", [&item.id])?;
                }
            }
            if !matches!(action, BatchAction::Delete { .. }) {
                tx.execute(
                    "UPDATE screenshots SET revision=revision+1,updated_at=?1 WHERE id=?2",
                    params![time, item.id],
                )?;
            }
        }
        tx.commit()?;
        if matches!(action, BatchAction::Delete { .. }) && self.cleanup().is_err() {
            self.recovery_warnings
                .push("批量删除已提交；文件清理队列保留，可重试或重启恢复。".into());
        }
        Ok(BatchResult {
            affected: items.len(),
            pending_cleanup: self.library_status()?.pending_cleanup,
        })
    }
    pub fn index_document(
        &mut self,
        path: &Path,
        previous: Option<&Path>,
    ) -> Result<ProjectDocument, StorageError> {
        let canonical = path.canonicalize()?;
        let path = canonical
            .to_str()
            .ok_or(StorageError::Invalid("文档路径编码无效。"))?;
        let filename = Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or(StorageError::Invalid("文档名称无效。"))?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let previous = previous
            .map(|p| {
                let parent = p
                    .parent()
                    .ok_or(StorageError::Invalid("原文档路径无效。"))?
                    .canonicalize()?;
                Ok::<_, StorageError>(
                    parent.join(
                        p.file_name()
                            .ok_or(StorageError::Invalid("原文档名称无效。"))?,
                    ),
                )
            })
            .transpose()?;
        if let Some(old) = previous.as_deref().filter(|p| *p != Path::new(path)) {
            tx.execute(
                "UPDATE markdown_documents SET path=?1,name=?2,updated_at=?3 WHERE path=?4",
                params![path, filename, now()?, old.to_string_lossy()],
            )?;
        }
        tx.execute("INSERT INTO markdown_documents VALUES(lower(hex(randomblob(16))),?1,?2,NULL,?3,?3) ON CONFLICT(path) DO UPDATE SET name=excluded.name,updated_at=excluded.updated_at",params![path,filename,now()?])?;
        let id: String = tx.query_row(
            "SELECT id FROM markdown_documents WHERE path=?1",
            [path],
            |r| r.get(0),
        )?;
        tx.commit()?;
        Ok(self.connection.query_row(
            "SELECT id,name,project_id,updated_at FROM markdown_documents WHERE id=?1",
            [id],
            |r| {
                Ok(ProjectDocument {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    project_id: r.get(2)?,
                    updated_at: r.get(3)?,
                    available: Path::new(path).is_file()
                        && super::files::ordinary(Path::new(path)).is_ok(),
                })
            },
        )?)
    }
    pub fn project_documents(
        &self,
        project_id: Option<&str>,
        unclassified: bool,
    ) -> Result<Vec<ProjectDocument>, StorageError> {
        if let Some(id) = project_id {
            check_id(id)?;
        }
        let rows=self.connection.prepare("SELECT id,name,project_id,updated_at,path FROM markdown_documents WHERE (?1 IS NULL OR project_id=?1) AND (?2=0 OR project_id IS NULL) ORDER BY updated_at DESC,id DESC LIMIT 5001")?.query_map(params![project_id,unclassified],|r|{let path: String=r.get(4)?;Ok(ProjectDocument{id:r.get(0)?,name:r.get(1)?,project_id:r.get(2)?,updated_at:r.get(3)?,available:super::files::ordinary(Path::new(&path)).is_ok()&&Path::new(&path).is_file()})})?.collect::<rusqlite::Result<Vec<_>>>()?;
        if rows.len() > 5000 {
            return Err(StorageError::Invalid("文档超过5000个，请按项目筛选。"));
        }
        Ok(rows)
    }
    pub fn assign_document(
        &mut self,
        id: &str,
        project_id: Option<&str>,
    ) -> Result<(), StorageError> {
        check_id(id)?;
        if let Some(p) = project_id {
            check_id(p)?;
        }
        if self.connection.execute(
            "UPDATE markdown_documents SET project_id=?1,updated_at=?2 WHERE id=?3",
            params![project_id, now()?, id],
        )? != 1
        {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
    pub fn indexed_document_path(&self, id: &str) -> Result<PathBuf, StorageError> {
        check_id(id)?;
        self.connection
            .query_row(
                "SELECT path FROM markdown_documents WHERE id=?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .map(PathBuf::from)
            .ok_or(StorageError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{capture::RawFrame, services::screen_capture::encode_png};
    fn create(s: &mut Storage, title: &str) -> SelectedScreenshot {
        let png = encode_png(RawFrame {
            width_px: 2,
            height_px: 2,
            rgba: [1, 2, 3, 128].repeat(4),
        })
        .unwrap();
        let shot=s.create_screenshot(title,serde_json::json!({"schemaVersion":1,"image":{"widthPx":2,"heightPx":2},"objects":[]}),&png).unwrap();
        SelectedScreenshot {
            id: shot.id,
            revision: shot.revision,
        }
    }
    #[test]
    fn project_membership_counts_notes_paths_and_restart_are_preserved() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("资料");
        let mut s = Storage::open(root.clone()).unwrap();
        s.save_project(None, "  论文 Alpha  ").unwrap();
        let p = s.projects().unwrap().remove(0);
        s.save_project(Some(&p.id), "新名称").unwrap();
        assert!(s.save_project(None, "新名称").is_err());
        let shot = create(&mut s, "中文 % 图片");
        let before = s.get_screenshot(&shot.id).unwrap();
        s.save_note(
            &shot.id,
            None,
            None,
            "笔记保持",
            Some("https://example.com"),
        )
        .unwrap();
        let bytes = std::fs::read(root.join(&before.original_image_path)).unwrap();
        s.batch_screenshots(
            std::slice::from_ref(&shot),
            BatchAction::Move {
                project_id: Some(p.id.clone()),
            },
        )
        .unwrap();
        assert!(s.delete_empty_project(&p.id).is_err());
        assert_eq!(s.projects().unwrap()[0].image_count, 1);
        let filter = LibraryFilter {
            project_id: Some(p.id.clone()),
            ..Default::default()
        };
        let selected = s
            .select_screenshots(false, LibrarySort::CreatedDesc, &filter)
            .unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(
            s.search_screenshot_page(
                0,
                24,
                false,
                LibrarySort::CreatedDesc,
                &LibraryFilter {
                    unclassified: true,
                    ..Default::default()
                }
            )
            .unwrap()
            .total,
            0
        );
        drop(s);
        let mut s = Storage::open(root.clone()).unwrap();
        let after = s.get_screenshot(&shot.id).unwrap();
        assert_eq!(before.original_image_path, after.original_image_path);
        assert_eq!(before.annotation_data, after.annotation_data);
        assert_eq!(
            bytes,
            std::fs::read(root.join(&after.original_image_path)).unwrap()
        );
        assert_eq!(s.list_notes(&shot.id).unwrap()[0].content, "笔记保持");
        s.batch_screenshots(&selected, BatchAction::Move { project_id: None })
            .unwrap();
        s.delete_empty_project(&p.id).unwrap();
        assert!(s.get_screenshot(&shot.id).is_ok());
    }
    #[test]
    fn batch_tags_add_not_replace_and_conflicts_roll_back_every_member() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = create(&mut s, "A");
        let b = create(&mut s, "B");
        s.batch_screenshots(
            &[a.clone(), b.clone()],
            BatchAction::Tags {
                names: vec!["共同".into(), "Alpha".into()],
            },
        )
        .unwrap();
        let rows = s
            .select_screenshots(false, LibrarySort::CreatedDesc, &LibraryFilter::default())
            .unwrap();
        let mut stale = rows.clone();
        stale[1].revision = 1;
        assert!(
            s.batch_screenshots(&stale, BatchAction::Favorite { value: true })
                .is_err()
        );
        for row in &rows {
            let card = s.knowledge_card(&row.id).unwrap();
            assert!(!card.screenshot.is_favorite);
            assert_eq!(card.tags.len(), 2);
        }
        s.batch_screenshots(
            &rows,
            BatchAction::Tags {
                names: vec!["第三个".into()],
            },
        )
        .unwrap();
        assert_eq!(s.screenshot_tags(&a.id).unwrap().len(), 3);
        assert_eq!(s.list_tags().unwrap().len(), 3);
        let updated = s
            .select_screenshots(false, LibrarySort::CreatedDesc, &LibraryFilter::default())
            .unwrap();
        assert!(
            s.batch_screenshots(
                &updated,
                BatchAction::Tags {
                    names: vec!["重复".into(), "重复".into()]
                }
            )
            .is_err()
        );
        assert_eq!(s.list_tags().unwrap().len(), 3);
    }
    #[test]
    fn delete_confirmation_conflict_and_durable_cleanup_are_safe() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("workspace");
        let mut s = Storage::open(root.clone()).unwrap();
        let a = create(&mut s, "A");
        let b = create(&mut s, "B");
        let rows = vec![a.clone(), b.clone()];
        assert!(
            s.batch_screenshots(&rows, BatchAction::Delete { confirmed_count: 1 })
                .is_err()
        );
        assert_eq!(s.list_screenshots(0, 50, false).unwrap().len(), 2);
        s.connection.execute_batch("CREATE TRIGGER fail_batch BEFORE DELETE ON screenshots WHEN OLD.title='B' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        assert!(
            s.batch_screenshots(&rows, BatchAction::Delete { confirmed_count: 2 })
                .is_err()
        );
        assert_eq!(s.list_screenshots(0, 50, false).unwrap().len(), 2);
        assert_eq!(s.library_status().unwrap().pending_cleanup, 0);
        assert!(root.join(format!("originals/{}.png", a.id)).is_file());
        s.connection
            .execute_batch("DROP TRIGGER fail_batch")
            .unwrap();
        let original = root.join(format!("originals/{}.png", a.id));
        std::fs::remove_file(&original).unwrap();
        std::fs::create_dir(&original).unwrap();
        let result = s
            .batch_screenshots(&rows, BatchAction::Delete { confirmed_count: 2 })
            .unwrap();
        assert_eq!(result.affected, 2);
        assert_eq!(result.pending_cleanup, 1);
        drop(s);
        std::fs::remove_dir(&original).unwrap();
        let s = Storage::open(root).unwrap();
        assert_eq!(s.library_status().unwrap().pending_cleanup, 0);
        assert!(s.list_screenshots(0, 50, false).unwrap().is_empty());
    }
    #[test]
    fn markdown_membership_rename_and_restart_do_not_change_file_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("workspace");
        let path = tmp.path().join("研究.md");
        std::fs::write(&path, "# 研究\n\n$\\alpha$ 中文").unwrap();
        let mut s = Storage::open(root.clone()).unwrap();
        s.save_project(None, "P").unwrap();
        let p = s.projects().unwrap().remove(0);
        let doc = s.index_document(&path, None).unwrap();
        s.assign_document(&doc.id, Some(&p.id)).unwrap();
        let new = tmp.path().join("改名.md");
        std::fs::rename(&path, &new).unwrap();
        let moved = s.index_document(&new, Some(&path)).unwrap();
        assert_eq!(doc.id, moved.id);
        assert_eq!(moved.project_id, Some(p.id.clone()));
        drop(s);
        let mut s = Storage::open(root).unwrap();
        assert_eq!(s.projects().unwrap()[0].document_count, 1);
        assert_eq!(
            s.project_documents(Some(&p.id), false).unwrap()[0].name,
            "改名.md"
        );
        assert_eq!(
            std::fs::read_to_string(&new).unwrap(),
            "# 研究\n\n$\\alpha$ 中文"
        );
        assert!(s.delete_empty_project(&p.id).is_err());
        s.assign_document(&doc.id, None).unwrap();
        s.delete_empty_project(&p.id).unwrap();
        assert!(new.is_file());
    }
    #[test]
    fn real_v2_upgrade_backs_up_and_keeps_existing_images_and_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("workspace");
        std::fs::create_dir(&root).unwrap();
        let path = root.join("researchclip.sqlite3");
        {
            let c = rusqlite::Connection::open(&path).unwrap();
            c.execute_batch(include_str!("../../migrations/001_workspace.sql"))
                .unwrap();
            c.execute_batch(include_str!("../../migrations/002_library.sql"))
                .unwrap();
            c.execute("INSERT INTO app_metadata VALUES('user_note','keep')", [])
                .unwrap();
            c.pragma_update(None, "user_version", 2).unwrap();
        }
        let s = Storage::open(root.clone()).unwrap();
        assert_eq!(s.status().unwrap().schema_version, 4);
        assert_eq!(
            s.connection
                .query_row(
                    "SELECT value FROM app_metadata WHERE key='user_note'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            "keep"
        );
        let backup = std::fs::read_dir(root.join("backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let c = rusqlite::Connection::open(backup).unwrap();
        assert_eq!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            2
        );
        assert!(s.projects().unwrap().is_empty());
    }
    #[test]
    fn failed_v3_migration_rolls_back_and_preserves_v2_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("researchclip.sqlite3");
        {
            let c = rusqlite::Connection::open(&path).unwrap();
            c.execute_batch(include_str!("../../migrations/001_workspace.sql"))
                .unwrap();
            c.execute_batch(include_str!("../../migrations/002_library.sql"))
                .unwrap();
            c.execute_batch("CREATE TABLE projects(user_data TEXT); INSERT INTO projects VALUES('keep'); PRAGMA user_version=2;").unwrap();
        }
        assert!(Storage::open(tmp.path().into()).is_err());
        let c = rusqlite::Connection::open(path).unwrap();
        assert_eq!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            c.query_row("SELECT user_data FROM projects", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "keep"
        );
        assert!(
            tmp.path()
                .join("backups")
                .read_dir()
                .unwrap()
                .next()
                .is_some()
        );
    }
    #[test]
    fn all_filter_selection_never_silently_truncates_over_5000() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Storage::open(tmp.path().into()).unwrap();
        s.connection.execute_batch(r#"WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i+1 FROM n WHERE i<5001)
        INSERT INTO screenshots SELECT printf('%032x',i),'科研', 'originals/'||printf('%032x',i)||'.png','thumbnails/'||printf('%032x',i)||'.png','{"schemaVersion":1,"image":{"widthPx":1,"heightPx":1},"objects":[]}',1,1,i,i,0,1 FROM n;"#).unwrap();
        assert!(
            s.select_screenshots(false, LibrarySort::CreatedDesc, &LibraryFilter::default())
                .is_err()
        );
        assert_eq!(
            s.search_screenshot_page(
                0,
                24,
                false,
                LibrarySort::CreatedDesc,
                &LibraryFilter::default()
            )
            .unwrap()
            .total,
            5001
        );
    }
}
