//! Playbook commands for the Playbooks page and the trade checklist.

use std::fmt::Display;

use rusqlite::Connection;
use tauri::State;

use crate::database::Db;
use crate::database::playbooks::Playbook;
use crate::features::playbooks::{self, PlaybookStats, RuleInput, TradePlaybook};

fn with_conn<T, E: Display>(
    db: &Db,
    f: impl FnOnce(&Connection) -> Result<T, E>,
) -> Result<T, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    f(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_playbooks(db: State<'_, Db>) -> Result<Vec<Playbook>, String> {
    with_conn(&db, playbooks::list)
}

#[tauri::command]
pub fn create_playbook(
    db: State<'_, Db>,
    name: String,
    description: String,
    rules: Vec<String>,
) -> Result<i64, String> {
    with_conn(&db, |c| playbooks::create(c, &name, &description, &rules))
}

#[tauri::command]
pub fn update_playbook(
    db: State<'_, Db>,
    id: i64,
    name: String,
    description: String,
    rules: Vec<RuleInput>,
) -> Result<(), String> {
    with_conn(&db, |c| {
        playbooks::update(c, id, &name, &description, &rules)
    })
}

#[tauri::command]
pub fn delete_playbook(db: State<'_, Db>, id: i64) -> Result<(), String> {
    with_conn(&db, |c| playbooks::delete(c, id))
}

#[tauri::command]
pub fn get_trade_playbook(db: State<'_, Db>, trade_id: i64) -> Result<TradePlaybook, String> {
    with_conn(&db, |c| playbooks::for_trade(c, trade_id))
}

#[tauri::command]
pub fn set_trade_playbook(
    db: State<'_, Db>,
    trade_id: i64,
    playbook_id: Option<i64>,
    checked: Vec<i64>,
) -> Result<TradePlaybook, String> {
    with_conn(&db, |c| {
        playbooks::set_for_trade(c, trade_id, playbook_id, &checked)
    })
}

#[tauri::command]
pub fn playbook_stats(db: State<'_, Db>) -> Result<Vec<PlaybookStats>, String> {
    with_conn(&db, playbooks::stats)
}
