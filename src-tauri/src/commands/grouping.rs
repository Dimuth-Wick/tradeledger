//! Manual grouping commands (merge / split / reset) and a trade's fills.

use std::fmt::Display;

use rusqlite::Connection;
use tauri::State;

use crate::db::Db;
use crate::grouping::edit::{self, TradeFill};
use crate::grouping::persist::RegroupSummary;

fn with_conn<T, E: Display>(
    db: &Db,
    f: impl FnOnce(&Connection) -> Result<T, E>,
) -> Result<T, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    f(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_trade_fills(db: State<'_, Db>, trade_id: i64) -> Result<Vec<TradeFill>, String> {
    with_conn(&db, |c| edit::trade_fills(c, trade_id))
}

#[tauri::command]
pub fn merge_trade_with_previous(db: State<'_, Db>, id: i64) -> Result<RegroupSummary, String> {
    with_conn(&db, |c| edit::merge_with_previous(c, id))
}

#[tauri::command]
pub fn split_trade_at_fill(
    db: State<'_, Db>,
    trade_id: i64,
    fill_id: i64,
) -> Result<RegroupSummary, String> {
    with_conn(&db, |c| edit::split_at_fill(c, trade_id, fill_id))
}

#[tauri::command]
pub fn reset_grouping(db: State<'_, Db>, trade_id: i64) -> Result<RegroupSummary, String> {
    with_conn(&db, |c| edit::reset_grouping(c, trade_id))
}
