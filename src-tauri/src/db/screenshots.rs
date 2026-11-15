//! Screenshot rows. The image files themselves are handled by
//! `crate::screenshots`; the DB only stores the generated file name.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use super::DbError;
use crate::models::TimestampMs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub id: i64,
    pub trade_id: i64,
    pub file_name: String,
    pub caption: String,
    pub created_at: TimestampMs,
}

fn from_row(r: &Row<'_>) -> rusqlite::Result<Screenshot> {
    Ok(Screenshot {
        id: r.get("id")?,
        trade_id: r.get("trade_id")?,
        file_name: r.get("file_name")?,
        caption: r.get("caption")?,
        created_at: r.get("created_at")?,
    })
}

pub fn insert(
    conn: &Connection,
    trade_id: i64,
    file_name: &str,
    caption: &str,
    created_at: TimestampMs,
) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO screenshots (trade_id, file_name, caption, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![trade_id, file_name, caption, created_at],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Screenshot>, DbError> {
    Ok(conn
        .query_row("SELECT * FROM screenshots WHERE id = ?1", [id], from_row)
        .optional()?)
}

/// Oldest first, so the order matches when they were added.
pub fn list_for_trade(conn: &Connection, trade_id: i64) -> Result<Vec<Screenshot>, DbError> {
    let mut stmt =
        conn.prepare("SELECT * FROM screenshots WHERE trade_id = ?1 ORDER BY created_at, id")?;
    let rows = stmt.query_map([trade_id], from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn set_caption(conn: &Connection, id: i64, caption: &str) -> Result<bool, DbError> {
    Ok(conn.execute(
        "UPDATE screenshots SET caption = ?1 WHERE id = ?2",
        params![caption, id],
    )? > 0)
}

pub fn delete(conn: &Connection, id: i64) -> Result<bool, DbError> {
    Ok(conn.execute("DELETE FROM screenshots WHERE id = ?1", [id])? > 0)
}

/// Every file name still referenced (to find leftover files).
pub fn all_file_names(conn: &Connection) -> Result<Vec<String>, DbError> {
    let mut stmt = conn.prepare("SELECT file_name FROM screenshots")?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}
