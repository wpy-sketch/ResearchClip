use super::StorageError;
use rusqlite::Connection;

pub const SCHEMA_VERSION: u32 = 1;

fn apply_migration(
    connection: &mut Connection,
    version: u32,
    sql: &str,
) -> Result<(), StorageError> {
    let transaction = connection.transaction()?;
    transaction.execute_batch(sql)?;
    transaction.pragma_update(None, "user_version", version)?;
    transaction.commit()?;
    Ok(())
}

pub fn migrate(connection: &mut Connection) -> Result<(), StorageError> {
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(StorageError::NewerSchema(version));
    }
    if version == 0 {
        apply_migration(
            connection,
            1,
            include_str!("../../migrations/001_workspace.sql"),
        )?;
    }
    // Verify the known schema before reporting storage as ready.
    let _: String = connection.query_row(
        "SELECT value FROM app_metadata WHERE key = 'workspace_initialized_at'",
        [],
        |row| row.get(0),
    )?;
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
}
