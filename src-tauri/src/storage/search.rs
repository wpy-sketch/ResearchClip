use super::{
    Storage, StorageError, files,
    library::{LibrarySort, ScreenshotPage, ScreenshotSummary, check_id},
};
use rusqlite::{Row, params};
use serde::Deserialize;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryFilter {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub tag_ids: Vec<String>,
    pub favorite: Option<bool>,
    pub project_id: Option<String>,
    #[serde(default)]
    pub unclassified: bool,
    #[serde(default)]
    pub source: SearchSource,
}

#[derive(Clone, Copy, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchSource {
    #[default]
    All,
    Title,
    Notes,
    Tags,
    Ocr,
}
impl SearchSource {
    pub fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Title => "title",
            Self::Notes => "notes",
            Self::Tags => "tags",
            Self::Ocr => "ocr",
        }
    }
}
pub(super) fn fts_query(query: &str, source: SearchSource) -> String {
    let query = if query.is_empty() {
        "__researchclip_empty__"
    } else {
        query
    };
    let phrase = format!("\"{}\"", query.replace('"', "\"\""));
    if matches!(source, SearchSource::All) {
        phrase
    } else {
        format!("{} : {phrase}", source.key())
    }
}
pub(super) const SUMMARY_COLUMNS: &str = "id,title,original_image_path,thumbnail_path,width_px,height_px,created_at,updated_at,is_favorite,revision";
pub(super) const FILTER: &str = r"(?1=0 OR is_favorite=1) AND (?2 IS NULL OR is_favorite=?2)
 AND (?3='' OR id IN (SELECT screenshot_id FROM screenshot_search WHERE
 (length(?3)>=3 AND rowid IN (SELECT rowid FROM screenshot_search WHERE screenshot_search MATCH ?11))
 OR (length(?3)<3 AND ((?10 IN ('all','title') AND title LIKE ?4 ESCAPE '\')
 OR (?10 IN ('all','notes') AND notes LIKE ?4 ESCAPE '\')
 OR (?10 IN ('all','tags') AND tags LIKE ?4 ESCAPE '\')
 OR (?10 IN ('all','ocr') AND ocr LIKE ?4 ESCAPE '\')))))
 AND NOT EXISTS(SELECT 1 FROM json_each(?5) wanted WHERE NOT EXISTS(
 SELECT 1 FROM screenshot_tags st WHERE st.screenshot_id=screenshots.id AND st.tag_id=wanted.value))
 AND (?8 IS NULL OR EXISTS(SELECT 1 FROM screenshot_projects sp WHERE sp.screenshot_id=screenshots.id AND sp.project_id=?8))
 AND (?9=0 OR NOT EXISTS(SELECT 1 FROM screenshot_projects sp WHERE sp.screenshot_id=screenshots.id))";

pub(super) fn pattern(query: &str) -> String {
    let mut result = String::from("%");
    for c in query.chars() {
        if matches!(c, '%' | '_' | '\\') {
            result.push('\\');
        }
        result.push(c);
    }
    result.push('%');
    result
}
impl Storage {
    pub(super) fn summary_row(&self, r: &Row<'_>) -> rusqlite::Result<ScreenshotSummary> {
        let id: String = r.get(0)?;
        let original: String = r.get(2)?;
        let thumbnail: String = r.get(3)?;
        let available = |path: &str, folder: &str| {
            path == format!("{folder}/{id}.png")
                && files::image_path(&self.data_dir, path).is_ok_and(|p| p.is_file())
        };
        Ok(ScreenshotSummary {
            match_excerpt: None,
            id: id.clone(),
            title: r.get(1)?,
            width_px: r.get(4)?,
            height_px: r.get(5)?,
            created_at: r.get(6)?,
            updated_at: r.get(7)?,
            is_favorite: r.get(8)?,
            revision: r.get(9)?,
            image_available: available(&original, "originals"),
            thumbnail_available: available(&thumbnail, "thumbnails"),
            original_image_path: original,
            thumbnail_path: thumbnail,
        })
    }
    pub fn search_screenshot_page(
        &self,
        page: u32,
        limit: u32,
        favorites: bool,
        sort: LibrarySort,
        filter: &LibraryFilter,
    ) -> Result<ScreenshotPage, StorageError> {
        if limit == 0
            || limit > 50
            || page > 1_000_000
            || filter.query.chars().count() > 200
            || filter.query.contains('\0')
            || filter.tag_ids.len() > 100
        {
            return Err(StorageError::Invalid(
                "搜索或分页参数无效（关键词最多200字符）。",
            ));
        }
        let mut unique = std::collections::HashSet::new();
        for id in &filter.tag_ids {
            check_id(id)?;
            if !unique.insert(id) {
                return Err(StorageError::Invalid("筛选标签不能重复。"));
            }
        }
        if let Some(id) = &filter.project_id {
            check_id(id)?;
        }
        if filter.project_id.is_some() && filter.unclassified {
            return Err(StorageError::Invalid("项目筛选冲突。"));
        }
        let query = filter.query.trim();
        let like = pattern(query);
        let tags = serde_json::to_string(&filter.tag_ids)
            .map_err(|_| StorageError::Invalid("标签筛选无效。"))?;
        let tx = self.connection.unchecked_transaction()?;
        let total: u64 = tx.query_row(
            &format!("SELECT count(*) FROM screenshots WHERE {FILTER}"),
            params![
                favorites,
                filter.favorite,
                query,
                like,
                tags,
                rusqlite::types::Null,
                rusqlite::types::Null,
                filter.project_id,
                filter.unclassified,
                filter.source.key(),
                fts_query(query, filter.source)
            ],
            |r| r.get(0),
        )?;
        let page = u64::from(page).min(total.saturating_sub(1) / u64::from(limit)) as u32;
        let order = match sort {
            LibrarySort::CreatedDesc => "created_at DESC,id DESC",
            LibrarySort::CreatedAsc => "created_at ASC,id ASC",
        };
        let mut items = {
            let mut stmt=tx.prepare(&format!("SELECT {SUMMARY_COLUMNS} FROM screenshots WHERE {FILTER} ORDER BY {order} LIMIT ?6 OFFSET ?7"))?;
            stmt.query_map(
                params![
                    favorites,
                    filter.favorite,
                    query,
                    like,
                    tags,
                    limit,
                    page * limit,
                    filter.project_id,
                    filter.unclassified,
                    filter.source.key(),
                    fts_query(query, filter.source)
                ],
                |r| self.summary_row(r),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?
        };
        for item in &mut items {
            item.match_excerpt = self.search_excerpt(&item.id, query, filter.source)?;
        }
        tx.commit()?;
        Ok(ScreenshotPage {
            items,
            total,
            page,
            limit,
            sort,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{capture::RawFrame, services::screen_capture::encode_png};
    fn create(s: &mut Storage, title: &str) -> String {
        let bytes = encode_png(RawFrame {
            width_px: 2,
            height_px: 2,
            rgba: [1, 2, 3, 255].repeat(4),
        })
        .unwrap();
        s.create_screenshot(
            title,
            serde_json::json!({"schemaVersion":1,"image":{"widthPx":2,"heightPx":2},"objects":[]}),
            &bytes,
        )
        .unwrap()
        .id
    }
    fn query(s: &Storage, text: &str) -> ScreenshotPage {
        s.search_screenshot_page(
            0,
            24,
            false,
            LibrarySort::CreatedDesc,
            &LibraryFilter {
                query: text.into(),
                ..Default::default()
            },
        )
        .unwrap()
    }
    #[test]
    fn searches_chinese_and_ascii_case_insensitive_titles_and_all_notes() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = create(&mut s, "细胞 Alpha显微图");
        let b = create(&mut s, "其他图片");
        s.save_note(&b, None, None, "# 科研笔记\nBETA 实验", None)
            .unwrap();
        s.save_note(&b, None, None, "第二条：细胞核", None).unwrap();
        assert_eq!(query(&s, "alpha").items[0].id, a);
        assert_eq!(query(&s, "beta").items[0].id, b);
        assert_eq!(query(&s, "细胞").total, 2);
        assert_eq!(query(&s, "细胞核").items[0].id, b);
        assert_eq!(query(&s, "  \n ").total, 2);
        assert_eq!(query(&s, "").total, 2);
    }
    #[test]
    fn wildcards_backslash_quotes_and_sql_like_text_are_literal_bound_values() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = create(&mut s, r"100% A_B C:\研究 O'Brien");
        create(&mut s, "100x AxB C研究");
        s.save_note(&a, None, None, "'); DROP TABLE screenshots; --", None)
            .unwrap();
        for text in ["%", "_", r"\", "O'Brien", "'); DROP TABLE screenshots; --"] {
            let result = query(&s, text);
            assert_eq!(result.total, 1);
            assert_eq!(result.items[0].id, a);
        }
        assert_eq!(query(&s, "missing").total, 0);
        assert_eq!(query(&s, "").total, 2);
    }
    #[test]
    fn multiple_tag_filters_use_and_and_combine_with_notes_and_favorites() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = create(&mut s, "A");
        let b = create(&mut s, "B");
        let t = s.save_tag(None, "课题").unwrap();
        let u = s.save_tag(None, "方法").unwrap();
        s.set_screenshot_tags(&a, 1, &[t.id.clone(), u.id.clone()])
            .unwrap();
        s.set_screenshot_tags(&b, 1, std::slice::from_ref(&t.id))
            .unwrap();
        s.save_note(&a, None, None, "共有关键词", None).unwrap();
        s.save_note(&b, None, None, "共有关键词", None).unwrap();
        s.update_screenshot(
            &a,
            2,
            crate::storage::library::ScreenshotPatch {
                title: None,
                is_favorite: Some(true),
                annotation_data: None,
            },
        )
        .unwrap();
        let mut filter = LibraryFilter {
            query: "关键词".into(),
            tag_ids: vec![t.id, u.id],
            favorite: Some(true),
            ..Default::default()
        };
        let page = s
            .search_screenshot_page(0, 24, false, LibrarySort::CreatedAsc, &filter)
            .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].id, a);
        filter.favorite = Some(false);
        assert_eq!(
            s.search_screenshot_page(0, 24, false, LibrarySort::CreatedAsc, &filter)
                .unwrap()
                .total,
            0
        );
        filter.tag_ids.pop();
        assert_eq!(
            s.search_screenshot_page(0, 24, false, LibrarySort::CreatedAsc, &filter)
                .unwrap()
                .items[0]
                .id,
            b
        );
        assert_eq!(
            s.search_screenshot_page(0, 24, true, LibrarySort::CreatedAsc, &filter)
                .unwrap()
                .total,
            0
        );
    }
    #[test]
    fn filtered_pages_sort_stably_and_read_metadata_without_originals() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let mut ids = Vec::new();
        for n in 0..5 {
            let id = create(&mut s, &format!("科研 {n}"));
            s.connection
                .execute(
                    "UPDATE screenshots SET created_at=?1 WHERE id=?2",
                    params![n + 1, id],
                )
                .unwrap();
            ids.push(id);
        }
        std::fs::remove_file(tmp.path().join(format!("originals/{}.png", ids[0]))).unwrap();
        let filter = LibraryFilter {
            query: "科研".into(),
            ..Default::default()
        };
        let first = s
            .search_screenshot_page(0, 2, false, LibrarySort::CreatedAsc, &filter)
            .unwrap();
        assert_eq!(first.total, 5);
        assert_eq!(first.items[0].id, ids[0]);
        assert!(!first.items[0].image_available);
        let last = s
            .search_screenshot_page(9, 2, false, LibrarySort::CreatedAsc, &filter)
            .unwrap();
        assert_eq!(last.page, 2);
        assert_eq!(last.items[0].id, ids[4]);
    }
    #[test]
    fn empty_database_unknown_tag_and_bad_query_are_handled_safely() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Storage::open(tmp.path().into()).unwrap();
        assert_eq!(query(&s, "研究").total, 0);
        for filter in [
            LibraryFilter {
                query: "x".repeat(201),
                ..Default::default()
            },
            LibraryFilter {
                query: "a\0b".into(),
                ..Default::default()
            },
            LibraryFilter {
                tag_ids: vec!["bad".into()],
                ..Default::default()
            },
            LibraryFilter {
                tag_ids: vec!["a".repeat(32); 2],
                ..Default::default()
            },
        ] {
            assert!(
                s.search_screenshot_page(0, 24, false, LibrarySort::CreatedDesc, &filter)
                    .is_err()
            );
        }
        assert_eq!(
            s.search_screenshot_page(
                0,
                24,
                false,
                LibrarySort::CreatedDesc,
                &LibraryFilter {
                    tag_ids: vec!["a".repeat(32)],
                    ..Default::default()
                }
            )
            .unwrap()
            .total,
            0
        );
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchExcerpt {
    pub source: String,
    pub text: String,
    pub keyword: String,
}
impl Storage {
    fn search_excerpt(
        &self,
        id: &str,
        query: &str,
        source: SearchSource,
    ) -> Result<Option<MatchExcerpt>, StorageError> {
        if query.is_empty() {
            return Ok(None);
        }
        let values = self.connection.query_row(
            "SELECT title,notes,tags,ocr FROM screenshot_search WHERE screenshot_id=?1",
            [id],
            |r| Ok([r.get::<_, String>(0)?, r.get(1)?, r.get(2)?, r.get(3)?]),
        )?;
        for (key, text) in ["title", "notes", "tags", "ocr"].into_iter().zip(values) {
            if source.key() != "all" && key != source.key() {
                continue;
            }
            // ASCII folding keeps byte indices stable; Chinese is matched literally. No HTML markers enter the UI.
            if let Some(byte) = text.to_ascii_lowercase().find(&query.to_ascii_lowercase()) {
                let before = text[..byte].chars().count();
                let excerpt: String = text
                    .chars()
                    .skip(before.saturating_sub(35))
                    .take(140)
                    .collect();
                return Ok(Some(MatchExcerpt {
                    source: key.into(),
                    text: excerpt,
                    keyword: query.into(),
                }));
            }
        }
        Ok(None)
    }
}
