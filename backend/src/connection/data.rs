//! Backup, restore and CSV export commands. Paths come from the system
//! Save/Open dialogs the user just used.

use std::path::{Path, PathBuf};

use tauri::State;

use crate::database::Db;
use crate::features::backup::{self, Manifest, RestoreResult};
use crate::features::calendar::now_ms;
use crate::features::export;
use crate::features::screenshots;
use crate::features::trade_query::TradeQuery;
use crate::features::updates;

/// Writes a backup zip of everything to `path`.
#[tauri::command]
pub fn backup_now(db: State<'_, Db>, path: String) -> Result<Manifest, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    backup::create(
        &conn,
        &screenshots::dir_for(&db.path),
        Path::new(&path),
        now_ms(),
    )
    .map_err(|e| e.to_string())
}

/// What a backup contains, checked but not restored.
#[tauri::command]
pub fn inspect_backup(path: String) -> Result<Manifest, String> {
    backup::inspect(Path::new(&path)).map_err(|e| e.to_string())
}

/// Replaces all data with the backup's. The current data is first saved to
/// `backups/before-restore-<time>.zip` next to the database.
#[tauri::command]
pub fn restore_backup(db: State<'_, Db>, path: String) -> Result<RestoreResult, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    backup::restore(
        &mut conn,
        &screenshots::dir_for(&db.path),
        Path::new(&path),
        &safety_dir(&db.path),
        now_ms(),
    )
    .map_err(|e| e.to_string())
}

/// Saves everything to `backups/before-update-<time>.zip` before a new
/// version installs (only the newest few are kept).
#[tauri::command]
pub fn backup_before_update(db: State<'_, Db>) -> Result<Manifest, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    updates::backup_before_update(
        &conn,
        &screenshots::dir_for(&db.path),
        &safety_dir(&db.path),
        now_ms(),
    )
    .map_err(|e| e.to_string())
}

/// The `backups` folder next to the database.
fn safety_dir(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .map_or_else(|| PathBuf::from("backups"), |p| p.join("backups"))
}

/// Saves the trades matching `query` as CSV; returns how many rows.
#[tauri::command]
pub fn export_trades_csv(
    db: State<'_, Db>,
    query: TradeQuery,
    path: String,
) -> Result<u32, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let (text, rows) = export::trades_csv(&conn, &query).map_err(|e| e.to_string())?;
    // A UTF-8 BOM so Excel shows symbols like "→" and "·" correctly.
    std::fs::write(&path, format!("\u{feff}{text}")).map_err(|e| e.to_string())?;
    Ok(rows)
}
