pub mod anchors;
pub mod citations;
pub mod clips;
pub mod document_organization;
pub mod embeddings;
pub(crate) mod files;
pub mod knowledge;
pub mod library;
pub mod links;
pub mod markdown;
mod migrations;
pub mod ocr;
pub mod projects;
pub mod references;
pub mod resources;
pub mod revision_meta;
pub mod revisions;
pub mod search;
pub mod similarity;
pub mod sync;

use crate::domain::WorkspaceStatus;
use rusqlite::Connection;
use std::{fs, path::PathBuf, time::Duration};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("文件系统错误：{0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite 错误：{0}")]
    Database(#[from] rusqlite::Error),
    #[error("数据库版本 {0} 高于当前应用支持版本，请使用较新版本的 ResearchClip")]
    NewerSchema(u32),
    #[error("{0}")]
    Invalid(&'static str),
    #[error("资料不存在或已删除。")]
    NotFound,
    #[error("资料已被其他操作修改，当前草稿已保留。请先保留草稿，再重新加载已保存资料。")]
    Conflict,
}

pub struct Storage {
    connection: Connection,
    data_dir: PathBuf,
    database_path: PathBuf,
    recovery_warnings: Vec<String>,
    _workspace_lock: fs::File,
}

impl Storage {
    pub fn open(data_dir: PathBuf) -> Result<Self, StorageError> {
        files::existing_ordinary(&data_dir)?;
        fs::create_dir_all(&data_dir)?;
        let data_dir = data_dir.canonicalize()?;
        files::ordinary(&data_dir)?;
        let lock_path = data_dir.join("workspace.lock");
        files::existing_ordinary(&lock_path)?;
        let workspace_lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        workspace_lock.try_lock().map_err(|_| {
            StorageError::Invalid(
                "资料库正被另一个应用实例使用，或无法取得工作区锁；请关闭该实例后重试。",
            )
        })?;
        let database_path = data_dir.join("researchclip.sqlite3");
        files::existing_ordinary(&database_path)?;
        for suffix in ["-wal", "-shm"] {
            let path = data_dir.join(format!("researchclip.sqlite3{suffix}"));
            files::existing_ordinary(&path)?;
        }
        let mut connection = Connection::open(&database_path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        // Refuse unknown future schemas before changing any database settings.
        migrations::migrate(&mut connection, Some(&data_dir))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        for directory in ["originals", "thumbnails", "exports"] {
            files::directory(&data_dir.join(directory))?;
        }
        let mut storage = Self {
            connection,
            data_dir,
            database_path,
            recovery_warnings: Vec::new(),
            _workspace_lock: workspace_lock,
        };
        storage.reconcile()?;
        storage.recover_ocr()?;
        Ok(storage)
    }

    pub fn status(&self) -> Result<WorkspaceStatus, StorageError> {
        Ok(WorkspaceStatus {
            app_name: "ResearchClip",
            app_version: env!("CARGO_PKG_VERSION"),
            platform: std::env::consts::OS,
            data_dir: self.data_dir.to_string_lossy().into_owned(),
            database_path: self.database_path.to_string_lossy().into_owned(),
            schema_version: self
                .connection
                .pragma_query_value(None, "user_version", |row| row.get(0))?,
            sqlite_version: self
                .connection
                .query_row("SELECT sqlite_version()", [], |row| row.get(0))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_workspace_and_real_database_in_unicode_path() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("科研工作区");
        let storage = Storage::open(root.clone()).unwrap();
        let status = storage.status().unwrap();
        assert_eq!(
            status.schema_version,
            crate::storage::migrations::SCHEMA_VERSION
        );
        assert!(root.join("researchclip.sqlite3").is_file());
        for name in ["originals", "thumbnails", "exports"] {
            assert!(root.join(name).is_dir());
        }
        let foreign_keys: u32 = storage
            .connection
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .unwrap();
        assert_eq!(foreign_keys, 1);
        let journal: String = storage
            .connection
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert_eq!(journal, "wal");
    }

    #[test]
    fn reopening_preserves_existing_metadata() {
        let temporary = tempfile::tempdir().unwrap();
        {
            let storage = Storage::open(temporary.path().to_path_buf()).unwrap();
            storage
                .connection
                .execute(
                    "INSERT INTO app_metadata VALUES ('test_note', 'keep me')",
                    [],
                )
                .unwrap();
        }
        let storage = Storage::open(temporary.path().to_path_buf()).unwrap();
        let note: String = storage
            .connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = 'test_note'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(note, "keep me");
        assert_eq!(
            storage.status().unwrap().schema_version,
            crate::storage::migrations::SCHEMA_VERSION
        );
    }

    #[test]
    fn refuses_future_schema_without_downgrading() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("researchclip.sqlite3");
        {
            let connection = Connection::open(&path).unwrap();
            connection.pragma_update(None, "user_version", 99).unwrap();
        }
        assert!(matches!(
            Storage::open(temporary.path().to_path_buf()),
            Err(StorageError::NewerSchema(99))
        ));
        let connection = Connection::open(path).unwrap();
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, 99);
    }

    #[test]
    fn refuses_corrupt_database_without_replacing_it() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("researchclip.sqlite3");
        fs::write(&path, b"not a SQLite database").unwrap();
        assert!(matches!(
            Storage::open(temporary.path().to_path_buf()),
            Err(StorageError::Database(_))
        ));
        assert_eq!(fs::read(path).unwrap(), b"not a SQLite database");
    }

    #[test]
    fn failed_migration_rolls_back_schema_and_version() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("CREATE TABLE app_metadata (unexpected TEXT);")
            .unwrap();
        assert!(migrations::migrate(&mut connection, None).is_err());
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, 0);
        let table_count: u32 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 1);
    }

    #[test]
    fn rejects_a_file_as_data_directory() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("blocked");
        fs::write(&path, b"keep").unwrap();
        assert!(matches!(
            Storage::open(path.clone()),
            Err(StorageError::Io(_))
        ));
        assert_eq!(fs::read(path).unwrap(), b"keep");
    }

    #[test]
    fn ipc_diagnostics_use_camel_case() {
        let temporary = tempfile::tempdir().unwrap();
        let storage = Storage::open(temporary.path().to_path_buf()).unwrap();
        let json = serde_json::to_value(storage.status().unwrap()).unwrap();
        assert_eq!(json["appName"], "ResearchClip");
        assert_eq!(json["schemaVersion"], migrations::SCHEMA_VERSION);
        assert!(json.get("data_dir").is_none());
    }
    #[test]
    fn a_second_writer_is_refused_and_os_lock_is_released_on_close() {
        let temporary = tempfile::tempdir().unwrap();
        let one = Storage::open(temporary.path().into()).unwrap();
        assert!(matches!(
            Storage::open(temporary.path().into()),
            Err(StorageError::Invalid(_))
        ));
        drop(one);
        assert!(Storage::open(temporary.path().into()).is_ok());
    }
}
