//! Backup, restore and CSV export commands. Paths come from the system
//! Save/Open dialogs the user just used.

use std::path::Path;

use tauri::State;

use crate::backup::{self, Manifest, RestoreResult};
use crate::calendar::now_ms;
use crate::db::Db;
use crate::export;
use crate::screenshots;
use crate::trade_query::TradeQuery;

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
    let safety_dir = db
        .path
        .parent()
        .map_or_else(|| Path::new("backups").to_path_buf(), |p| p.join("backups"));
    backup::restore(
        &mut conn,
        &screenshots::dir_for(&db.path),
        Path::new(&path),
        &safety_dir,
        now_ms(),
    )
    .map_err(|e| e.to_string())
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
