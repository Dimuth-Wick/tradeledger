//! Database: where the file lives, how it is opened, and how its schema is
//! upgraded.
//!
//! The DB file is stored in the OS app-data folder (`%APPDATA%\TradeLedger`
//! on Windows), never inside the repo, so real trades can't be committed.

mod migrations;
mod sql;

// Repositories: one module per table group. Each takes a `&Connection`, so
// tests can pass an in-memory DB and commands can pass the shared one.
pub mod allocations;
pub mod fills;
pub mod funding;
pub mod import_batches;
pub mod settings;
pub mod trades;

#[cfg(test)]
pub(crate) mod test_data;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;
use thiserror::Error;

/// File name inside the app-data folder.
pub const DB_FILE_NAME: &str = "tradeledger.db";

/// Folder name under the OS data dir. Fixed name (not the bundle identifier)
/// so users can find their data easily.
pub const APP_DIR_NAME: &str = "TradeLedger";

#[derive(Debug, Error)]
pub enum DbError {
    #[error("could not create data folder {path}: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration failed: {0}")]
    Migration(#[from] rusqlite_migration::Error),
    #[error("invalid JSON in settings: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result of inserting a row that has a `dedupe_hash`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertOutcome {
    /// New row, with its id.
    Inserted(i64),
    /// Same hash already stored; nothing written.
    Duplicate,
}

/// Shared connection held in Tauri state. A `Mutex` because SQLite
/// connections must not be used from two threads at once, and Tauri runs
/// commands on a thread pool.
pub struct Db {
    pub conn: Mutex<Connection>,
    pub path: PathBuf,
}

/// Full path of the DB file for a given OS data dir
/// (e.g. `C:\Users\me\AppData\Roaming` → `...\TradeLedger\tradeledger.db`).
pub fn db_path(os_data_dir: &Path) -> PathBuf {
    os_data_dir.join(APP_DIR_NAME).join(DB_FILE_NAME)
}

/// Opens (or creates) the DB file, creating its folder if needed, and brings
/// the schema up to date.
pub fn open(path: &Path) -> Result<Connection, DbError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|source| DbError::CreateDir {
            path: dir.to_path_buf(),
            source,
        })?;
    }
    let mut conn = Connection::open(path)?;
    prepare(&mut conn)?;
    Ok(conn)
}

/// In-memory DB with the same setup as the real one. Used by tests.
#[cfg(test)]
pub fn open_in_memory() -> Result<Connection, DbError> {
    let mut conn = Connection::open_in_memory()?;
    prepare(&mut conn)?;
    Ok(conn)
}

/// Connection settings + migrations, shared by file and in-memory DBs so
/// tests exercise exactly what the app runs.
fn prepare(conn: &mut Connection) -> Result<(), DbError> {
    // SQLite ignores FOREIGN KEY constraints unless this is on, per connection.
    conn.pragma_update(None, "foreign_keys", "ON")?;
    // WAL: readers don't block the writer; safer against crashes mid-write.
    // (In-memory DBs silently stay in "memory" mode, which is fine.)
    conn.pragma_update(None, "journal_mode", "WAL")?;
    migrations::migrations().to_latest(conn)?;
    Ok(())
}

/// Current schema version (number of migrations applied).
pub fn schema_version(conn: &Connection) -> Result<u32, DbError> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique temp file path so parallel tests don't collide.
    fn temp_db_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "tradeledger-test-{name}-{}-{nanos}",
                std::process::id()
            ))
            .join(DB_FILE_NAME)
    }

    #[test]
    fn db_path_is_inside_tradeledger_folder() {
        let p = db_path(Path::new("C:/data"));
        assert_eq!(p, Path::new("C:/data/TradeLedger/tradeledger.db"));
    }

    #[test]
    fn in_memory_db_is_migrated_to_latest() {
        let conn = open_in_memory().unwrap();
        assert_eq!(schema_version(&conn).unwrap(), migrations::LATEST_VERSION);
    }

    #[test]
    fn foreign_keys_are_enabled() {
        let conn = open_in_memory().unwrap();
        let on: i64 = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .unwrap();
        assert_eq!(on, 1);
    }

    #[test]
    fn open_creates_folder_and_file_then_reopens_without_rerunning() {
        let path = temp_db_path("reopen");
        assert!(!path.exists());

        {
            let conn = open(&path).unwrap();
            conn.execute(
                "INSERT INTO settings (key, value) VALUES ('theme', '\"dark\"')",
                [],
            )
            .unwrap();
        }
        assert!(path.exists());

        // Second open must keep data and not re-apply migrations.
        let conn = open(&path).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), migrations::LATEST_VERSION);
        let theme: String = conn
            .query_row("SELECT value FROM settings WHERE key = 'theme'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(theme, "\"dark\"");

        drop(conn);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn file_db_uses_wal_journal() {
        let path = temp_db_path("wal");
        let conn = open(&path).unwrap();
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");

        drop(conn);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
