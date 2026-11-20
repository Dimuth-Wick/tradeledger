//! Milestone badge commands (50, 100, 300 … trades logged).

use tauri::State;

use crate::database::Db;
use crate::features::milestones::{self, Progress};

/// Badges, progress to the next one, and any not celebrated yet.
#[tauri::command]
pub fn get_milestones(db: State<'_, Db>) -> Result<Progress, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    milestones::load(&conn).map_err(|e| e.to_string())
}

/// Called after the celebration pop-up is closed.
#[tauri::command]
pub fn mark_milestones_seen(db: State<'_, Db>) -> Result<Progress, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    milestones::mark_seen(&conn).map_err(|e| e.to_string())
}
