//! Tag commands for Settings → Tags, trade detail and trade log.

use std::fmt::Display;

use rusqlite::Connection;
use tauri::State;

use crate::database::Db;
use crate::database::tags::TagGroup;
use crate::features::tags;

fn with_conn<T, E: Display>(
    db: &Db,
    f: impl FnOnce(&Connection) -> Result<T, E>,
) -> Result<T, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    f(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_tag_groups(db: State<'_, Db>) -> Result<Vec<TagGroup>, String> {
    with_conn(&db, tags::list)
}

#[tauri::command]
pub fn create_tag_group(db: State<'_, Db>, name: String, color: String) -> Result<i64, String> {
    with_conn(&db, |c| tags::create_group(c, &name, &color))
}

#[tauri::command]
pub fn update_tag_group(
    db: State<'_, Db>,
    id: i64,
    name: String,
    color: String,
) -> Result<(), String> {
    with_conn(&db, |c| tags::update_group(c, id, &name, &color))
}

#[tauri::command]
pub fn delete_tag_group(db: State<'_, Db>, id: i64) -> Result<(), String> {
    with_conn(&db, |c| tags::delete_group(c, id))
}

#[tauri::command]
pub fn create_tag(
    db: State<'_, Db>,
    group_id: i64,
    name: String,
    color: String,
) -> Result<i64, String> {
    with_conn(&db, |c| tags::create_tag(c, group_id, &name, &color))
}

#[tauri::command]
pub fn update_tag(db: State<'_, Db>, id: i64, name: String, color: String) -> Result<(), String> {
    with_conn(&db, |c| tags::update_tag(c, id, &name, &color))
}

#[tauri::command]
pub fn delete_tag(db: State<'_, Db>, id: i64) -> Result<(), String> {
    with_conn(&db, |c| tags::delete_tag(c, id))
}

#[tauri::command]
pub fn get_trade_tags(db: State<'_, Db>, trade_id: i64) -> Result<Vec<i64>, String> {
    with_conn(&db, |c| tags::trade_tags(c, trade_id))
}

#[tauri::command]
pub fn set_trade_tags(
    db: State<'_, Db>,
    trade_id: i64,
    tag_ids: Vec<i64>,
) -> Result<Vec<i64>, String> {
    with_conn(&db, |c| tags::set_trade_tags(c, trade_id, &tag_ids))
}
