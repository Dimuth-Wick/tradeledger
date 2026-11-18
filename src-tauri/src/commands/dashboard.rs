//! Dashboard layout commands.

use tauri::State;

use crate::dashboard_layout::{self, LayoutItem};
use crate::db::Db;

#[tauri::command]
pub fn get_dashboard_layout(db: State<'_, Db>) -> Result<Vec<LayoutItem>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    dashboard_layout::load(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_dashboard_layout(
    db: State<'_, Db>,
    items: Vec<LayoutItem>,
) -> Result<Vec<LayoutItem>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    dashboard_layout::save(&conn, &items).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_dashboard_layout(db: State<'_, Db>) -> Result<Vec<LayoutItem>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    dashboard_layout::reset(&conn).map_err(|e| e.to_string())
}
