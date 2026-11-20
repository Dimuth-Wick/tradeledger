//! Trade commands. Each one locks the shared connection, calls the core
//! (`trade_entry` / `db::trades`) and turns errors into plain text for the UI.

use std::fmt::Display;

use rusqlite::Connection;
use tauri::State;

use crate::database::{self as db, Db};
use crate::features::journal::{self, JournalInput};
use crate::features::stats::{self, Stats};
use crate::features::symbols::{self, SymbolChoice};
use crate::features::trade_entry::{self, CloseInput, PlanInput, PlanPreview, TradeInput};
use crate::features::trade_query::{self, TradePage, TradeQuery};
use crate::models::{Market, Trade, WithId};

/// Runs `f` with the locked connection and converts any error to a String.
fn with_conn<T, E: Display>(
    db: &Db,
    f: impl FnOnce(&Connection) -> Result<T, E>,
) -> Result<T, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    f(&conn).map_err(|e| e.to_string())
}

/// Computes status, PnL, R:R and R-multiple for the form's live preview
/// without saving. Same code path as `create_trade`, so the preview always
/// matches what will be stored.
#[tauri::command]
pub fn preview_trade(input: TradeInput) -> Result<Trade, String> {
    trade_entry::build_trade(&input, true).map_err(|e| e.to_string())
}

/// R:R and the profit/loss at the target and stop, for the risk-plan boxes.
#[tauri::command]
pub fn preview_plan(input: PlanInput) -> Result<PlanPreview, String> {
    trade_entry::plan_preview(&input).map_err(|e| e.to_string())
}

/// The new-trade symbol menu for one market.
#[tauri::command]
pub fn symbol_choices(db: State<'_, Db>, market: Market) -> Result<Vec<SymbolChoice>, String> {
    with_conn(&db, |c| symbols::choices_from_db(c, market))
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

/// The trade with this exit applied, without saving (live preview).
#[tauri::command]
pub fn preview_close(db: State<'_, Db>, id: i64, input: CloseInput) -> Result<Trade, String> {
    with_conn(&db, |c| trade_entry::preview_close(c, id, &input))
}

/// Saves an exit price on a manual trade (empty = reopen it).
#[tauri::command]
pub fn close_trade(db: State<'_, Db>, id: i64, input: CloseInput) -> Result<WithId<Trade>, String> {
    with_conn(&db, |c| trade_entry::close(c, id, &input))
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

/// Trade detail: R values for the edited plan, without saving.
#[tauri::command]
pub fn preview_journal(db: State<'_, Db>, id: i64, input: JournalInput) -> Result<Trade, String> {
    with_conn(&db, |c| journal::preview(c, id, &input))
}

/// Trade detail: save plan, leverage and notes.
#[tauri::command]
pub fn update_journal(
    db: State<'_, Db>,
    id: i64,
    input: JournalInput,
) -> Result<WithId<Trade>, String> {
    with_conn(&db, |c| journal::update(c, id, &input))
}

/// Trade log: filtered, sorted, paginated, with totals.
#[tauri::command]
pub fn query_trades(db: State<'_, Db>, query: TradeQuery) -> Result<TradePage, String> {
    with_conn(&db, |c| trade_query::query(c, &query))
}

/// Dashboard stats for the same filters as the trade log.
#[tauri::command]
pub fn get_stats(db: State<'_, Db>, query: TradeQuery) -> Result<Stats, String> {
    with_conn(&db, |c| stats::query(c, &query))
}

#[tauri::command]
pub fn list_symbols(db: State<'_, Db>) -> Result<Vec<String>, String> {
    with_conn(&db, db::trades::symbols)
}
