use super::{
    Storage, StorageError,
    library::{check_id, now},
    similarity::SearchScope,
};
use crate::services::embeddings::{MODEL_ID, MODEL_VERSION, validate_vector};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticMatch {
    pub id: String,
    pub title: String,
    pub score: f32,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticResults {
    pub items: Vec<SemanticMatch>,
    pub stale: u64,
    pub searched: u64,
}
#[derive(Serialize)]
pub struct SemanticStats {
    pub total: u64,
    pub ready: u64,
}

impl Storage {
    pub fn semantic_stats(&self) -> Result<SemanticStats, StorageError> {
        Ok(SemanticStats {
            total: self.connection.query_row("SELECT count(*) FROM screenshots", [], |r|r.get(0))?,
            ready: self.connection.query_row("SELECT count(*) FROM semantic_embeddings WHERE embedding_model_id=?1 AND embedding_version=?2", params![MODEL_ID, MODEL_VERSION], |r|r.get(0))?,
        })
    }
    pub fn semantic_input(
        &self,
        id: &str,
        rebuild: bool,
    ) -> Result<Option<(Vec<u8>, String, String)>, StorageError> {
        check_id(id)?;
        let stamp = self.feature_stamp(id)?;
        let bytes = self.screenshot_image(id, false)?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        if !rebuild && self.connection.query_row("SELECT image_hash=?2 AND embedding_model_id=?3 AND embedding_version=?4 AND source_stamp=?5 FROM semantic_embeddings WHERE screenshot_id=?1",params![id,hash,MODEL_ID,MODEL_VERSION,stamp],|r|r.get::<_,bool>(0)).optional()?.unwrap_or(false) {
            return Ok(None);
        }
        Ok(Some((bytes, hash, stamp)))
    }
    pub fn save_embedding(
        &mut self,
        id: &str,
        hash: &str,
        stamp: &str,
        vector: &[f32],
    ) -> Result<(), StorageError> {
        validate_vector(vector)?;
        if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(StorageError::Invalid("图片摘要无效。"));
        }
        let bytes = self.screenshot_image(id, false)?;
        if self.feature_stamp(id)? != stamp || format!("{:x}", Sha256::digest(&bytes)) != hash {
            return Err(StorageError::Conflict);
        }
        let data: Vec<u8> = vector.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.connection.execute("INSERT INTO semantic_embeddings VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(screenshot_id) DO UPDATE SET embedding_model_id=excluded.embedding_model_id,embedding_version=excluded.embedding_version,image_hash=excluded.image_hash,embedding_data=excluded.embedding_data,source_stamp=excluded.source_stamp,updated_at=excluded.updated_at",params![id,MODEL_ID,MODEL_VERSION,hash,data,stamp,now()?])?;
        Ok(())
    }
    pub fn clear_embeddings(&mut self) -> Result<(), StorageError> {
        self.connection
            .execute("DELETE FROM semantic_embeddings", [])?;
        Ok(())
    }
    pub fn semantic_search(
        &self,
        vector: &[f32],
        scope: &SearchScope,
    ) -> Result<SemanticResults, StorageError> {
        validate_vector(vector)?;
        if let Some(id) = &scope.project_id {
            check_id(id)?;
            if scope.unclassified {
                return Err(StorageError::Invalid("项目与未分类不能同时选择。"));
            }
        }
        let rows=self.connection.prepare("SELECT e.screenshot_id,s.title,e.embedding_data,e.source_stamp FROM semantic_embeddings e JOIN screenshots s ON s.id=e.screenshot_id WHERE e.embedding_model_id=?1 AND e.embedding_version=?2 AND (?3 IS NULL OR EXISTS(SELECT 1 FROM screenshot_projects p WHERE p.screenshot_id=s.id AND p.project_id=?3)) AND (NOT ?4 OR NOT EXISTS(SELECT 1 FROM screenshot_projects p WHERE p.screenshot_id=s.id)) AND (NOT ?5 OR s.is_favorite=1) ORDER BY e.screenshot_id LIMIT 2001")?.query_map(params![MODEL_ID,MODEL_VERSION,scope.project_id,scope.unclassified,scope.favorites],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Vec<u8>>(2)?,r.get::<_,String>(3)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        if rows.len() > 2000 {
            return Err(StorageError::Invalid(
                "实验语义检索最多2000张，请按项目筛选。",
            ));
        }
        let mut result = SemanticResults {
            items: Vec::new(),
            stale: 0,
            searched: 0,
        };
        for (id, title, data, stamp) in rows {
            if !self
                .feature_stamp(&id)
                .is_ok_and(|current| current == stamp)
            {
                result.stale += 1;
                continue;
            }
            if data.len() != 2048 {
                return Err(StorageError::Invalid("向量数据损坏，请重建语义索引。"));
            }
            let target: Vec<f32> = data
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();
            validate_vector(&target)?;
            let score = vector
                .iter()
                .zip(&target)
                .map(|(a, b)| a * b)
                .sum::<f32>()
                .clamp(-1., 1.);
            result.items.push(SemanticMatch { id, title, score });
            result.searched += 1;
        }
        result
            .items
            .sort_by(|a, b| b.score.total_cmp(&a.score).then(a.id.cmp(&b.id)));
        result.items.truncate(60);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schema10_upgrade_snapshot_and_failed_migration_preserve_existing_data() {
        let (temp, s, id) = setup();
        let original = s.screenshot_image(&id, false).unwrap();
        s.connection
            .execute_batch("DROP TABLE semantic_embeddings; PRAGMA user_version=10;")
            .unwrap();
        drop(s);
        let s = Storage::open(temp.path().into()).unwrap();
        assert_eq!(s.status().unwrap().schema_version, 11);
        assert_eq!(s.screenshot_image(&id, false).unwrap(), original);
        let backups: Vec<_> = std::fs::read_dir(temp.path().join("backups"))
            .unwrap()
            .map(|f| f.unwrap().path())
            .collect();
        assert_eq!(backups.len(), 1);
        let old = rusqlite::Connection::open(&backups[0]).unwrap();
        assert_eq!(
            old.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            10
        );
        assert!(old.prepare("SELECT * FROM semantic_embeddings").is_err());
        s.connection.execute_batch("DROP TABLE semantic_embeddings; CREATE TABLE semantic_embeddings(unexpected TEXT); INSERT INTO semantic_embeddings VALUES('keep'); PRAGMA user_version=10;").unwrap();
        drop(s);
        assert!(Storage::open(temp.path().into()).is_err());
        let db = rusqlite::Connection::open(temp.path().join("researchclip.sqlite3")).unwrap();
        assert_eq!(
            db.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            10
        );
        assert_eq!(
            db.query_row("SELECT unexpected FROM semantic_embeddings", [], |r| r
                .get::<_, String>(
                0
            ))
            .unwrap(),
            "keep"
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM screenshots WHERE id=?1", [&id], |r| r
                .get::<_, u32>(0))
                .unwrap(),
            1
        );
    }
    fn setup() -> (tempfile::TempDir, Storage, String) {
        let temp = tempfile::tempdir().unwrap();
        let mut s = Storage::open(temp.path().into()).unwrap();
        let png = crate::services::screen_capture::encode_png(crate::capture::RawFrame {
            width_px: 2,
            height_px: 2,
            rgba: [30, 40, 50, 255].repeat(4),
        })
        .unwrap();
        let image=s.create_screenshot("公式",serde_json::json!({"schemaVersion":1,"image":{"widthPx":2,"heightPx":2},"objects":[]}),&png).unwrap();
        (temp, s, image.id)
    }
    fn vector() -> Vec<f32> {
        let mut v = vec![0.; 512];
        v[0] = 1.;
        v
    }
    #[test]
    fn persists_skips_and_rebuilds_with_stable_model_hash() {
        let (temp, mut s, id) = setup();
        let (_, hash, stamp) = s.semantic_input(&id, false).unwrap().unwrap();
        s.save_embedding(&id, &hash, &stamp, &vector()).unwrap();
        assert!(s.semantic_input(&id, false).unwrap().is_none());
        assert!(s.semantic_input(&id, true).unwrap().is_some());
        drop(s);
        let s = Storage::open(temp.path().into()).unwrap();
        let r = s
            .semantic_search(&vector(), &SearchScope::default())
            .unwrap();
        assert_eq!(r.items[0].id, id);
        assert_eq!(r.items[0].score, 1.);
        assert_eq!(s.semantic_stats().unwrap().ready, 1);
    }
    #[test]
    fn rejects_bad_vectors_conflicting_hash_and_unknown_ids() {
        let (_temp, mut s, id) = setup();
        let (_, hash, stamp) = s.semantic_input(&id, false).unwrap().unwrap();
        for v in [
            vec![0.; 512],
            vec![f32::NAN; 512],
            vec![1.; 511],
            vec![1.; 512],
        ] {
            assert!(s.save_embedding(&id, &hash, &stamp, &v).is_err());
        }
        assert!(
            s.save_embedding(&id, &"0".repeat(64), &stamp, &vector())
                .is_err()
        );
        assert!(s.semantic_input("../../secret", false).is_err());
        assert_eq!(s.semantic_stats().unwrap().ready, 0);
    }
    #[test]
    fn other_model_stale_original_delete_and_clear_are_safe() {
        let (_temp, mut s, id) = setup();
        let (_, hash, stamp) = s.semantic_input(&id, false).unwrap().unwrap();
        s.save_embedding(&id, &hash, &stamp, &vector()).unwrap();
        s.connection
            .execute(
                "UPDATE semantic_embeddings SET embedding_model_id='other'",
                [],
            )
            .unwrap();
        assert!(
            s.semantic_search(&vector(), &SearchScope::default())
                .unwrap()
                .items
                .is_empty()
        );
        s.connection
            .execute(
                "UPDATE semantic_embeddings SET embedding_model_id=?1,source_stamp='stale'",
                [MODEL_ID],
            )
            .unwrap();
        assert_eq!(
            s.semantic_search(&vector(), &SearchScope::default())
                .unwrap()
                .stale,
            1
        );
        s.clear_embeddings().unwrap();
        assert_eq!(s.semantic_stats().unwrap().ready, 0);
        assert!(s.get_screenshot(&id).is_ok());
        s.save_embedding(&id, &hash, &stamp, &vector()).unwrap();
        s.connection
            .execute("DELETE FROM screenshots WHERE id=?1", [id])
            .unwrap();
        assert_eq!(s.semantic_stats().unwrap().ready, 0);
    }
    #[test]
    fn filters_and_corrupt_vectors_are_not_silently_accepted() {
        let (_temp, mut s, id) = setup();
        let (_, hash, stamp) = s.semantic_input(&id, false).unwrap().unwrap();
        s.save_embedding(&id, &hash, &stamp, &vector()).unwrap();
        let favorites = SearchScope {
            favorites: true,
            ..Default::default()
        };
        assert!(
            s.semantic_search(&vector(), &favorites)
                .unwrap()
                .items
                .is_empty()
        );
        s.connection
            .execute("UPDATE screenshots SET is_favorite=1", [])
            .unwrap();
        assert_eq!(
            s.semantic_search(&vector(), &favorites)
                .unwrap()
                .items
                .len(),
            1
        );
        s.connection
            .execute(
                "UPDATE semantic_embeddings SET embedding_data=?1",
                [vec![0u8; 2048]],
            )
            .unwrap();
        assert!(
            s.semantic_search(&vector(), &SearchScope::default())
                .is_err()
        );
    }
}
