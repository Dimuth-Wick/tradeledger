//! Custom field definitions and their values per trade. Plain SQL only;
//! validation lives in `crate::custom_fields`.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use super::DbError;
use crate::models::FieldType;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomField {
    pub id: i64,
    pub name: String,
    pub field_type: FieldType,
    /// Choices of a `select` field (empty for other types).
    pub options: Vec<String>,
    pub position: i64,
    /// How many trades have a value for this field.
    pub used_by: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldValue {
    pub field_id: i64,
    pub value: String,
}

fn from_row(r: &Row<'_>) -> rusqlite::Result<CustomField> {
    let options_json: String = r.get("options")?;
    Ok(CustomField {
        id: r.get("id")?,
        name: r.get("name")?,
        field_type: r.get("field_type")?,
        // Written by this app as a JSON array; a broken value shows no options
        // rather than making every field unreadable.
        options: serde_json::from_str(&options_json).unwrap_or_default(),
        position: r.get("position")?,
        used_by: r.get("used_by")?,
    })
}

const SELECT: &str =
    "SELECT f.*, (SELECT COUNT(*) FROM custom_values v WHERE v.field_id = f.id) AS used_by
                      FROM custom_fields f";

pub fn list(conn: &Connection) -> Result<Vec<CustomField>, DbError> {
    let mut stmt = conn.prepare(&format!("{SELECT} ORDER BY f.position, f.id"))?;
    let rows = stmt.query_map([], from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<CustomField>, DbError> {
    Ok(conn
        .query_row(&format!("{SELECT} WHERE f.id = ?1"), [id], from_row)
        .optional()?)
}

pub fn create(
    conn: &Connection,
    name: &str,
    field_type: FieldType,
    options: &[String],
) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO custom_fields (name, field_type, options, position)
         VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(position) + 1, 0) FROM custom_fields))",
        params![name, field_type, serde_json::to_string(options)?],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, name: &str, options: &[String]) -> Result<bool, DbError> {
    Ok(conn.execute(
        "UPDATE custom_fields SET name = ?1, options = ?2 WHERE id = ?3",
        params![name, serde_json::to_string(options)?, id],
    )? > 0)
}

/// Also removes every trade's value for it (ON DELETE CASCADE).
pub fn delete(conn: &Connection, id: i64) -> Result<bool, DbError> {
    Ok(conn.execute("DELETE FROM custom_fields WHERE id = ?1", [id])? > 0)
}

pub fn name_taken(conn: &Connection, name: &str, except: Option<i64>) -> Result<bool, DbError> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM custom_fields WHERE name = ?1 COLLATE NOCASE AND id IS NOT ?2)",
        params![name, except],
        |r| r.get(0),
    )?)
}

/// How many trades use `value` for this field (e.g. a select option).
pub fn count_value(conn: &Connection, field_id: i64, value: &str) -> Result<u32, DbError> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM custom_values WHERE field_id = ?1 AND value = ?2",
        params![field_id, value],
        |r| r.get(0),
    )?)
}

pub fn values_for_trade(conn: &Connection, trade_id: i64) -> Result<Vec<FieldValue>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT field_id, value FROM custom_values WHERE trade_id = ?1 ORDER BY field_id",
    )?;
    let rows = stmt.query_map([trade_id], |r| {
        Ok(FieldValue {
            field_id: r.get(0)?,
            value: r.get(1)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn set_value(
    conn: &Connection,
    trade_id: i64,
    field_id: i64,
    value: &str,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO custom_values (trade_id, field_id, value) VALUES (?1, ?2, ?3)
         ON CONFLICT (trade_id, field_id) DO UPDATE SET value = excluded.value",
        params![trade_id, field_id, value],
    )?;
    Ok(())
}

pub fn clear_value(conn: &Connection, trade_id: i64, field_id: i64) -> Result<(), DbError> {
    conn.execute(
        "DELETE FROM custom_values WHERE trade_id = ?1 AND field_id = ?2",
        params![trade_id, field_id],
    )?;
    Ok(())
}
