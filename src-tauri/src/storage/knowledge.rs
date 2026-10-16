use super::{
    Storage, StorageError,
    library::{Note, ScreenshotSummary, Tag, check_id, now, title},
    search::SUMMARY_COLUMNS,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeCard {
    pub screenshot: ScreenshotSummary,
    pub note: Option<Note>,
    pub tags: Vec<Tag>,
    pub additional_note_count: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardInput {
    pub id: String,
    pub revision: i64,
    pub title: String,
    pub is_favorite: bool,
    pub note_id: Option<String>,
    pub note_revision: Option<i64>,
    pub content: String,
    pub source_url: Option<String>,
    pub tag_names: Vec<String>,
}
pub fn source_url(input: Option<&str>) -> Result<Option<String>, StorageError> {
    let Some(value) = input else { return Ok(None) };
    if value.len() > 2048
        || value.chars().any(|c| c.is_whitespace() || c.is_control())
        || (!value.to_ascii_lowercase().starts_with("https://")
            && !value.to_ascii_lowercase().starts_with("http://"))
    {
        return Err(StorageError::Invalid(
            "来源链接仅支持有效的http/https URL，不能包含空白或账号密码。",
        ));
    }
    let url = tauri::Url::parse(value).map_err(|_| StorageError::Invalid("来源链接无效。"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(StorageError::Invalid(
            "来源链接仅支持有效的http/https URL，不能包含账号密码。",
        ));
    }
    let canonical = url.to_string();
    if canonical.len() > 2048 {
        return Err(StorageError::Invalid(
            "来源链接编码后超过2048字节，请使用更短的URL。",
        ));
    }
    Ok(Some(canonical))
}
fn primary_note(c: &Connection, id: &str) -> Result<Option<Note>, StorageError> {
    Ok(c.query_row("SELECT id,screenshot_id,content,source_url,created_at,updated_at,revision FROM notes WHERE screenshot_id=?1 ORDER BY created_at,id LIMIT 1",[id],|r|Ok(Note{id:r.get(0)?,screenshot_id:r.get(1)?,content:r.get(2)?,source_url:r.get(3)?,created_at:r.get(4)?,updated_at:r.get(5)?,revision:r.get(6)?})).optional()?)
}
fn random_id(c: &Connection) -> rusqlite::Result<String> {
    c.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))
}
impl Storage {
    pub fn knowledge_card(&self, id: &str) -> Result<KnowledgeCard, StorageError> {
        check_id(id)?;
        let tx = self.connection.unchecked_transaction()?;
        let card = self.knowledge_card_from(&tx, id)?;
        tx.commit()?;
        Ok(card)
    }
    pub(super) fn knowledge_card_from(
        &self,
        connection: &Connection,
        id: &str,
    ) -> Result<KnowledgeCard, StorageError> {
        let screenshot = connection
            .query_row(
                &format!("SELECT {SUMMARY_COLUMNS} FROM screenshots WHERE id=?1"),
                [id],
                |r| self.summary_row(r),
            )
            .optional()?
            .ok_or(StorageError::NotFound)?;
        let note = primary_note(connection, id)?;
        let count: u32 = connection.query_row(
            "SELECT count(*) FROM notes WHERE screenshot_id=?1",
            [id],
            |r| r.get(0),
        )?;
        let tags = {
            let mut stmt=connection.prepare("SELECT t.id,t.name FROM tags t JOIN screenshot_tags st ON t.id=st.tag_id WHERE st.screenshot_id=?1 ORDER BY t.normalized_name,t.id")?;
            stmt.query_map([id], |r| {
                Ok(Tag {
                    id: r.get(0)?,
                    name: r.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
        };
        Ok(KnowledgeCard {
            screenshot,
            note,
            tags,
            additional_note_count: count.saturating_sub(1),
        })
    }
    pub fn save_knowledge_card(&mut self, input: CardInput) -> Result<KnowledgeCard, StorageError> {
        check_id(&input.id)?;
        let name = title(&input.title)?;
        if input.content.len() > 1024 * 1024 || input.tag_names.len() > 100 {
            return Err(StorageError::Invalid(
                "笔记最多1 MiB，每张截图最多100个标签。",
            ));
        }
        let url = source_url(input.source_url.as_deref())?;
        let mut names = Vec::new();
        let mut unique = std::collections::HashSet::new();
        for name in &input.tag_names {
            let name = name.trim();
            if name.is_empty() || name.chars().count() > 50 || name.chars().any(char::is_control) {
                return Err(StorageError::Invalid("标签须为1至50个字符。"));
            }
            let normalized = name.to_ascii_lowercase();
            if !unique.insert(normalized.clone()) {
                return Err(StorageError::Invalid("不能重复添加同名标签。"));
            }
            names.push((name.to_string(), normalized));
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = primary_note(&tx, &input.id)?;
        if current.as_ref().map(|n| &n.id) != input.note_id.as_ref()
            || current.as_ref().map(|n| n.revision) != input.note_revision
        {
            return Err(StorageError::Conflict);
        }
        let stamp = now()?;
        if tx.execute("UPDATE screenshots SET title=?1,is_favorite=?2,updated_at=?3,revision=revision+1 WHERE id=?4 AND revision=?5",params![name,input.is_favorite,stamp,input.id,input.revision])?!=1{return Err(StorageError::Conflict);}
        if let Some(note) = current {
            if tx.execute("UPDATE notes SET content=?1,source_url=?2,updated_at=?3,revision=revision+1 WHERE id=?4 AND screenshot_id=?5 AND revision=?6",params![input.content,url,stamp,note.id,input.id,note.revision])?!=1{return Err(StorageError::Conflict);}
        } else {
            tx.execute(
                "INSERT INTO notes VALUES (?1,?2,?3,?4,?5,?5,1)",
                params![random_id(&tx)?, input.id, input.content, url, stamp],
            )?;
        }
        tx.execute(
            "DELETE FROM screenshot_tags WHERE screenshot_id=?1",
            [&input.id],
        )?;
        for (name, normalized) in names {
            let existing: Option<String> = tx
                .query_row(
                    "SELECT id FROM tags WHERE normalized_name=?1",
                    [&normalized],
                    |r| r.get(0),
                )
                .optional()?;
            let tag = if let Some(id) = existing {
                id
            } else {
                let count: u32 = tx.query_row("SELECT count(*) FROM tags", [], |r| r.get(0))?;
                if count >= 1000 {
                    return Err(StorageError::Invalid("当前最多1000个标签。"));
                }
                let id = random_id(&tx)?;
                tx.execute(
                    "INSERT INTO tags VALUES (?1,?2,?3)",
                    params![id, name, normalized],
                )?;
                id
            };
            tx.execute(
                "INSERT INTO screenshot_tags VALUES (?1,?2)",
                params![input.id, tag],
            )?;
        }
        tx.commit()?;
        self.knowledge_card(&input.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{capture::RawFrame, services::screen_capture::encode_png};
    fn create(s: &mut Storage) -> String {
        let png = encode_png(RawFrame {
            width_px: 2,
            height_px: 2,
            rgba: [1, 2, 3, 255].repeat(4),
        })
        .unwrap();
        s.create_screenshot(
            "科研图片",
            serde_json::json!({"schemaVersion":1,"image":{"widthPx":2,"heightPx":2},"objects":[]}),
            &png,
        )
        .unwrap()
        .id
    }
    fn input(card: &KnowledgeCard) -> CardInput {
        CardInput {
            id: card.screenshot.id.clone(),
            revision: card.screenshot.revision,
            title: "研究卡片".into(),
            is_favorite: true,
            note_id: card.note.as_ref().map(|n| n.id.clone()),
            note_revision: card.note.as_ref().map(|n| n.revision),
            content: "# 实验观察\n细胞与 Alpha\n<script>alert(1)</script>".into(),
            source_url: Some("https://example.org/paper".into()),
            tag_names: vec!["细胞".into(), "Method".into()],
        }
    }
    #[test]
    fn card_note_tags_and_source_survive_reopen_without_modifying_original() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let id = create(&mut s);
        let initial = s.knowledge_card(&id).unwrap();
        assert!(initial.note.is_none());
        let original = s.screenshot_image(&id, false).unwrap();
        let saved = s.save_knowledge_card(input(&initial)).unwrap();
        assert_eq!(
            saved.note.as_ref().unwrap().content,
            "# 实验观察\n细胞与 Alpha\n<script>alert(1)</script>"
        );
        assert_eq!(saved.tags.len(), 2);
        assert!(saved.screenshot.is_favorite);
        assert_eq!(saved.screenshot.revision, 2);
        assert_eq!(s.screenshot_image(&id, false).unwrap(), original);
        drop(s);
        let s = Storage::open(tmp.path().into()).unwrap();
        let loaded = s.knowledge_card(&id).unwrap();
        assert_eq!(
            loaded.note.unwrap().source_url.as_deref(),
            Some("https://example.org/paper")
        );
        assert_eq!(loaded.tags.len(), 2);
        assert_eq!(loaded.screenshot.title, "研究卡片");
        assert_eq!(s.screenshot_image(&id, false).unwrap(), original);
    }
    #[test]
    fn subsequent_saves_reuse_note_and_existing_tag_ids_and_keep_legacy_notes() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let id = create(&mut s);
        let first = s
            .save_knowledge_card(input(&s.knowledge_card(&id).unwrap()))
            .unwrap();
        let legacy = s
            .save_note(&id, None, None, "另一条既有笔记", None)
            .unwrap();
        // Explicit later creation avoids depending on clock resolution in this legacy fixture.
        s.connection
            .execute(
                "UPDATE notes SET created_at=?1 WHERE id=?2",
                params![first.note.as_ref().unwrap().created_at + 10, legacy.id],
            )
            .unwrap();
        let mut update = input(&first);
        update.content = "更新正文".into();
        update.tag_names = vec!["method".into()];
        let saved = s.save_knowledge_card(update).unwrap();
        assert_eq!(saved.note.as_ref().unwrap().id, first.note.unwrap().id);
        assert_eq!(saved.note.as_ref().unwrap().revision, 2);
        assert_eq!(saved.additional_note_count, 1);
        assert_eq!(saved.tags[0].name, "Method");
        assert_eq!(
            s.list_notes(&id)
                .unwrap()
                .iter()
                .find(|n| n.id == legacy.id)
                .unwrap()
                .content,
            "另一条既有笔记"
        );
        assert_eq!(s.list_tags().unwrap().len(), 2);
    }
    #[test]
    fn stale_screenshot_or_note_revisions_never_overwrite_drafts_in_database() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let id = create(&mut s);
        let initial = s.knowledge_card(&id).unwrap();
        let saved = s.save_knowledge_card(input(&initial)).unwrap();
        assert!(matches!(
            s.save_knowledge_card(input(&initial)),
            Err(StorageError::Conflict)
        ));
        let note = saved.note.as_ref().unwrap();
        s.save_note(&id, Some(&note.id), Some(note.revision), "外部更新", None)
            .unwrap();
        assert!(matches!(
            s.save_knowledge_card(input(&saved)),
            Err(StorageError::Conflict)
        ));
        assert_eq!(
            s.knowledge_card(&id).unwrap().note.unwrap().content,
            "外部更新"
        );
        assert_eq!(s.get_screenshot(&id).unwrap().revision, 2);
    }
    #[test]
    fn failed_tag_write_rolls_back_title_note_favorite_and_associations() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let id = create(&mut s);
        s.connection.execute_batch("CREATE TRIGGER fail_tag BEFORE INSERT ON screenshot_tags BEGIN SELECT RAISE(ABORT,'test-only'); END;").unwrap();
        assert!(
            s.save_knowledge_card(input(&s.knowledge_card(&id).unwrap()))
                .is_err()
        );
        let card = s.knowledge_card(&id).unwrap();
        assert_eq!(card.screenshot.title, "科研图片");
        assert!(!card.screenshot.is_favorite);
        assert_eq!(card.screenshot.revision, 1);
        assert!(card.note.is_none());
        assert!(card.tags.is_empty());
        assert!(s.list_tags().unwrap().is_empty());
    }
    #[test]
    fn removing_association_or_deleting_tag_preserves_screenshot_and_note() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let id = create(&mut s);
        let saved = s
            .save_knowledge_card(input(&s.knowledge_card(&id).unwrap()))
            .unwrap();
        let tag = saved.tags[0].id.clone();
        s.delete_tag(&tag).unwrap();
        let current = s.knowledge_card(&id).unwrap();
        assert_eq!(current.tags.len(), 1);
        assert!(current.note.is_some());
        assert_eq!(current.screenshot.revision, 3);
        assert!(matches!(
            s.save_knowledge_card(input(&saved)),
            Err(StorageError::Conflict)
        ));
        let mut update = input(&current);
        update.tag_names.clear();
        let removed = s.save_knowledge_card(update).unwrap();
        assert!(removed.tags.is_empty());
        assert!(removed.note.is_some());
        assert!(s.screenshot_image(&id, false).is_ok());
    }
    #[test]
    fn source_urls_and_limits_are_validated_without_html_execution_or_data_changes() {
        for url in [
            "javascript:alert(1)",
            "data:text/html,<script>",
            "file:///C:/private",
            "https://",
            "https://user:pass@example.org",
            "https://example.org/a b",
        ] {
            assert!(source_url(Some(url)).is_err());
        }
        assert_eq!(
            source_url(Some("https://example.org/path?x=1&y=2"))
                .unwrap()
                .as_deref(),
            Some("https://example.org/path?x=1&y=2")
        );
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let id = create(&mut s);
        let card = s.knowledge_card(&id).unwrap();
        let mut bad = input(&card);
        bad.tag_names = vec!["Tag".into(), "tag".into()];
        assert!(s.save_knowledge_card(bad).is_err());
        let mut bad = input(&card);
        bad.content = "x".repeat(1024 * 1024 + 1);
        assert!(s.save_knowledge_card(bad).is_err());
        assert_eq!(s.knowledge_card(&id).unwrap().screenshot.revision, 1);
    }
    #[test]
    fn source_url_normalization_keeps_encoded_urls_reopenable_within_bounds() {
        assert_eq!(
            source_url(Some("HTTPS://EXAMPLE.ORG/论文"))
                .unwrap()
                .as_deref(),
            Some("https://example.org/%E8%AE%BA%E6%96%87")
        );
        let input = format!("https://example.org/{}", "论".repeat(300));
        assert!(input.len() < 2048);
        assert!(source_url(Some(&input)).is_err());
    }
}
