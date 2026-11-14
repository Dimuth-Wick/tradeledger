//! Import commands used by the Import page.

use std::time::{SystemTime, UNIX_EPOCH};

use tauri::State;

use crate::db::Db;
use crate::import_flow::{self, FlowError, ImportPreview, ImportResult};
use crate::importers::ImportError;

/// Plain text for the UI; "unknown format" names the supported file types.
fn message(e: FlowError) -> String {
    match e {
        FlowError::Import(ImportError::UnknownFormat) => import_flow::unknown_format_message(),
        other => other.to_string(),
    }
}

#[tauri::command]
pub fn preview_import(
    db: State<'_, Db>,
    file_name: String,
    content: String,
) -> Result<ImportPreview, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    import_flow::preview(&conn, &file_name, &content).map_err(message)
}

#[tauri::command]
pub fn confirm_import(
    db: State<'_, Db>,
    file_name: String,
    content: String,
) -> Result<ImportResult, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|e| e.to_string())?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    import_flow::confirm(&conn, &file_name, &content, now).map_err(message)
}
