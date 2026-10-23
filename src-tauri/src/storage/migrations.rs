use super::StorageError;
use rusqlite::{Connection, OpenFlags, TransactionBehavior};
use std::path::Path;

pub const SCHEMA_VERSION: u32 = 12;

fn apply_migration(
    connection: &mut Connection,
    version: u32,
    sql: &str,
) -> Result<(), StorageError> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute_batch(sql)?;
    if version >= 2 {
        validate_schema(&transaction, version)?;
    }
    transaction.pragma_update(None, "user_version", version)?;
    transaction.commit()?;
    Ok(())
}

pub fn migrate(connection: &mut Connection, root: Option<&Path>) -> Result<(), StorageError> {
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(StorageError::NewerSchema(version));
    }
    let integrity: String = connection.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(StorageError::Invalid("数据库完整性检查失败，未重置资料。"));
    }
    if version > 0 {
        validate_schema(connection, version)?;
    }
    if version > 0
        && version < SCHEMA_VERSION
        && let Some(root) = root
    {
        let folder = root.join("backups");
        super::files::directory(&folder)?;
        let id: String =
            connection.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;
        let backup = folder.join(format!("schema-{version}-{id}.sqlite3"));
        if backup.try_exists()? {
            return Err(StorageError::Invalid("迁移备份文件已存在。"));
        }
        // VACUUM INTO is a consistent SQLite snapshot, including committed WAL pages.
        let stage = tempfile::NamedTempFile::new_in(&folder)?;
        connection.execute("VACUUM INTO ?1", [stage.path().to_string_lossy().as_ref()])?;
        stage.as_file().sync_all()?;
        let check = Connection::open_with_flags(stage.path(), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        validate_schema(&check, version)?;
        let status: String = check.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if status != "ok" {
            return Err(StorageError::Invalid("迁移备份校验失败，升级未执行。"));
        }
        drop(check);
        stage
            .persist_noclobber(backup)
            .map_err(|e| StorageError::Io(e.error))?;
    }
    if version == 0 {
        apply_migration(
            connection,
            1,
            include_str!("../../migrations/001_workspace.sql"),
        )?;
    }
    if version < 2 {
        apply_migration(
            connection,
            2,
            include_str!("../../migrations/002_library.sql"),
        )?;
    }
    if version < 3 {
        apply_migration(
            connection,
            3,
            include_str!("../../migrations/003_projects.sql"),
        )?;
    }
    if version < 4 {
        apply_migration(
            connection,
            4,
            include_str!("../../migrations/004_ocr_index.sql"),
        )?;
    }
    if version < 5 {
        apply_migration(
            connection,
            5,
            include_str!("../../migrations/005_annotation_revisions.sql"),
        )?;
    }
    if version < 6 {
        apply_migration(connection, 6, include_str!("../../migrations/006_sync.sql"))?;
    }
    if version < 7 {
        apply_migration(
            connection,
            7,
            include_str!("../../migrations/007_image_features.sql"),
        )?;
    }
    if version < 8 {
        apply_migration(
            connection,
            8,
            include_str!("../../migrations/008_references.sql"),
        )?;
    }
    if version < 9 {
        apply_migration(
            connection,
            9,
            include_str!("../../migrations/009_internal_links.sql"),
        )?;
    }
    if version < 10 {
        apply_migration(
            connection,
            10,
            include_str!("../../migrations/010_annotation_anchors.sql"),
        )?;
    }
    if version < 11 {
        apply_migration(
            connection,
            11,
            include_str!("../../migrations/011_semantic_embeddings.sql"),
        )?;
    }
    if version < 12 {
        apply_migration(
            connection,
            12,
            include_str!("../../migrations/012_research_resources.sql"),
        )?;
    }
    validate_schema(connection, SCHEMA_VERSION)?;
    Ok(())
}
fn validate_schema(connection: &Connection, version: u32) -> Result<(), StorageError> {
    if version >= 12 {
        for sql in [
            "SELECT resource_type,id,title,format_version,revision,data_json FROM research_resource_catalog LIMIT 0",
            "SELECT source_type,source_id,target_type,target_id,relation_type,origin FROM research_resource_edges LIMIT 0",
            "SELECT instance_id,library_id,item_key,resource_type,resource_id FROM research_zotero_keys LIMIT 0",
            "SELECT document_id,content_hash,byte_size FROM research_document_fingerprints LIMIT 0",
        ] {
            connection.prepare(sql)?;
        }
    }

    if version >= 11 {
        connection.prepare("SELECT screenshot_id,embedding_model_id,embedding_version,image_hash,embedding_data,source_stamp,updated_at FROM semantic_embeddings LIMIT 0")?;
    }
    if version >= 10 {
        connection.prepare("SELECT anchor_id,screenshot_id,image_revision,x,y,width,height,title,description,revision FROM annotation_anchors LIMIT 0")?;
        connection.prepare("SELECT anchor_id FROM link_index LIMIT 0")?;
    }
    if version >= 9 {
        for sql in [
            "SELECT source_id,target_id,link_type,source_location,source_end,source_line,label,updated_at FROM link_index LIMIT 0",
            "SELECT document_id,source_stamp,available,updated_at FROM link_index_state LIMIT 0",
            "SELECT document_id,tag_id FROM document_tags LIMIT 0",
        ] {
            connection.prepare(sql)?;
        }
    }
    if version >= 8 {
        for sql in [
            "SELECT id,title,authors_json,year,venue,doi,url,bibtex_key,bibtex_entry,verified_source,source_metadata,updated_at,revision FROM literature_references LIMIT 0",
            "SELECT screenshot_id,reference_id FROM screenshot_references LIMIT 0",
            "SELECT document_id,reference_id FROM document_references LIMIT 0",
        ] {
            connection.prepare(sql)?;
        }
    }
    if version >= 7 {
        connection.prepare("SELECT screenshot_id,content_hash,perceptual_hash,difference_hash,descriptor,hash_algorithm_version,source_stamp,status,error,updated_at FROM image_features LIMIT 0")?;
    }
    // Verify the known schema before reporting storage as ready.
    let _: String = connection.query_row(
        "SELECT value FROM app_metadata WHERE key = 'workspace_initialized_at'",
        [],
        |row| row.get(0),
    )?;
    if version >= 2 {
        for sql in [
            "SELECT id,title,original_image_path,thumbnail_path,annotation_data,width_px,height_px,created_at,updated_at,is_favorite,revision FROM screenshots LIMIT 0",
            "SELECT id,screenshot_id,content,source_url,created_at,updated_at,revision FROM notes LIMIT 0",
            "SELECT id,name,normalized_name FROM tags LIMIT 0",
            "SELECT screenshot_id,tag_id FROM screenshot_tags LIMIT 0",
            "SELECT id,title,annotation_data,width_px,height_px,created_at FROM pending_screenshots LIMIT 0",
            "SELECT relative_path,screenshot_id FROM pending_file_deletions LIMIT 0",
        ] {
            connection.prepare(sql)?;
        }
        if connection
            .prepare("PRAGMA foreign_key_check")?
            .query([])?
            .next()?
            .is_some()
        {
            return Err(StorageError::Invalid("资料外键校验失败，未重置数据库。"));
        }
    }
    if version >= 3 {
        for sql in [
            "SELECT id,name,normalized_name,created_at,updated_at FROM projects LIMIT 0",
            "SELECT screenshot_id,project_id FROM screenshot_projects LIMIT 0",
            "SELECT id,path,name,project_id,created_at,updated_at FROM markdown_documents LIMIT 0",
        ] {
            connection.prepare(sql)?;
        }
    }
    if version >= 5 {
        connection.prepare("SELECT revision_id,screenshot_id,version_number,parent_revision_id,annotation_json,created_at,description,schema_version FROM annotation_revisions LIMIT 0")?;
    }
    if version >= 6 {
        for sql in [
            "SELECT screenshot_id,revision_id,needs_review FROM annotation_heads LIMIT 0",
            "SELECT device_id,counter FROM sync_identity LIMIT 0",
            "SELECT connection_id,resource_key,content_hash,manifest_hash,remote_etag FROM sync_state LIMIT 0",
            "SELECT operation_id,phase FROM sync_queue LIMIT 0",
            "SELECT id,resolved FROM sync_conflicts LIMIT 0",
            "SELECT resource_key,project_id FROM sync_tombstones LIMIT 0",
            "SELECT document_id,authorized_path FROM sync_document_grants LIMIT 0",
        ] {
            connection.prepare(sql)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partially_applied_migration_rolls_back_ddl_and_version() {
        let mut connection = Connection::open_in_memory().unwrap();
        let result = apply_migration(
            &mut connection,
            1,
            "CREATE TABLE partially_created (id INTEGER); INSERT INTO nonexistent VALUES (1);",
        );
        assert!(result.is_err());
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 0);
        let tables: u32 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 0);
    }
    #[test]
    fn legacy_wal_upgrade_backs_up_metadata_and_never_rewrites_v1_migration() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("researchclip.sqlite3");
        let mut legacy = Connection::open(&path).unwrap();
        apply_migration(
            &mut legacy,
            1,
            include_str!("../../migrations/001_workspace.sql"),
        )
        .unwrap();
        legacy.pragma_update(None, "journal_mode", "WAL").unwrap();
        legacy
            .execute("INSERT INTO app_metadata VALUES ('research','保留')", [])
            .unwrap();
        let storage = crate::storage::Storage::open(tmp.path().into()).unwrap();
        assert_eq!(storage.status().unwrap().schema_version, SCHEMA_VERSION);
        let backup = std::fs::read_dir(tmp.path().join("backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let snapshot =
            Connection::open_with_flags(backup, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(
            snapshot
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            snapshot
                .query_row(
                    "SELECT value FROM app_metadata WHERE key='research'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            "保留"
        );
        assert_eq!(
            legacy
                .query_row(
                    "SELECT value FROM app_metadata WHERE key='research'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            "保留"
        );
    }
    #[test]
    fn failed_upgrade_preserves_old_version_and_consistent_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("researchclip.sqlite3");
        let mut conn = Connection::open(path).unwrap();
        apply_migration(
            &mut conn,
            1,
            include_str!("../../migrations/001_workspace.sql"),
        )
        .unwrap();
        conn.execute_batch("CREATE TABLE screenshots (unexpected TEXT);")
            .unwrap();
        assert!(crate::storage::Storage::open(tmp.path().into()).is_err());
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            std::fs::read_dir(tmp.path().join("backups"))
                .unwrap()
                .count(),
            1
        );
        assert!(conn.prepare("SELECT * FROM notes").is_err());
    }
    #[test]
    fn backup_failure_stops_upgrade_and_preserves_existing_files() {
        let tmp = tempfile::tempdir().unwrap();
        let mut conn = Connection::open(tmp.path().join("researchclip.sqlite3")).unwrap();
        apply_migration(
            &mut conn,
            1,
            include_str!("../../migrations/001_workspace.sql"),
        )
        .unwrap();
        std::fs::write(tmp.path().join("backups"), b"keep").unwrap();
        assert!(crate::storage::Storage::open(tmp.path().into()).is_err());
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        assert_eq!(std::fs::read(tmp.path().join("backups")).unwrap(), b"keep");
    }
}
