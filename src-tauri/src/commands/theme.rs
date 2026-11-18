//! Theme commands for the Settings page.

use tauri::State;

use crate::db::Db;
use crate::theme::{self, AccentPreset, Mode, Theme};

#[tauri::command]
pub fn get_theme(db: State<'_, Db>) -> Result<Theme, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    theme::load(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_theme(db: State<'_, Db>, mode: Mode, accent: String) -> Result<Theme, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    theme::save(&conn, mode, &accent).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_accent_presets() -> Result<Vec<AccentPreset>, String> {
    Ok(theme::PRESETS.to_vec())
}
