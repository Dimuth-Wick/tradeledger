//! Tag management with validation and friendly errors. SQL lives in
//! `db::tags`; this layer checks names/colors and explains failures.

use rusqlite::{Connection, ErrorCode};
use thiserror::Error;

use crate::database::tags::TagGroup;
use crate::database::{self as db, DbError};

/// Groups created on the very first start, so tagging works right away.
/// The user can rename or delete them; they are never re-created.
const DEFAULT_GROUPS: &[(&str, &str)] = &[
    ("Setup", "#396cd8"),
    ("Mistake", "#cf222e"),
    ("Emotion", "#9a6700"),
];
const SEEDED_KEY: &str = "tags_seeded";

#[derive(Debug, Error)]
pub enum TagError {
    #[error("name can't be empty")]
    EmptyName,
    #[error("name is too long (max 40 characters)")]
    NameTooLong,
    #[error("color must look like #1a2b3c")]
    BadColor,
    #[error("{0} already exists")]
    Duplicate(String),
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error(transparent)]
    Db(#[from] DbError),
}

impl From<rusqlite::Error> for TagError {
    fn from(e: rusqlite::Error) -> Self {
        TagError::Db(e.into())
    }
}

fn clean_name(name: &str) -> Result<String, TagError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(TagError::EmptyName);
    }
    if name.chars().count() > 40 {
        return Err(TagError::NameTooLong);
    }
    Ok(name.to_string())
}

/// "" (no color) or "#rrggbb", stored lowercase.
fn clean_color(color: &str) -> Result<String, TagError> {
    let c = color.trim();
    let ok = c.is_empty()
        || (c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit()));
    if ok {
        Ok(c.to_ascii_lowercase())
    } else {
        Err(TagError::BadColor)
    }
}

/// A UNIQUE violation becomes "Tag group "X" already exists" instead of a
/// raw SQLite message.
fn unique_as_duplicate(e: DbError, what: String) -> TagError {
    match &e {
        DbError::Sqlite(rusqlite::Error::SqliteFailure(f, _))
            if f.code == ErrorCode::ConstraintViolation =>
        {
            TagError::Duplicate(what)
        }
        _ => TagError::Db(e),
    }
}

pub fn seed_defaults(conn: &Connection) -> Result<(), TagError> {
    if db::settings::get(conn, SEEDED_KEY)?.is_some() {
        return Ok(());
    }
    let tx = conn.unchecked_transaction()?;
    let empty: bool = tx.query_row("SELECT NOT EXISTS (SELECT 1 FROM tag_groups)", [], |r| {
        r.get(0)
    })?;
    if empty {
        for (name, color) in DEFAULT_GROUPS {
            db::tags::create_group(&tx, name, color)?;
        }
    }
    db::settings::set(&tx, SEEDED_KEY, &serde_json::json!(true))?;
    tx.commit()?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<TagGroup>, TagError> {
    Ok(db::tags::list(conn)?)
}

// Names are unique ignoring case ("Breakout" = "breakout"). The DB's UNIQUE
// index is case-sensitive, so the case-insensitive check happens here; the
// index still guards against exact duplicates.

fn check_group_name(conn: &Connection, name: &str, except: Option<i64>) -> Result<(), TagError> {
    if db::tags::group_name_taken(conn, name, except)? {
        return Err(TagError::Duplicate(format!("Tag group \"{name}\"")));
    }
    Ok(())
}

fn check_tag_name(
    conn: &Connection,
    group_id: i64,
    name: &str,
    except: Option<i64>,
) -> Result<(), TagError> {
    if db::tags::tag_name_taken(conn, group_id, name, except)? {
        return Err(TagError::Duplicate(format!("Tag \"{name}\" in this group")));
    }
    Ok(())
}

pub fn create_group(conn: &Connection, name: &str, color: &str) -> Result<i64, TagError> {
    let (name, color) = (clean_name(name)?, clean_color(color)?);
    check_group_name(conn, &name, None)?;
    db::tags::create_group(conn, &name, &color)
        .map_err(|e| unique_as_duplicate(e, format!("Tag group \"{name}\"")))
}

pub fn update_group(conn: &Connection, id: i64, name: &str, color: &str) -> Result<(), TagError> {
    let (name, color) = (clean_name(name)?, clean_color(color)?);
    check_group_name(conn, &name, Some(id))?;
    let found = db::tags::update_group(conn, id, &name, &color)
        .map_err(|e| unique_as_duplicate(e, format!("Tag group \"{name}\"")))?;
    found.then_some(()).ok_or(TagError::NotFound("tag group"))
}

pub fn delete_group(conn: &Connection, id: i64) -> Result<(), TagError> {
    db::tags::delete_group(conn, id)?
        .then_some(())
        .ok_or(TagError::NotFound("tag group"))
}

pub fn create_tag(
    conn: &Connection,
    group_id: i64,
    name: &str,
    color: &str,
) -> Result<i64, TagError> {
    let (name, color) = (clean_name(name)?, clean_color(color)?);
    if !db::tags::group_exists(conn, group_id)? {
        return Err(TagError::NotFound("tag group"));
    }
    check_tag_name(conn, group_id, &name, None)?;
    db::tags::create_tag(conn, group_id, &name, &color)
        .map_err(|e| unique_as_duplicate(e, format!("Tag \"{name}\" in this group")))
}

pub fn update_tag(conn: &Connection, id: i64, name: &str, color: &str) -> Result<(), TagError> {
    let (name, color) = (clean_name(name)?, clean_color(color)?);
    let group_id = db::tags::tag_group_id(conn, id)?.ok_or(TagError::NotFound("tag"))?;
    check_tag_name(conn, group_id, &name, Some(id))?;
    let found = db::tags::update_tag(conn, id, &name, &color)
        .map_err(|e| unique_as_duplicate(e, format!("Tag \"{name}\" in this group")))?;
    found.then_some(()).ok_or(TagError::NotFound("tag"))
}

pub fn delete_tag(conn: &Connection, id: i64) -> Result<(), TagError> {
    db::tags::delete_tag(conn, id)?
        .then_some(())
        .ok_or(TagError::NotFound("tag"))
}

/// Replaces a trade's tags. All-or-nothing: an unknown tag id fails the whole
/// change (foreign key) and nothing is altered.
pub fn set_trade_tags(
    conn: &Connection,
    trade_id: i64,
    tag_ids: &[i64],
) -> Result<Vec<i64>, TagError> {
    let tx = conn.unchecked_transaction()?;
    if db::trades::get(&tx, trade_id)?.is_none() {
        return Err(TagError::NotFound("trade"));
    }
    db::tags::set_for_trade(&tx, trade_id, tag_ids).map_err(|e| match e {
        DbError::Sqlite(rusqlite::Error::SqliteFailure(f, _))
            if f.code == ErrorCode::ConstraintViolation =>
        {
            TagError::NotFound("tag")
        }
        other => TagError::Db(other),
    })?;
    let ids = db::tags::ids_for_trade(&tx, trade_id)?;
    tx.commit()?;
    Ok(ids)
}

pub fn trade_tags(conn: &Connection, trade_id: i64) -> Result<Vec<i64>, TagError> {
    Ok(db::tags::ids_for_trade(conn, trade_id)?)
}

#[cfg(test)]
mod tests;
