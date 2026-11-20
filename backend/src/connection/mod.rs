//! The connection: the only functions the frontend can call, with
//! `invoke("name", …)` from `frontend/connection/backend.ts`.
//!
//! These (Tauri "commands") stay thin: they check input, call a module in
//! `crate::features`, and turn errors into plain text so the UI shows a
//! readable message. One file per area, same names as the frontend uses.

pub mod calendar;
pub mod custom_fields;
pub mod dashboard;
pub mod data;
pub mod grouping;
pub mod import;
pub mod milestones;
pub mod playbooks;
pub mod reports;
pub mod screenshots;
pub mod tags;
pub mod theme;
pub mod trades;

use serde::Serialize;
use tauri::State;

use crate::database::{self as db, Db};

/// Health check used by the UI to prove the Rust core is reachable.
///
/// Returns `Result` even though it can't fail, so every command has the same
/// shape on the TypeScript side (a promise that resolves or rejects).
#[tauri::command]
pub fn ping() -> Result<String, String> {
    Ok(format!(
        "pong from TradeLedger core v{}",
        env!("CARGO_PKG_VERSION")
    ))
}

/// What the UI needs to show about the database.
/// camelCase so the JSON matches TypeScript naming (`schemaVersion`).
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DbInfo {
    pub path: String,
    pub schema_version: u32,
}

#[tauri::command]
pub fn db_info(db: State<'_, Db>) -> Result<DbInfo, String> {
    // A poisoned mutex means an earlier command panicked mid-query; report it
    // instead of crashing the app.
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let schema_version = db::schema_version(&conn).map_err(|e| e.to_string())?;
    Ok(DbInfo {
        path: db.path.display().to_string(),
        schema_version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_answers_with_pong_and_version() {
        let reply = ping().expect("ping never fails");
        assert!(reply.starts_with("pong"));
        assert!(reply.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn db_info_serializes_as_camel_case() {
        let info = DbInfo {
            path: "C:/x/tradeledger.db".into(),
            schema_version: 1,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "path": "C:/x/tradeledger.db", "schemaVersion": 1 })
        );
    }
}
