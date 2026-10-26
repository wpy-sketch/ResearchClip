//! Cache and repository over already confirmed Zotero/PDF provenance.
use super::{
    Storage, StorageError,
    library::{check_id, now},
};
use crate::services::zotero_annotations::{AnnotationReadCache, AnnotationSource};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
type Result<T> = std::result::Result<T, StorageError>;
impl Storage {
    pub fn annotation_source(&self, document_id: &str, source: &AnnotationSource) -> Result<()> {
        check_id(document_id)?;
        source
            .validate()
            .map_err(|_| StorageError::Invalid("批注来源身份无效。"))?;
        if !self.pdf_zotero_sources(document_id)?.iter().any(|s| {
            s.instance_id == source.instance_id
                && s.library_type == source.library_type
                && s.library_id == source.library_id
                && s.item_key == source.item_key
                && s.attachment_key == source.attachment_key
        }) {
            return Err(StorageError::Invalid(
                "此PDF尚未确认该Zotero来源，请先在来源面板核对。",
            ));
        }
        Ok(())
    }
    pub fn annotation_cache(
        &self,
        document_id: &str,
        source: &AnnotationSource,
    ) -> Result<Option<AnnotationReadCache>> {
        self.annotation_source(document_id, source)?;
        let record = self.pdf_record(document_id)?;
        let json:Option<String>=self.connection.query_row("SELECT snapshot_json FROM zotero_annotation_cache WHERE document_id=?1 AND instance_id=?2 AND library_type=?3 AND library_id=?4 AND attachment_key=?5 AND content_hash=?6",params![document_id,source.instance_id,source.library_type,source.library_id,source.attachment_key,record.content_hash],|r|r.get(0)).optional()?;
        json.map(|json| {
            if json.len() > 8 * 1024 * 1024 {
                return Err(StorageError::Invalid("批注缓存超出预算，未删除原缓存。"));
            }
            let cache: AnnotationReadCache = serde_json::from_str(&json)
                .map_err(|_| StorageError::Invalid("批注缓存损坏或未来格式，未重置。"))?;
            cache
                .validate()
                .map_err(|_| StorageError::Invalid("批注缓存格式或坐标无效。"))?;
            if cache.source != *source || cache.pdf_content_hash != record.content_hash {
                return Err(StorageError::Conflict);
            }
            Ok(cache)
        })
        .transpose()
    }
    pub fn save_annotation_cache(
        &mut self,
        document_id: &str,
        cache: &AnnotationReadCache,
    ) -> Result<()> {
        self.annotation_source(document_id, &cache.source)?;
        cache
            .validate()
            .map_err(|_| StorageError::Invalid("批注快照无效。"))?;
        let record = self.pdf_record(document_id)?;
        if record.content_hash != cache.pdf_content_hash {
            return Err(StorageError::Conflict);
        }
        let json = serde_json::to_string(cache)
            .map_err(|_| StorageError::Invalid("批注快照序列化失败。"))?;
        if json.len() > 8 * 1024 * 1024 {
            return Err(StorageError::Invalid("批注快照超过8MiB，旧缓存保留。"));
        }
        let s = &cache.source;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let(total,old):(u64,u64)=tx.query_row("SELECT coalesce(sum(length(cast(snapshot_json AS BLOB))),0),coalesce(sum(CASE WHEN document_id=?1 AND instance_id=?2 AND library_type=?3 AND library_id=?4 AND attachment_key=?5 AND content_hash=?6 THEN length(cast(snapshot_json AS BLOB)) ELSE 0 END),0) FROM zotero_annotation_cache",params![document_id,s.instance_id,s.library_type,s.library_id,s.attachment_key,cache.pdf_content_hash],|r|Ok((r.get(0)?,r.get(1)?)))?;
        if total - old + json.len() as u64 > 64 * 1024 * 1024 {
            return Err(StorageError::Invalid(
                "只读批注缓存达到64MiB上限，旧缓存保留。",
            ));
        }
        let previous:Option<i64>=tx.query_row("SELECT fetched_at FROM zotero_annotation_cache WHERE document_id=?1 AND instance_id=?2 AND library_type=?3 AND library_id=?4 AND attachment_key=?5 AND content_hash=?6",params![document_id,s.instance_id,s.library_type,s.library_id,s.attachment_key,cache.pdf_content_hash],|r|r.get(0)).optional()?;
        if previous.is_some_and(|time| time > cache.fetched_at) {
            return Err(StorageError::Conflict);
        }
        tx.execute("INSERT INTO zotero_annotation_cache VALUES('document',?1,?2,?3,?4,?5,?6,1,?7,?8) ON CONFLICT(document_id,instance_id,library_type,library_id,attachment_key,content_hash) DO UPDATE SET fetched_at=excluded.fetched_at,snapshot_json=excluded.snapshot_json",params![document_id,s.instance_id,s.library_type,s.library_id,s.attachment_key,cache.pdf_content_hash,cache.fetched_at.min(now()?),json])?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{
        zotero_annotations::{AnnotationReadCache, parse_annotation},
        zotero_local::{ZoteroHealth, ZoteroScope, instance_identity, metadata_candidate},
    };
    use serde_json::json;
    fn bind(
        s: &mut Storage,
        document_id: &str,
        hash: &str,
        kind: &str,
        id: u64,
    ) -> AnnotationSource {
        let h = ZoteroHealth {
            api_version: 3,
            schema_version: Some(44),
            instance_id: instance_identity("AnnotationCacheFixture").unwrap(),
            product_version: Some("10.0.6-fixture".into()),
            server_id: "AnnotationCacheFixture".into(),
        };
        let scope = ZoteroScope {
            library_type: kind.into(),
            library_id: if kind == "user" { 0 } else { id },
        };
        let attachment = json!({"key":"ATTACH01","version":1,"library":{"type":kind,"id":id},"data":{"key":"ATTACH01","itemType":"attachment","contentType":"application/pdf","parentItem":"PARENT01"}});
        let parent = json!({"key":"PARENT01","version":1,"library":{"type":kind,"id":id},"data":{"key":"PARENT01","itemType":"journalArticle","title":format!("虚构只读缓存练习 {kind}")}});
        let c = metadata_candidate(&h, &scope, &attachment, &parent).unwrap();
        s.bind_pdf_zotero(document_id, hash, &c).unwrap();
        AnnotationSource {
            instance_id: c.instance_id,
            library_type: kind.into(),
            library_id: id,
            item_key: "PARENT01".into(),
            attachment_key: "ATTACH01".into(),
        }
    }
    fn cache(source: &AnnotationSource, hash: &str) -> AnnotationReadCache {
        let raw = json!({"key":"ANN00001","version":1,"library":{"type":source.library_type,"id":source.library_id},"data":{"parentItem":"ATTACH01","itemType":"annotation","annotationType":"highlight","annotationColor":"#ffd400","annotationText":"虚构批注","annotationComment":"","annotationPosition":"{\"pageIndex\":0,\"rects\":[[10,20,110,40]]}"}});
        AnnotationReadCache {
            format_version: 1,
            source: source.clone(),
            pdf_content_hash: hash.into(),
            fetched_at: 1,
            product_version: Some("10.0.6-fixture".into()),
            library_version: Some(1),
            annotations: vec![parse_annotation(&raw, source).unwrap()],
            truncated: false,
        }
    }
    #[test]
    fn migration18_keeps_existing_pdf_sources_markdown_and_images_then_cache_survives_restart() {
        use crate::{capture::RawFrame, services::screen_capture::encode_png};
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().join("data")).unwrap();
        let p = s.register_pdf("练习PDF", &"a".repeat(64), 12).unwrap();
        let source = bind(&mut s, &p.document_id, &p.content_hash, "user", 42);
        let png = encode_png(RawFrame {
            width_px: 2,
            height_px: 2,
            rgba: vec![255; 16],
        })
        .unwrap();
        let shot = s
            .create_screenshot(
                "旧截图",
                json!({"schemaVersion":1,"image":{"widthPx":2,"heightPx":2},"objects":[]}),
                &png,
            )
            .unwrap();
        let markdown = t.path().join("笔记.md");
        std::fs::write(&markdown, "# 原稿\n\n$ a  + b $\n").unwrap();
        s.index_document(&markdown, None).unwrap();
        super::super::migrations::discard_annotation_cache_fixture(&s.connection);
        s.connection
            .pragma_update(None, "user_version", 18)
            .unwrap();
        drop(s);
        let mut s = Storage::open(t.path().join("data")).unwrap();
        assert_eq!(s.status().unwrap().schema_version, 19);
        assert_eq!(s.screenshot_image(&shot.id, false).unwrap(), png);
        assert_eq!(
            std::fs::read_to_string(&markdown).unwrap(),
            "# 原稿\n\n$ a  + b $\n"
        );
        assert_eq!(
            s.pdf_zotero_sources(&p.document_id).unwrap()[0].attachment_key,
            "ATTACH01"
        );
        s.save_annotation_cache(&p.document_id, &cache(&source, &p.content_hash))
            .unwrap();
        drop(s);
        let s = Storage::open(t.path().join("data")).unwrap();
        let stored = s
            .annotation_cache(&p.document_id, &source)
            .unwrap()
            .unwrap();
        assert_eq!(stored.annotations[0].identity.annotation_key, "ANN00001");
        assert_eq!(stored.source.library_id, 42);
        assert!(
            std::fs::read_dir(t.path().join("data/backups"))
                .unwrap()
                .any(|x| x
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("schema-18-"))
        );
    }
    #[test]
    fn identical_annotation_keys_in_user_group_and_other_instances_never_share_cache_rows() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().into()).unwrap();
        let p = s.register_pdf("PDF", &"a".repeat(64), 12).unwrap();
        let user = bind(&mut s, &p.document_id, &p.content_hash, "user", 42);
        let group = bind(&mut s, &p.document_id, &p.content_hash, "group", 42);
        let mut u = cache(&user, &p.content_hash);
        u.annotations[0].comment = "个人库".into();
        let mut g = cache(&group, &p.content_hash);
        g.annotations[0].comment = "群组库".into();
        s.save_annotation_cache(&p.document_id, &u).unwrap();
        s.save_annotation_cache(&p.document_id, &g).unwrap();
        assert_eq!(
            s.annotation_cache(&p.document_id, &user)
                .unwrap()
                .unwrap()
                .annotations[0]
                .comment,
            "个人库"
        );
        assert_eq!(
            s.annotation_cache(&p.document_id, &group)
                .unwrap()
                .unwrap()
                .annotations[0]
                .comment,
            "群组库"
        );
        let mut foreign = user.clone();
        foreign.instance_id = "b".repeat(32);
        assert!(s.annotation_cache(&p.document_id, &foreign).is_err());
    }
    #[test]
    fn invalid_oversized_or_failed_cache_commit_preserves_the_previous_snapshot() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().into()).unwrap();
        let p = s.register_pdf("PDF", &"a".repeat(64), 12).unwrap();
        let source = bind(&mut s, &p.document_id, &p.content_hash, "user", 42);
        let original = cache(&source, &p.content_hash);
        s.save_annotation_cache(&p.document_id, &original).unwrap();
        let mut wrong = original.clone();
        wrong.pdf_content_hash = "b".repeat(64);
        assert!(matches!(
            s.save_annotation_cache(&p.document_id, &wrong),
            Err(StorageError::Conflict)
        ));
        let mut oversized = original.clone();
        oversized.annotations = (0..130)
            .map(|i| {
                let mut a = original.annotations[0].clone();
                a.identity.annotation_key = format!("N{i:07}");
                a.comment = "x".repeat(65536);
                a
            })
            .collect();
        assert!(s.save_annotation_cache(&p.document_id, &oversized).is_err());
        s.connection.execute_batch("CREATE TRIGGER fail_annotation_cache BEFORE UPDATE ON zotero_annotation_cache BEGIN SELECT RAISE(ABORT,'owned test fault'); END;").unwrap();
        let mut next = original.clone();
        next.fetched_at = 2;
        next.annotations[0].version = 2;
        assert!(s.save_annotation_cache(&p.document_id, &next).is_err());
        assert_eq!(
            s.annotation_cache(&p.document_id, &source)
                .unwrap()
                .unwrap()
                .annotations[0]
                .version,
            1
        );
    }
    #[test]
    fn future_or_corrupt_cache_is_reported_without_deleting_it() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Storage::open(t.path().into()).unwrap();
        let p = s.register_pdf("PDF", &"a".repeat(64), 12).unwrap();
        let source = bind(&mut s, &p.document_id, &p.content_hash, "user", 42);
        let mut future = cache(&source, &p.content_hash);
        s.save_annotation_cache(&p.document_id, &future).unwrap();
        future.format_version = 2;
        let json = serde_json::to_string(&future).unwrap();
        s.connection
            .execute(
                "UPDATE zotero_annotation_cache SET snapshot_json=?1",
                [&json],
            )
            .unwrap();
        assert!(s.annotation_cache(&p.document_id, &source).is_err());
        let retained: String = s
            .connection
            .query_row(
                "SELECT snapshot_json FROM zotero_annotation_cache",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(retained, json);
    }
}
