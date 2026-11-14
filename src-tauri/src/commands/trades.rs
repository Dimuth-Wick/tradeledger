//! Trade commands. Each one locks the shared connection, calls the core
//! (`trade_entry` / `db::trades`) and turns errors into plain text for the UI.

use std::fmt::Display;

use rusqlite::Connection;
use tauri::State;

use crate::db::{self, Db};
use crate::models::{Trade, WithId};
use crate::trade_entry::{self, TradeInput};

/// Runs `f` with the locked connection and converts any error to a String.
fn with_conn<T, E: Display>(
    db: &Db,
    f: impl FnOnce(&Connection) -> Result<T, E>,
) -> Result<T, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    f(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_trade(db: State<'_, Db>, input: TradeInput) -> Result<WithId<Trade>, String> {
    with_conn(&db, |c| trade_entry::create(c, &input))
}

#[tauri::command]
pub fn update_trade(
    db: State<'_, Db>,
    id: i64,
    input: TradeInput,
) -> Result<WithId<Trade>, String> {
    with_conn(&db, |c| trade_entry::update(c, id, &input))
}

#[tauri::command]
pub fn delete_trade(db: State<'_, Db>, id: i64) -> Result<(), String> {
    with_conn(&db, |c| trade_entry::delete(c, id))
}

#[tauri::command]
pub fn get_trade(db: State<'_, Db>, id: i64) -> Result<Option<WithId<Trade>>, String> {
    with_conn(&db, |c| db::trades::get(c, id))
}

#[tauri::command]
pub fn list_trades(db: State<'_, Db>) -> Result<Vec<WithId<Trade>>, String> {
    with_conn(&db, db::trades::list)
}
