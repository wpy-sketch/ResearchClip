use super::*;
use crate::services::zotero_write_model::WriteGeometry;
use crate::storage::zotero_annotations::tests::{bind, cache};
fn create() -> AnnotationWriteInput {
    AnnotationWriteInput::Create {
        annotation_type: "highlight".into(),
        color: "#ffd400".into(),
        text: "校准选文".into(),
        comment: "持久草稿".into(),
        geometry: WriteGeometry {
            page_index: 0,
            view: [0.0, 0.0, 600.0, 800.0],
            user_unit: 1.0,
            rectangles: vec![[10.0, 20.0, 100.0, 20.0]],
        },
    }
}
fn update(version: u64, comment: &str) -> AnnotationWriteInput {
    AnnotationWriteInput::Update {
        annotation_key: "ANN00001".into(),
        expected_version: version,
        annotation_type: "highlight".into(),
        color: "#ffd400".into(),
        text: "虚构批注".into(),
        comment: comment.into(),
    }
}
#[test]
fn durable_new_key_local_revision_and_pending_survive_process_reopen_without_a_secret() {
    let t = tempfile::tempdir().unwrap();
    let path = t.path().join("owned");
    let mut s = Storage::open(path.clone()).unwrap();
    let p = s.register_pdf("练习", &"a".repeat(64), 12).unwrap();
    let source = bind(&mut s, &p.document_id, &p.content_hash, "user", 42);
    let e = s
        .stage_annotation_change(&p.document_id, &source, None, None, create())
        .unwrap();
    let key = e.annotation_key.clone();
    let id = e.id.clone();
    assert_eq!(e.status, AnnotationSyncState::LocalDirty);
    assert_ne!(key, id);
    let e = s
        .stage_annotation_change(
            &p.document_id,
            &source,
            Some(&id),
            Some(e.revision),
            create(),
        )
        .unwrap();
    assert_eq!(e.annotation_key, key);
    assert!(matches!(
        s.stage_annotation_change(
            &p.document_id,
            &source,
            Some(&id),
            Some(e.revision - 1),
            create()
        ),
        Err(StorageError::Conflict)
    ));
    let mut e = e;
    e.needs_verification = true;
    e.fail("ZOTERO_WRITE_UNKNOWN", "断线，结果待复核", 2);
    let revision = e.revision;
    let e = s.save_annotation_sync_entry(e, revision, true).unwrap();
    drop(s);
    let s = Storage::open(path).unwrap();
    let restored = s
        .annotation_sync_entry(&p.document_id, &source, &id)
        .unwrap();
    assert_eq!(restored, e);
    assert!(restored.needs_verification);
    assert_eq!(restored.local, Some(create()));
    assert!(!serde_json::to_string(&restored).unwrap().contains("apiKey"));
}
#[test]
fn two_clients_keep_both_versions_and_conflict_resolution_archives_failed_input() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().join("data")).unwrap();
    let p = s.register_pdf("PDF", &"a".repeat(64), 12).unwrap();
    let source = bind(&mut s, &p.document_id, &p.content_hash, "user", 42);
    let mut remote = cache(&source, &p.content_hash);
    s.save_annotation_cache(&p.document_id, &remote).unwrap();
    s.observe_annotation_sync(&p.document_id, &remote).unwrap();
    let local = s
        .stage_annotation_change(
            &p.document_id,
            &source,
            None,
            None,
            update(1, "客户端B本地"),
        )
        .unwrap();
    assert_eq!(local.status, AnnotationSyncState::LocalDirty);
    let id = local.id.clone();
    remote.annotations[0].version = 2;
    remote.annotations[0].comment = "客户端A远端".into();
    remote.fetched_at = 2;
    s.observe_annotation_sync(&p.document_id, &remote).unwrap();
    let conflict = s
        .annotation_sync_entry(&p.document_id, &source, &id)
        .unwrap();
    assert_eq!(conflict.status, AnnotationSyncState::Conflict);
    assert_eq!(conflict.local, Some(update(1, "客户端B本地")));
    assert_eq!(conflict.base.as_ref().unwrap().version, 1);
    assert_eq!(conflict.remote.as_ref().unwrap().comment, "客户端A远端");
    let mut resolved = conflict.clone();
    resolved.accept_remote(3, "显式采用远端，本地旧输入保留修订");
    let rev = resolved.revision;
    s.save_annotation_sync_entry(resolved, rev, true).unwrap();
    let history = s
        .annotation_sync_history(&p.document_id, &source, &id)
        .unwrap();
    let old = s
        .annotation_sync_revision(&p.document_id, &source, &id, &history[0].revision_id)
        .unwrap();
    assert_eq!(old, conflict);
    assert!(
        s.save_annotation_sync_entry(local.clone(), local.revision, true)
            .is_err()
    );
}
#[test]
fn truncated_children_never_prove_deletion_and_get404_can_settle_a_pending_delete() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().join("data")).unwrap();
    let p = s.register_pdf("PDF", &"a".repeat(64), 12).unwrap();
    let source = bind(&mut s, &p.document_id, &p.content_hash, "user", 42);
    let mut remote = cache(&source, &p.content_hash);
    s.save_annotation_cache(&p.document_id, &remote).unwrap();
    let e = s
        .stage_annotation_change(
            &p.document_id,
            &source,
            None,
            None,
            AnnotationWriteInput::Delete {
                annotation_key: "ANN00001".into(),
                expected_version: 1,
            },
        )
        .unwrap();
    remote.annotations.clear();
    remote.truncated = true;
    remote.fetched_at = 2;
    s.observe_annotation_sync(&p.document_id, &remote).unwrap();
    let mut e = s
        .annotation_sync_entry(&p.document_id, &source, &e.id)
        .unwrap();
    assert!(e.remote.is_some());
    assert!(e.local.is_some());
    e.needs_verification = true;
    e.observe(None, false, 3);
    assert_eq!(e.status, AnnotationSyncState::Conflict);
    assert!(e.local.is_some());
    e.observe(None, true, 4);
    assert_eq!(e.status, AnnotationSyncState::Synced);
    assert!(e.local.is_none());
}
#[test]
fn same_input_converges_without_writes_and_unchanged_poll_does_not_grow_revisions() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().join("data")).unwrap();
    let p = s.register_pdf("PDF", &"a".repeat(64), 12).unwrap();
    let source = bind(&mut s, &p.document_id, &p.content_hash, "user", 42);
    let mut c = cache(&source, &p.content_hash);
    s.save_annotation_cache(&p.document_id, &c).unwrap();
    let e = s
        .stage_annotation_change(&p.document_id, &source, None, None, update(1, "desired"))
        .unwrap();
    c.annotations[0].version = 2;
    c.annotations[0].comment = "desired".into();
    s.observe_annotation_sync(&p.document_id, &c).unwrap();
    let synced = s
        .annotation_sync_entry(&p.document_id, &source, &e.id)
        .unwrap();
    assert_eq!(synced.status, AnnotationSyncState::Synced);
    let n = s
        .annotation_sync_history(&p.document_id, &source, &e.id)
        .unwrap()
        .len();
    c.fetched_at = 2;
    s.observe_annotation_sync(&p.document_id, &c).unwrap();
    assert_eq!(
        s.annotation_sync_history(&p.document_id, &source, &e.id)
            .unwrap()
            .len(),
        n
    );
}
#[test]
fn independent_libraries_scope_refresh_options_and_immutable_key_are_validated() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().join("data")).unwrap();
    let p = s.register_pdf("PDF", &"a".repeat(64), 12).unwrap();
    let user = bind(&mut s, &p.document_id, &p.content_hash, "user", 42);
    let group = bind(&mut s, &p.document_id, &p.content_hash, "group", 7);
    let e = s
        .stage_annotation_change(&p.document_id, &user, None, None, create())
        .unwrap();
    assert!(
        s.annotation_sync_entry(&p.document_id, &group, &e.id)
            .is_err()
    );
    let mut scope = AnnotationSyncScope {
        refresh_interval: 1,
        ..Default::default()
    };
    assert!(
        s.save_annotation_sync_scope(&p.document_id, &user, &scope)
            .is_err()
    );
    scope.refresh_interval = 30;
    s.save_annotation_sync_scope(&p.document_id, &user, &scope)
        .unwrap();
    assert_eq!(
        s.annotation_sync_snapshot(&p.document_id, &group)
            .unwrap()
            .scope
            .refresh_interval,
        0
    );
    let mut changed = e.clone();
    changed.annotation_key = "OTHER001".into();
    assert!(
        s.save_annotation_sync_entry(changed, e.revision, false)
            .is_err()
    );
}
#[test]
fn migration19_preserves_originals_markdown_cache_and_failed20_rolls_back() {
    use crate::{capture::RawFrame, services::screen_capture::encode_png};
    let t = tempfile::tempdir().unwrap();
    let path = t.path().join("data");
    let mut s = Storage::open(path.clone()).unwrap();
    let p = s.register_pdf("旧PDF", &"a".repeat(64), 12).unwrap();
    let source = bind(&mut s, &p.document_id, &p.content_hash, "user", 42);
    let c = cache(&source, &p.content_hash);
    s.save_annotation_cache(&p.document_id, &c).unwrap();
    let png = encode_png(RawFrame {
        width_px: 2,
        height_px: 2,
        rgba: vec![255; 16],
    })
    .unwrap();
    let shot = s
        .create_screenshot(
            "旧截图",
            serde_json::json!({"schemaVersion":1,"image":{"widthPx":2,"heightPx":2},"objects":[]}),
            &png,
        )
        .unwrap();
    let md = t.path().join("原笔记.md");
    std::fs::write(&md, "# 原稿\n\n![旧图](assets/keep.png)\n\n$$ a  +b $$\n").unwrap();
    let assets = t.path().join("assets");
    std::fs::create_dir(&assets).unwrap();
    std::fs::write(assets.join("keep.png"), &png).unwrap();
    let text = std::fs::read(&md).unwrap();
    s.index_document(&md, None).unwrap();
    super::super::migrations::discard_annotation_sync_fixture(&s.connection);
    s.connection
        .pragma_update(None, "user_version", 19)
        .unwrap();
    drop(s);
    let s = Storage::open(path.clone()).unwrap();
    assert_eq!(s.status().unwrap().schema_version, 20);
    assert_eq!(s.screenshot_image(&shot.id, false).unwrap(), png);
    assert_eq!(std::fs::read(md).unwrap(), text);
    assert_eq!(std::fs::read(assets.join("keep.png")).unwrap(), png);
    assert_eq!(
        s.annotation_cache(&p.document_id, &source)
            .unwrap()
            .unwrap()
            .annotations,
        c.annotations
    );
    assert!(std::fs::read_dir(path.join("backups")).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("schema-19-")
    }));
    super::super::migrations::discard_annotation_sync_fixture(&s.connection);
    s.connection
        .pragma_update(None, "user_version", 19)
        .unwrap();
    // Force a real migration20 DDL failure after its first two CREATEs.
    s.connection
        .execute_batch("CREATE TABLE zotero_annotation_sync_scopes (fixture INTEGER);")
        .unwrap();
    drop(s);
    assert!(Storage::open(path.clone()).is_err());
    let check = rusqlite::Connection::open(path.join("researchclip.sqlite3")).unwrap();
    assert_eq!(
        check
            .pragma_query_value::<u32, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        19
    );
    for table in ["zotero_annotation_sync", "zotero_annotation_sync_revisions"] {
        assert_eq!(
            check
                .query_row::<i64, _, _>(
                    "SELECT count(*) FROM sqlite_master WHERE name=?1",
                    [table],
                    |r| r.get(0)
                )
                .unwrap(),
            0
        );
    }
    assert_eq!(
        check
            .query_row::<i64, _, _>("SELECT count(*) FROM zotero_annotation_cache", [], |r| r
                .get(0))
            .unwrap(),
        1
    );
}

#[test]
fn unfinished_draft_is_durable_but_cannot_be_sent_and_permission_failures_keep_input() {
    let t = tempfile::tempdir().unwrap();
    let mut s = Storage::open(t.path().join("data")).unwrap();
    let p = s.register_pdf("PDF", &"a".repeat(64), 12).unwrap();
    let source = bind(&mut s, &p.document_id, &p.content_hash, "user", 42);
    let mut input = create();
    if let AnnotationWriteInput::Create { text, .. } = &mut input {
        *text = String::new();
    }
    assert!(input.validate_draft().is_ok());
    assert!(input.validate().is_err());
    let mut e = s
        .stage_annotation_change(&p.document_id, &source, None, None, input.clone())
        .unwrap();
    for code in [
        "ZOTERO_WRITE_FORBIDDEN",
        "ZOTERO_WRITE_AUTH_REQUIRED",
        "ZOTERO_WRITE_UNAVAILABLE",
    ] {
        let rev = e.revision;
        e.fail(code, "保留输入", 2);
        e = s.save_annotation_sync_entry(e, rev, true).unwrap();
        assert_eq!(e.local, Some(input.clone()));
        assert_eq!(
            e.status,
            if code == "ZOTERO_WRITE_UNAVAILABLE" {
                AnnotationSyncState::Failed
            } else {
                AnnotationSyncState::ReadOnly
            }
        );
    }
    let prior = s
        .annotation_sync_history(&p.document_id, &source, &e.id)
        .unwrap()
        .len();
    let edited = s
        .stage_annotation_change(
            &p.document_id,
            &source,
            Some(&e.id),
            Some(e.revision),
            create(),
        )
        .unwrap();
    assert!(
        s.annotation_sync_history(&p.document_id, &source, &e.id)
            .unwrap()
            .len()
            > prior
    );
    let old = s
        .annotation_sync_history(&p.document_id, &source, &e.id)
        .unwrap();
    let snapshot = s
        .annotation_sync_revision(&p.document_id, &source, &e.id, &old[0].revision_id)
        .unwrap();
    assert_eq!(snapshot.local, Some(input));
    assert_eq!(edited.status, AnnotationSyncState::LocalDirty);
}
