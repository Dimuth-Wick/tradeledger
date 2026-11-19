//! Backup and restore: the whole journal (database + screenshots) in one
//! .zip the user keeps wherever they like.
//!
//! Zip layout (nothing else is accepted on restore):
//!
//! ```text
//! manifest.json        what's inside (app, format, schema, counts, date)
//! tradeledger.db       a consistent copy made with VACUUM INTO
//! screenshots/t…png    image files, only names this app generates
//! ```
//!
//! Restore never trusts the zip: every entry name is checked before anything
//! is written, the database is opened and integrity-checked on its own, and
//! the current data is saved to a safety backup first.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;
use rusqlite::backup::Backup;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::database::{self as db, DbError};
use crate::features::screenshots;
use crate::models::TimestampMs;

const APP: &str = "TradeLedger";
/// Bump if the zip layout changes.
const FORMAT: u32 = 1;
const MANIFEST: &str = "manifest.json";
const DB_ENTRY: &str = "tradeledger.db";
const SHOTS_PREFIX: &str = "screenshots/";
/// Refuse absurd entries (a zip bomb, a broken file).
const MAX_ENTRY_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum BackupError {
    #[error("this file is not a TradeLedger backup")]
    NotABackup,
    #[error("this backup was made by a newer TradeLedger (data version {0}); update the app first")]
    TooNew(u32),
    #[error("the backup's database is damaged ({0})")]
    Damaged(String),
    #[error("file error: {0}")]
    Io(#[from] io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error(transparent)]
    Db(#[from] DbError),
}

impl From<rusqlite::Error> for BackupError {
    fn from(e: rusqlite::Error) -> Self {
        BackupError::Db(e.into())
    }
}

impl From<serde_json::Error> for BackupError {
    fn from(_: serde_json::Error) -> Self {
        BackupError::NotABackup
    }
}

/// What a backup holds; shown before restoring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub app: String,
    pub format: u32,
    pub schema_version: u32,
    pub created_at: TimestampMs,
    pub trades: u32,
    pub screenshots: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    /// What was restored.
    pub restored: Manifest,
    /// Where the data from before the restore was saved.
    pub safety_backup: String,
}

/// A path next to `p` with a suffix, for temp and staging files.
fn sibling(p: &Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// Removes a SQLite file and its -wal / -shm companions, ignoring absence.
fn remove_db_files(p: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = fs::remove_file(sibling(p, suffix));
    }
}

fn screenshot_files(dir: &Path) -> Result<Vec<(String, PathBuf)>, BackupError> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e.into()),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type()?.is_file() && screenshots::is_generated_name(&name) {
            out.push((name, entry.path()));
        }
    }
    out.sort();
    Ok(out)
}

/// Writes a backup of the open database and its screenshots to `dest`.
/// The zip is written to a temporary name and renamed at the end, so a
/// failed backup never leaves a half-written file under the chosen name.
pub fn create(
    conn: &Connection,
    shots_dir: &Path,
    dest: &Path,
    now: TimestampMs,
) -> Result<Manifest, BackupError> {
    let copy = std::env::temp_dir().join(format!(
        "tradeledger-backup-{}-{now}.db",
        std::process::id()
    ));
    remove_db_files(&copy);
    let result = (|| {
        // A consistent snapshot even while the app keeps running.
        conn.execute("VACUUM INTO ?1", [copy.to_string_lossy().into_owned()])?;
        let shots = screenshot_files(shots_dir)?;
        let manifest = Manifest {
            app: APP.into(),
            format: FORMAT,
            schema_version: db::schema_version(conn)?,
            created_at: now,
            trades: conn.query_row("SELECT COUNT(*) FROM trades", [], |r| r.get(0))?,
            screenshots: shots.len() as u32,
        };

        let part = sibling(dest, ".part");
        let mut zip = ZipWriter::new(File::create(&part)?);
        let packed = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .large_file(true);
        // Images are already compressed; storing them is faster and as small.
        let stored = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .large_file(true);

        zip.start_file(MANIFEST, packed)?;
        zip.write_all(&serde_json::to_vec_pretty(&manifest).expect("manifest serializes"))?;
        zip.start_file(DB_ENTRY, packed)?;
        io::copy(&mut File::open(&copy)?, &mut zip)?;
        for (name, path) in &shots {
            zip.start_file(format!("{SHOTS_PREFIX}{name}"), stored)?;
            io::copy(&mut File::open(path)?, &mut zip)?;
        }
        zip.finish()?;
        fs::rename(&part, dest)?;
        Ok(manifest)
    })();
    remove_db_files(&copy);
    if result.is_err() {
        let _ = fs::remove_file(sibling(dest, ".part"));
    }
    result
}

/// What an entry is, or None if it has no business in a backup.
enum Entry {
    Manifest,
    Database,
    Screenshot(String),
}

fn classify(name: &str) -> Option<Entry> {
    match name {
        MANIFEST => Some(Entry::Manifest),
        DB_ENTRY => Some(Entry::Database),
        _ => {
            let file = name.strip_prefix(SHOTS_PREFIX)?;
            // `is_generated_name` allows only letters, digits, '-' and one
            // extension, so "..", slashes and drive letters can't get in.
            screenshots::is_generated_name(file).then(|| Entry::Screenshot(file.to_string()))
        }
    }
}

/// Opens a backup and checks it before anything is written: the manifest
/// is ours and not from a newer version, every entry is expected, sizes are
/// sane, and the database is there.
fn open_checked(src: &Path) -> Result<(ZipArchive<File>, Manifest), BackupError> {
    let mut zip = ZipArchive::new(File::open(src)?).map_err(|_| BackupError::NotABackup)?;
    let mut has_db = false;
    for i in 0..zip.len() {
        let entry = zip.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry
            .name()
            .map_err(|_| BackupError::NotABackup)?
            .into_owned();
        match classify(&name) {
            None => return Err(BackupError::NotABackup),
            Some(Entry::Database) => has_db = true,
            Some(_) => {}
        }
        if entry.size() > MAX_ENTRY_BYTES {
            return Err(BackupError::NotABackup);
        }
    }
    let manifest: Manifest = {
        let mut text = String::new();
        zip.by_name(MANIFEST)
            .map_err(|_| BackupError::NotABackup)?
            .take(1024 * 1024)
            .read_to_string(&mut text)?;
        serde_json::from_str(&text)?
    };
    if manifest.app != APP || manifest.format != FORMAT || !has_db {
        return Err(BackupError::NotABackup);
    }
    let current = db::migrations_latest();
    if manifest.schema_version > current {
        return Err(BackupError::TooNew(manifest.schema_version));
    }
    Ok((zip, manifest))
}

/// Reads what a backup contains, without changing anything.
pub fn inspect(src: &Path) -> Result<Manifest, BackupError> {
    open_checked(src).map(|(_, m)| m)
}

/// Replaces the open database and the screenshots with the backup's.
///
/// Steps, so a bad file never costs data: check the zip; unpack the database
/// to a temp file, open it (applying any newer migrations) and run SQLite's
/// integrity check; unpack the screenshots to a staging folder; save the
/// current data to `safety_dir`; only then copy the database in (SQLite's
/// online backup, so the open connection stays valid) and swap the folders.
pub fn restore(
    conn: &mut Connection,
    shots_dir: &Path,
    src: &Path,
    safety_dir: &Path,
    now: TimestampMs,
) -> Result<RestoreResult, BackupError> {
    let (mut zip, manifest) = open_checked(src)?;

    let temp_db = std::env::temp_dir().join(format!(
        "tradeledger-restore-{}-{now}.db",
        std::process::id()
    ));
    let staging = sibling(shots_dir, ".restoring");
    remove_db_files(&temp_db);
    let _ = fs::remove_dir_all(&staging);

    let result = (|| {
        // 1. Unpack everything to temporary places.
        fs::create_dir_all(&staging)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            if entry.is_dir() {
                continue;
            }
            let name = entry
                .name()
                .map_err(|_| BackupError::NotABackup)?
                .into_owned();
            let target = match classify(&name) {
                Some(Entry::Database) => temp_db.clone(),
                Some(Entry::Screenshot(file)) => staging.join(file),
                Some(Entry::Manifest) => continue,
                None => return Err(BackupError::NotABackup),
            };
            let mut out = File::create(&target)?;
            io::copy(&mut (&mut entry).take(MAX_ENTRY_BYTES), &mut out)?;
        }

        // 2. The database must open, upgrade and pass SQLite's own check.
        {
            let restored = db::open(&temp_db).map_err(|e| BackupError::Damaged(e.to_string()))?;
            let check: String = restored.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
            if check != "ok" {
                return Err(BackupError::Damaged(check));
            }
        }

        // 3. Keep the current data, just in case.
        fs::create_dir_all(safety_dir)?;
        let safety = safety_dir.join(format!("before-restore-{now}.zip"));
        create(conn, shots_dir, &safety, now)?;

        // 4. Swap in: database pages first, then the screenshot folder.
        {
            let restored = Connection::open(&temp_db)?;
            let backup = Backup::new(&restored, conn)?;
            backup.run_to_completion(1024, Duration::ZERO, None)?;
        }
        let old = sibling(shots_dir, ".old");
        let _ = fs::remove_dir_all(&old);
        if shots_dir.exists() {
            fs::rename(shots_dir, &old)?;
        }
        if let Err(e) = fs::rename(&staging, shots_dir) {
            // Put the previous images back rather than leave none.
            let _ = fs::rename(&old, shots_dir);
            return Err(e.into());
        }
        let _ = fs::remove_dir_all(&old);

        Ok(RestoreResult {
            restored: manifest.clone(),
            safety_backup: safety.display().to_string(),
        })
    })();

    remove_db_files(&temp_db);
    let _ = fs::remove_dir_all(&staging);
    result
}

#[cfg(test)]
mod tests;
