use super::{
    Storage, StorageError, files,
    library::{check_id, now},
};
use crate::services::similarity::{ALGORITHM_VERSION, Features, color_distance, distance};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
type Result<T> = std::result::Result<T, StorageError>;
#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchScope {
    pub project_id: Option<String>,
    #[serde(default)]
    pub unclassified: bool,
    #[serde(default)]
    pub favorites: bool,
}
impl SearchScope {
    fn validate(&self) -> Result<()> {
        if let Some(id) = &self.project_id {
            check_id(id)?;
            if self.unclassified {
                return Err(StorageError::Invalid("项目与未分类范围不能同时选择。"));
            }
        }
        Ok(())
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureStats {
    pub total: u64,
    pub ready: u64,
    pub failed: u64,
    pub algorithm_version: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Match {
    pub id: String,
    pub title: String,
    pub width_px: u32,
    pub height_px: u32,
    pub created_at: i64,
    pub exact: bool,
    pub distance: u32,
    pub difference_distance: u32,
    pub color_distance: u32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Matches {
    pub items: Vec<Match>,
    pub total: u64,
    pub page: u32,
    pub stale: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub reference_id: String,
    pub content_hash: String,
    pub count: u64,
    pub title: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Groups {
    pub items: Vec<DuplicateGroup>,
    pub total: u64,
    pub page: u32,
}
pub struct Input {
    pub stamp: String,
    pub bytes: Vec<u8>,
}
impl Storage {
    pub fn feature_stats(&self) -> Result<FeatureStats> {
        Ok(FeatureStats{total:self.connection.query_row("SELECT count(*) FROM screenshots",[],|r|r.get(0))?,ready:self.connection.query_row("SELECT count(*) FROM image_features WHERE status='ready' AND hash_algorithm_version=?1",[ALGORITHM_VERSION],|r|r.get(0))?,failed:self.connection.query_row("SELECT count(*) FROM image_features WHERE status='failed'",[],|r|r.get(0))?,algorithm_version:ALGORITHM_VERSION})
    }
    pub fn next_feature_id(&self, after: &str, upper: &str) -> Result<Option<String>> {
        Ok(self
            .connection
            .query_row(
                "SELECT id FROM screenshots WHERE id>?1 AND id<=?2 ORDER BY id LIMIT 1",
                params![after, upper],
                |r| r.get(0),
            )
            .optional()?)
    }
    pub fn feature_upper(&self) -> Result<String> {
        Ok(self
            .connection
            .query_row("SELECT coalesce(max(id),'') FROM screenshots", [], |r| {
                r.get(0)
            })?)
    }
    pub(crate) fn feature_stamp(&self, id: &str) -> Result<String> {
        check_id(id)?;
        let (relative, w, h): (String, u32, u32) = self
            .connection
            .query_row(
                "SELECT original_image_path,width_px,height_px FROM screenshots WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
            .ok_or(StorageError::NotFound)?;
        let path = files::image_path(&self.data_dir, &relative)?;
        let meta = std::fs::metadata(path)?;
        if !meta.is_file() || meta.len() > crate::capture::MAX_PNG_BYTES as u64 {
            return Err(StorageError::Invalid("原图不是普通文件或已超出大小限制。"));
        }
        let modified = meta
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| StorageError::Invalid("原图时间戳无效。"))?
            .as_nanos();
        Ok(format!("{relative}:{w}:{h}:{}:{modified}", meta.len()))
    }
    pub fn feature_input(&self, id: &str, rebuild: bool) -> Result<Option<Input>> {
        let stamp = self.feature_stamp(id)?;
        if !rebuild&&self.connection.query_row("SELECT source_stamp=?2 AND hash_algorithm_version=?3 FROM image_features WHERE screenshot_id=?1",params![id,stamp,ALGORITHM_VERSION],|r|r.get::<_,bool>(0)).optional()?.unwrap_or(false){return Ok(None);}
        let path: String = self.connection.query_row(
            "SELECT original_image_path FROM screenshots WHERE id=?1",
            [id],
            |r| r.get(0),
        )?;
        let bytes = files::read(&self.data_dir, &path)?;
        if self.feature_stamp(id)? != stamp {
            return Err(StorageError::Conflict);
        }
        Ok(Some(Input { stamp, bytes }))
    }
    pub fn save_features(&mut self, id: &str, stamp: &str, features: &Features) -> Result<()> {
        let dimensions: (u32, u32) = self.connection.query_row(
            "SELECT width_px,height_px FROM screenshots WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if dimensions != (features.width_px, features.height_px) {
            return Err(StorageError::Invalid(
                "原图实际尺寸与资料不一致；请核对文件。",
            ));
        }
        if self.feature_stamp(id)? != stamp {
            return Err(StorageError::Conflict);
        }
        // The upsert is one atomic statement; a failed rebuild never deletes other entries.
        self.connection.execute("INSERT INTO image_features VALUES(?1,?2,?3,?4,?5,?6,?7,'ready',NULL,?8) ON CONFLICT(screenshot_id) DO UPDATE SET content_hash=excluded.content_hash,perceptual_hash=excluded.perceptual_hash,difference_hash=excluded.difference_hash,descriptor=excluded.descriptor,hash_algorithm_version=excluded.hash_algorithm_version,source_stamp=excluded.source_stamp,status='ready',error=NULL,updated_at=excluded.updated_at",params![id,features.content_hash,features.perceptual_hash,features.difference_hash,features.descriptor,ALGORITHM_VERSION,stamp,now()?])?;
        Ok(())
    }
    pub fn fail_features(&mut self, id: &str) -> Result<()> {
        check_id(id)?;
        if !self.connection.query_row(
            "SELECT exists(SELECT 1 FROM screenshots WHERE id=?1)",
            [id],
            |r| r.get::<_, bool>(0),
        )? {
            return Ok(());
        }
        let stamp = self
            .feature_stamp(id)
            .unwrap_or_else(|_| "unavailable".into());
        self.connection.execute("INSERT INTO image_features VALUES(?1,NULL,NULL,NULL,NULL,?2,?3,'failed',?4,?5) ON CONFLICT(screenshot_id) DO UPDATE SET content_hash=NULL,perceptual_hash=NULL,difference_hash=NULL,descriptor=NULL,hash_algorithm_version=excluded.hash_algorithm_version,source_stamp=excluded.source_stamp,status='failed',error=excluded.error,updated_at=excluded.updated_at",params![id,ALGORITHM_VERSION,stamp,"原图损坏、缺失、已改变或暂不可访问；原资料保留。",now()?])?;
        Ok(())
    }
    #[cfg(test)]
    pub fn similar_images(
        &self,
        id: &str,
        exact_only: bool,
        threshold: u32,
        color_limit: u32,
        page: u32,
    ) -> Result<Matches> {
        self.scoped_similar_images(
            id,
            exact_only,
            threshold,
            color_limit,
            page,
            &SearchScope::default(),
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn scoped_similar_images(
        &self,
        id: &str,
        exact_only: bool,
        threshold: u32,
        color_limit: u32,
        page: u32,
        scope: &SearchScope,
    ) -> Result<Matches> {
        scope.validate()?;
        check_id(id)?;
        if threshold > 64 || color_limit > 255 || page >= 200 {
            return Err(StorageError::Invalid("距离阈值或页码超出范围。"));
        }
        let(source_hash,p,d,descriptor,stamp):(String,String,String,Vec<u8>,String)=self.connection.query_row("SELECT content_hash,perceptual_hash,difference_hash,descriptor,source_stamp FROM image_features WHERE screenshot_id=?1 AND status='ready' AND hash_algorithm_version=?2",params![id,ALGORITHM_VERSION],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?.ok_or(StorageError::Invalid("此图片尚无可用特征，请先更新索引；失败时重建索引。"))?;
        if self.feature_stamp(id)? != stamp {
            return Err(StorageError::Invalid(
                "参考原图已改变，请更新索引后再检索。",
            ));
        }
        let mut statement=self.connection.prepare("SELECT s.id,s.title,s.width_px,s.height_px,s.created_at,f.content_hash,f.perceptual_hash,f.difference_hash,f.descriptor,f.source_stamp FROM screenshots s JOIN image_features f ON s.id=f.screenshot_id WHERE f.status='ready' AND f.hash_algorithm_version=?1 AND s.id!=?2 AND (?3=0 OR f.content_hash=?4) AND (?5 IS NULL OR EXISTS(SELECT 1 FROM screenshot_projects p WHERE p.screenshot_id=s.id AND p.project_id=?5)) AND (?6=0 OR NOT EXISTS(SELECT 1 FROM screenshot_projects p WHERE p.screenshot_id=s.id)) AND (?7=0 OR s.is_favorite=1) ORDER BY s.id")?;
        let mut rows = statement.query(params![
            ALGORITHM_VERSION,
            id,
            exact_only,
            source_hash,
            scope.project_id,
            scope.unclassified,
            scope.favorites
        ])?;
        let mut items: Vec<Match> = vec![];
        let mut total = 0;
        let mut stale = 0;
        let keep = (page as usize + 1) * 25;
        while let Some(r) = rows.next()? {
            let hash: String = r.get(5)?;
            let exact = hash == source_hash;
            let pd = distance(&p, &r.get::<_, String>(6)?)?;
            let dd = distance(&d, &r.get::<_, String>(7)?)?;
            let cd = color_distance(&descriptor, &r.get::<_, Vec<u8>>(8)?)?;
            if !exact && (exact_only || pd > threshold || cd > color_limit) {
                continue;
            }
            let candidate: String = r.get(0)?;
            if self.feature_stamp(&candidate).ok().as_ref() != Some(&r.get::<_, String>(9)?) {
                stale += 1;
                continue;
            }
            total += 1;
            let value = Match {
                id: candidate,
                title: r.get(1)?,
                width_px: r.get(2)?,
                height_px: r.get(3)?,
                created_at: r.get(4)?,
                exact,
                distance: pd,
                difference_distance: dd,
                color_distance: cd,
            };
            let key = |v: &Match| (!v.exact, v.distance, v.color_distance, v.id.clone());
            let index = items.partition_point(|v| key(v) <= key(&value));
            if index < keep {
                items.insert(index, value);
                if items.len() > keep {
                    items.pop();
                }
            }
        }
        Ok(Matches {
            items: items.into_iter().skip(page as usize * 25).collect(),
            total,
            page,
            stale,
        })
    }
    #[cfg(test)]
    pub fn duplicate_groups(&self, page: u32) -> Result<Groups> {
        self.scoped_duplicate_groups(page, &SearchScope::default())
    }
    pub fn scoped_duplicate_groups(&self, page: u32, scope: &SearchScope) -> Result<Groups> {
        scope.validate()?;
        if page >= 5000 {
            return Err(StorageError::Invalid("重复组页码超出范围。"));
        }
        let predicate = "FROM image_features f JOIN screenshots s ON s.id=f.screenshot_id WHERE f.status='ready' AND f.hash_algorithm_version=?1 AND (?2 IS NULL OR EXISTS(SELECT 1 FROM screenshot_projects p WHERE p.screenshot_id=s.id AND p.project_id=?2)) AND (?3=0 OR NOT EXISTS(SELECT 1 FROM screenshot_projects p WHERE p.screenshot_id=s.id)) AND (?4=0 OR s.is_favorite=1) GROUP BY content_hash HAVING count(*)>1";
        let total = self.connection.query_row(
            &format!("SELECT count(*) FROM (SELECT content_hash {predicate})"),
            params![
                ALGORITHM_VERSION,
                scope.project_id,
                scope.unclassified,
                scope.favorites
            ],
            |r| r.get(0),
        )?;
        let mut stmt=self.connection.prepare(&format!("SELECT min(f.screenshot_id),f.content_hash,count(*) {predicate} ORDER BY count(*) DESC,content_hash LIMIT 20 OFFSET ?5"))?;
        let groups = stmt
            .query_map(
                params![
                    ALGORITHM_VERSION,
                    scope.project_id,
                    scope.unclassified,
                    scope.favorites,
                    page * 20
                ],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, u64>(2)?,
                    ))
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let items = groups
            .into_iter()
            .map(|(id, hash, count)| {
                let title = self.connection.query_row(
                    "SELECT title FROM screenshots WHERE id=?1",
                    [&id],
                    |r| r.get(0),
                )?;
                Ok(DuplicateGroup {
                    reference_id: id,
                    content_hash: hash,
                    count,
                    title,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Groups { items, total, page })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        capture::RawFrame,
        services::{screen_capture::encode_png, similarity::compute},
    };
    fn png(color: u8) -> Vec<u8> {
        encode_png(RawFrame {
            width_px: 32,
            height_px: 24,
            rgba: (0..768)
                .flat_map(|i| [color, (i % 256) as u8, ((i / 32) * 10) as u8, 255])
                .collect(),
        })
        .unwrap()
    }
    fn add(s: &mut Storage, title: &str, color: u8) -> String {
        s.create_screenshot(title,serde_json::json!({"schemaVersion":1,"image":{"widthPx":32,"heightPx":24},"objects":[]}),&png(color)).unwrap().id
    }
    fn index(s: &mut Storage, id: &str) {
        let input = s.feature_input(id, false).unwrap().unwrap();
        s.save_features(id, &input.stamp, &compute(&input.bytes).unwrap())
            .unwrap();
    }
    #[test]
    fn exact_different_names_pagination_reopen_no_automatic_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = add(&mut s, "中文研究", 20);
        let b = add(&mut s, "different-name", 20);
        let c = add(&mut s, "other", 250);
        for id in [&a, &b, &c] {
            index(&mut s, id);
        }
        assert!(s.feature_input(&a, false).unwrap().is_none());
        let hits = s.similar_images(&a, true, 0, 0, 0).unwrap();
        assert_eq!(hits.total, 1);
        assert_eq!(hits.items[0].id, b);
        assert!(hits.items[0].exact);
        assert_eq!(s.duplicate_groups(0).unwrap().items[0].count, 2);
        assert_eq!(s.feature_stats().unwrap().total, 3);
        assert!(s.similar_images(&a, false, 65, 20, 0).is_err());
        assert!(s.similar_images("../../bad", false, 8, 25, 0).is_err());
        assert!(s.similar_images(&a, false, 8, 25, 200).is_err());
        drop(s);
        let mut s = Storage::open(tmp.path().into()).unwrap();
        assert_eq!(s.feature_stats().unwrap().ready, 3);
        assert_eq!(s.similar_images(&a, true, 0, 0, 0).unwrap().items[0].id, b);
        let shot = s.get_screenshot(&b).unwrap();
        s.delete_screenshot(&b, shot.revision).unwrap();
        assert_eq!(s.duplicate_groups(0).unwrap().total, 0);
        assert_eq!(s.feature_stats().unwrap().ready, 2);
        assert_eq!(s.screenshot_image(&a, false).unwrap(), png(20));
    }
    #[test]
    fn changed_corrupt_and_failed_rebuild_are_isolated() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = add(&mut s, "A", 20);
        let b = add(&mut s, "B", 20);
        index(&mut s, &a);
        index(&mut s, &b);
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(tmp.path().join(format!("originals/{a}.png")), png(200)).unwrap();
        assert!(s.similar_images(&a, true, 0, 0, 0).is_err());
        assert!(s.feature_input(&a, false).unwrap().is_some());
        index(&mut s, &a);
        assert_eq!(s.similar_images(&a, true, 0, 0, 0).unwrap().total, 0);
        std::fs::write(tmp.path().join(format!("originals/{a}.png")), b"broken").unwrap();
        let input = s.feature_input(&a, true).unwrap().unwrap();
        assert!(compute(&input.bytes).is_err());
        s.fail_features(&a).unwrap();
        assert_eq!(s.feature_stats().unwrap().failed, 1);
        assert_eq!(s.feature_stats().unwrap().ready, 1);
        assert_eq!(s.screenshot_image(&b, false).unwrap(), png(20));
        assert!(s.feature_input(&b, false).unwrap().is_none());
    }
    #[test]
    fn project_unclassified_and_favorites_filter_candidates_and_duplicate_groups() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(tmp.path().into()).unwrap();
        let a = add(&mut s, "A", 20);
        let b = add(&mut s, "B", 20);
        let c = add(&mut s, "C", 20);
        for id in [&a, &b, &c] {
            index(&mut s, id);
        }
        s.save_project(None, "Research").unwrap();
        let project = s.projects().unwrap().remove(0).id;
        s.connection
            .execute(
                "INSERT INTO screenshot_projects VALUES(?1,?2)",
                params![a, project],
            )
            .unwrap();
        s.connection
            .execute(
                "INSERT INTO screenshot_projects VALUES(?1,?2)",
                params![b, project],
            )
            .unwrap();
        s.connection
            .execute("UPDATE screenshots SET is_favorite=1 WHERE id=?1", [&b])
            .unwrap();
        let scope = SearchScope {
            project_id: Some(project),
            unclassified: false,
            favorites: false,
        };
        assert_eq!(
            s.scoped_similar_images(&c, true, 0, 0, 0, &scope)
                .unwrap()
                .total,
            2
        );
        assert_eq!(
            s.scoped_duplicate_groups(0, &scope).unwrap().items[0].count,
            2
        );
        let favorite = SearchScope {
            favorites: true,
            ..scope
        };
        assert_eq!(
            s.scoped_similar_images(&a, true, 0, 0, 0, &favorite)
                .unwrap()
                .items[0]
                .id,
            b
        );
        assert_eq!(s.scoped_duplicate_groups(0, &favorite).unwrap().total, 0);
        let none = SearchScope {
            unclassified: true,
            ..SearchScope::default()
        };
        assert_eq!(
            s.scoped_similar_images(&a, true, 0, 0, 0, &none)
                .unwrap()
                .items[0]
                .id,
            c
        );
        assert!(
            s.scoped_duplicate_groups(
                0,
                &SearchScope {
                    project_id: Some("../".into()),
                    ..SearchScope::default()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn v6_migration_preserves_original_and_creates_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let id;
        {
            let mut s = Storage::open(tmp.path().into()).unwrap();
            id = add(&mut s, "Keep", 30);
        }
        let conn = rusqlite::Connection::open(tmp.path().join("researchclip.sqlite3")).unwrap();
        crate::storage::resources::discard_schema12_test_fixture(&conn);
        conn.execute_batch("DROP TABLE semantic_embeddings;DROP TABLE annotation_anchors;DROP TABLE document_tags;DROP TABLE link_index_state;DROP TABLE link_index;DROP TABLE screenshot_references;DROP TABLE document_references;DROP TABLE literature_references;DROP TRIGGER image_features_source_changed;DROP TABLE image_features;PRAGMA user_version=6;").unwrap();
        drop(conn);
        let s = Storage::open(tmp.path().into()).unwrap();
        assert_eq!(
            s.status().unwrap().schema_version,
            crate::storage::migrations::SCHEMA_VERSION
        );
        assert_eq!(s.screenshot_image(&id, false).unwrap(), png(30));
        assert_eq!(s.feature_stats().unwrap().ready, 0);
        assert!(
            std::fs::read_dir(tmp.path().join("backups"))
                .unwrap()
                .any(|p| p
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("schema-6-"))
        );
    }
}
