//! Custom field commands for the Fields page and trade detail.

use std::fmt::Display;

use rusqlite::Connection;
use tauri::State;

use crate::custom_fields::{self, ValueInput};
use crate::db::Db;
use crate::db::custom_fields::{CustomField, FieldValue};
use crate::models::FieldType;

fn with_conn<T, E: Display>(
    db: &Db,
    f: impl FnOnce(&Connection) -> Result<T, E>,
) -> Result<T, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    f(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_custom_fields(db: State<'_, Db>) -> Result<Vec<CustomField>, String> {
    with_conn(&db, custom_fields::list)
}

#[tauri::command]
pub fn create_custom_field(
    db: State<'_, Db>,
    name: String,
    field_type: FieldType,
    options: Vec<String>,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        custom_fields::create(c, &name, field_type, &options)
    })
}

#[tauri::command]
pub fn update_custom_field(
    db: State<'_, Db>,
    id: i64,
    name: String,
    options: Vec<String>,
) -> Result<(), String> {
    with_conn(&db, |c| custom_fields::update(c, id, &name, &options))
}

#[tauri::command]
pub fn delete_custom_field(db: State<'_, Db>, id: i64) -> Result<(), String> {
    with_conn(&db, |c| custom_fields::delete(c, id))
}

#[tauri::command]
pub fn get_custom_values(db: State<'_, Db>, trade_id: i64) -> Result<Vec<FieldValue>, String> {
    with_conn(&db, |c| custom_fields::values(c, trade_id))
}

#[tauri::command]
pub fn set_custom_values(
    db: State<'_, Db>,
    trade_id: i64,
    values: Vec<ValueInput>,
) -> Result<Vec<FieldValue>, String> {
    with_conn(&db, |c| custom_fields::set_values(c, trade_id, &values))
}
