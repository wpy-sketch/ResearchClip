use super::*;
#[test]
fn original_asset_hash_and_file_grant_are_checked_before_restore_packet() {
    use crate::{
        capture::RawFrame,
        services::{markdown_document::DocumentFiles, screen_capture::encode_png},
    };
    let (_t, mut s, p) = store();
    let assets = p.parent().unwrap().join("assets");
    std::fs::create_dir(&assets).unwrap();
    let name = format!("researchclip-{}.png", "e".repeat(32));
    let png = |value| {
        encode_png(RawFrame {
            width_px: 1,
            height_px: 1,
            rgba: vec![value, 0, 0, 255],
        })
        .unwrap()
    };
    std::fs::write(assets.join(&name), png(1)).unwrap();
    let raw = format!("![图](assets/{name})\r\n\r\n注释");
    std::fs::write(&p, &raw).unwrap();
    let id = s.index_document(&p, None).unwrap().id;
    let row = s.markdown_history(&id, i64::MAX).unwrap().remove(0);
    let revision = s.markdown_revision(&id, &row.metadata.revision_id).unwrap();
    let mut files = DocumentFiles::default();
    let packet = files.open(&p).unwrap();
    let size = u32::from_le_bytes(packet[..4].try_into().unwrap()) as usize;
    let meta: serde_json::Value = serde_json::from_slice(&packet[4..4 + size]).unwrap();
    let token = meta["token"].as_str().unwrap();
    assert!(
        files
            .revision_packet(token, &revision.markdown, &revision.assets)
            .is_ok()
    );
    std::fs::write(assets.join(&name), png(2)).unwrap();
    assert!(
        files
            .revision_packet(token, &revision.markdown, &revision.assets)
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(&p).unwrap(), raw);
    std::fs::write(assets.join(&name), png(1)).unwrap();
    std::fs::write(&p, "external edit").unwrap();
    assert!(
        files
            .revision_packet(token, &revision.markdown, &revision.assets)
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "external edit");
}
fn store() -> (tempfile::TempDir, Storage, PathBuf) {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("draft.md");
    std::fs::write(&p, "# 原稿\r\n\r\n$x$\r\n").unwrap();
    let s = Storage::open(t.path().join("data")).unwrap();
    (t, s, p)
}
#[test]
fn snapshots_append_deduplicate_and_restore_as_new_child() {
    let (_t, mut s, p) = store();
    let id = s.index_document(&p, None).unwrap().id;
    let first = s.markdown_history(&id, i64::MAX).unwrap().remove(0);
    s.index_document(&p, None).unwrap();
    assert_eq!(s.markdown_history(&id, i64::MAX).unwrap().len(), 1);
    let old = s
        .markdown_revision(&id, &first.metadata.revision_id)
        .unwrap()
        .markdown;
    std::fs::write(&p, "new").unwrap();
    s.index_document(&p, None).unwrap();
    std::fs::write(&p, &old).unwrap();
    s.index_document_revision(&p, None, Some(&first.metadata.revision_id), false)
        .unwrap();
    let rows = s.markdown_history(&id, i64::MAX).unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].metadata.version_number, 3);
    assert_eq!(
        rows[0].metadata.parent_revision_id,
        Some(rows[1].metadata.revision_id.clone())
    );
    assert_eq!(
        rows[0].restored_from,
        Some(first.metadata.revision_id.clone())
    );
    assert_eq!(
        s.markdown_revision(&id, &rows[0].metadata.revision_id)
            .unwrap()
            .markdown,
        old
    );
    s.index_document_revision(&p, None, Some(&first.metadata.revision_id), false)
        .unwrap();
    assert_eq!(s.markdown_history(&id, i64::MAX).unwrap().len(), 4);
}
#[test]
fn revisions_are_immutable_owned_and_do_not_mix_screenshots() {
    let (_t, mut s, p) = store();
    let id = s.index_document(&p, None).unwrap().id;
    let r = s.markdown_history(&id, i64::MAX).unwrap().remove(0);
    assert!(
        s.markdown_revision(&"e".repeat(32), &r.metadata.revision_id)
            .is_err()
    );
    assert!(
        s.connection
            .execute("UPDATE markdown_revisions SET markdown='tampered'", [])
            .is_err()
    );
    assert!(
        s.connection
            .execute("DELETE FROM markdown_revisions", [])
            .is_err()
    );
    let n: i64 = s
        .connection
        .query_row("SELECT count(*) FROM annotation_revisions", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n, 0);
}
#[test]
fn directory_favorite_folder_and_recent_survive_restart() {
    let (t, mut s, p) = store();
    let id = s.index_document_revision(&p, None, None, true).unwrap().id;
    let root = s.create_document_folder(None, "研究").unwrap();
    let child = s.create_document_folder(Some(&root), "数学").unwrap();
    s.organize_document(&id, Some(&child), true).unwrap();
    assert!(s.delete_document_folder(&root).is_err());
    assert!(s.delete_document_folder(&child).is_err());
    assert!(s.create_document_folder(None, "研究").is_err());
    drop(s);
    let s = Storage::open(t.path().join("data")).unwrap();
    let d = s.document_directory().unwrap();
    assert_eq!(d.documents[0].id, id);
    assert!(d.documents[0].favorite);
    assert!(d.documents[0].last_opened_at > 0);
    assert_eq!(d.documents[0].folder_id, Some(child));
    assert_eq!(d.folders.len(), 2);
}
#[test]
fn depth_and_invalid_identifiers_are_rejected() {
    let (_t, mut s, _p) = store();
    let mut parent = None;
    for _ in 0..8 {
        parent = Some(s.create_document_folder(parent.as_deref(), "层").unwrap());
    }
    assert!(s.create_document_folder(parent.as_deref(), "过深").is_err());
    assert!(s.markdown_history("invalid", 9).is_err());
    assert!(s.markdown_history(&"a".repeat(32), 0).is_err());
    assert!(s.create_document_folder(None, "a\nb").is_err());
}
#[test]
fn external_move_rename_and_deletion_retain_identity_history_and_favorite() {
    let (_t, mut s, p) = store();
    let id = s.index_document(&p, None).unwrap().id;
    s.organize_document(&id, None, true).unwrap();
    let renamed = p.with_file_name("renamed.md");
    std::fs::rename(&p, &renamed).unwrap();
    assert_eq!(s.index_document(&renamed, Some(&p)).unwrap().id, id);
    let (old, hash) = s.document_location(&id).unwrap();
    let moved = renamed.with_file_name("moved.md");
    std::fs::rename(old, &moved).unwrap();
    s.relink_markdown_resource(&id, &hash, &moved).unwrap();
    assert_eq!(s.document_directory().unwrap().documents[0].id, id);
    std::fs::remove_file(&moved).unwrap();
    let row = s.document_directory().unwrap().documents.remove(0);
    assert!(!row.available);
    assert!(row.favorite);
    assert_eq!(s.markdown_history(&id, i64::MAX).unwrap().len(), 1);
}
#[test]
fn migration_12_preserves_files_and_metadata_and_takes_valid_snapshot() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("data");
    std::fs::create_dir(&root).unwrap();
    let db = root.join("researchclip.sqlite3");
    let c = Connection::open(&db).unwrap();
    let scripts = [
        include_str!("../../../migrations/001_workspace.sql"),
        include_str!("../../../migrations/002_library.sql"),
        include_str!("../../../migrations/003_projects.sql"),
        include_str!("../../../migrations/004_ocr_index.sql"),
        include_str!("../../../migrations/005_annotation_revisions.sql"),
        include_str!("../../../migrations/006_sync.sql"),
        include_str!("../../../migrations/007_image_features.sql"),
        include_str!("../../../migrations/008_references.sql"),
        include_str!("../../../migrations/009_internal_links.sql"),
        include_str!("../../../migrations/010_annotation_anchors.sql"),
        include_str!("../../../migrations/011_semantic_embeddings.sql"),
        include_str!("../../../migrations/012_research_resources.sql"),
    ];
    for (i, sql) in scripts.iter().enumerate() {
        c.execute_batch(sql).unwrap();
        c.pragma_update(None, "user_version", i + 1).unwrap();
    }
    let p = t.path().join("old.md");
    std::fs::write(&p, "旧稿\r\n$x$\r\n").unwrap();
    c.execute(
        "INSERT INTO markdown_documents VALUES(?1,?2,'old.md',NULL,1,1)",
        params!["a".repeat(32), p.to_str().unwrap()],
    )
    .unwrap();
    drop(c);
    let before = std::fs::read(&p).unwrap();
    let s = Storage::open(root.clone()).unwrap();
    assert_eq!(s.status().unwrap().schema_version, 13);
    assert_eq!(std::fs::read(&p).unwrap(), before);
    assert_eq!(
        s.document_directory().unwrap().documents[0].id,
        "a".repeat(32)
    );
    assert!(
        s.markdown_history(&"a".repeat(32), i64::MAX)
            .unwrap()
            .is_empty()
    );
    let snapshot = std::fs::read_dir(root.join("backups"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let c =
        Connection::open_with_flags(snapshot, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        12
    );
}
#[test]
fn corrupt_history_and_failed_migration_never_succeed_silently() {
    let (_t, mut s, p) = store();
    let id = s.index_document(&p, None).unwrap().id;
    let r = s.markdown_history(&id, i64::MAX).unwrap().remove(0);
    s.connection.execute_batch("DROP TRIGGER markdown_revision_no_update; UPDATE markdown_revisions SET markdown='bad';").unwrap();
    assert!(s.markdown_revision(&id, &r.metadata.revision_id).is_err());
    let t = tempfile::tempdir().unwrap();
    let mut c = Connection::open_in_memory().unwrap();
    super::super::migrations::migrate(&mut c, None).unwrap();
    c.execute_batch("DROP TRIGGER markdown_revision_no_update; DROP TRIGGER markdown_revision_no_delete; DROP TABLE markdown_revisions; DROP TABLE document_organization; PRAGMA user_version=12;").unwrap();
    assert!(super::super::migrations::migrate(&mut c, Some(t.path())).is_err());
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        12
    );
    assert!(c.prepare("SELECT * FROM document_organization").is_err());
}
