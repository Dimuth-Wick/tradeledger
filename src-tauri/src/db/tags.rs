//! Tag groups, tags, and which tags each trade has. Plain SQL only; names
//! are validated in `crate::tags`.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::DbError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: i64,
    pub group_id: i64,
    pub name: String,
    /// "#rrggbb", or "" to use the group's color.
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagGroup {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub position: i64,
    pub tags: Vec<Tag>,
}

pub fn create_group(conn: &Connection, name: &str, color: &str) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO tag_groups (name, color, position)
         VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM tag_groups))",
        params![name, color],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_group(conn: &Connection, id: i64, name: &str, color: &str) -> Result<bool, DbError> {
    Ok(conn.execute(
        "UPDATE tag_groups SET name = ?1, color = ?2 WHERE id = ?3",
        params![name, color, id],
    )? > 0)
}

/// Also deletes its tags and their links to trades (ON DELETE CASCADE).
pub fn delete_group(conn: &Connection, id: i64) -> Result<bool, DbError> {
    Ok(conn.execute("DELETE FROM tag_groups WHERE id = ?1", [id])? > 0)
}

pub fn create_tag(
    conn: &Connection,
    group_id: i64,
    name: &str,
    color: &str,
) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO tags (group_id, name, color) VALUES (?1, ?2, ?3)",
        params![group_id, name, color],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_tag(conn: &Connection, id: i64, name: &str, color: &str) -> Result<bool, DbError> {
    Ok(conn.execute(
        "UPDATE tags SET name = ?1, color = ?2 WHERE id = ?3",
        params![name, color, id],
    )? > 0)
}

pub fn delete_tag(conn: &Connection, id: i64) -> Result<bool, DbError> {
    Ok(conn.execute("DELETE FROM tags WHERE id = ?1", [id])? > 0)
}

/// Is a group name used (ignoring case), other than by group `except`?
pub fn group_name_taken(
    conn: &Connection,
    name: &str,
    except: Option<i64>,
) -> Result<bool, DbError> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM tag_groups
                        WHERE name = ?1 COLLATE NOCASE AND id IS NOT ?2)",
        params![name, except],
        |r| r.get(0),
    )?)
}

/// Is a tag name used in this group (ignoring case), other than by tag `except`?
pub fn tag_name_taken(
    conn: &Connection,
    group_id: i64,
    name: &str,
    except: Option<i64>,
) -> Result<bool, DbError> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM tags
                        WHERE group_id = ?1 AND name = ?2 COLLATE NOCASE AND id IS NOT ?3)",
        params![group_id, name, except],
        |r| r.get(0),
    )?)
}

pub fn tag_group_id(conn: &Connection, tag_id: i64) -> Result<Option<i64>, DbError> {
    Ok(conn
        .query_row("SELECT group_id FROM tags WHERE id = ?1", [tag_id], |r| {
            r.get(0)
        })
        .optional()?)
}

pub fn group_exists(conn: &Connection, id: i64) -> Result<bool, DbError> {
    Ok(conn
        .query_row("SELECT 1 FROM tag_groups WHERE id = ?1", [id], |_| Ok(()))
        .optional()?
        .is_some())
}

/// All groups (in display order) with their tags (A→Z).
pub fn list(conn: &Connection) -> Result<Vec<TagGroup>, DbError> {
    let mut groups: Vec<TagGroup> = conn
        .prepare("SELECT id, name, color, position FROM tag_groups ORDER BY position, id")?
        .query_map([], |r| {
            Ok(TagGroup {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                position: r.get(3)?,
                tags: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    let mut stmt = conn
        .prepare("SELECT id, group_id, name, color FROM tags ORDER BY name COLLATE NOCASE, id")?;
    let tags = stmt.query_map([], |r| {
        Ok(Tag {
            id: r.get(0)?,
            group_id: r.get(1)?,
            name: r.get(2)?,
            color: r.get(3)?,
        })
    })?;
    for tag in tags {
        let tag = tag?;
        if let Some(g) = groups.iter_mut().find(|g| g.id == tag.group_id) {
            g.tags.push(tag);
        }
    }
    Ok(groups)
}

/// Replaces a trade's tags with exactly `tag_ids`. Call inside a transaction.
pub fn set_for_trade(conn: &Connection, trade_id: i64, tag_ids: &[i64]) -> Result<(), DbError> {
    conn.execute("DELETE FROM trade_tags WHERE trade_id = ?1", [trade_id])?;
    let mut stmt =
        conn.prepare("INSERT OR IGNORE INTO trade_tags (trade_id, tag_id) VALUES (?1, ?2)")?;
    for id in tag_ids {
        stmt.execute(params![trade_id, id])?;
    }
    Ok(())
}

pub fn ids_for_trade(conn: &Connection, trade_id: i64) -> Result<Vec<i64>, DbError> {
    let mut stmt =
        conn.prepare("SELECT tag_id FROM trade_tags WHERE trade_id = ?1 ORDER BY tag_id")?;
    let rows = stmt.query_map([trade_id], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Tag ids of every tagged trade (for reports over many trades).
pub fn all_trade_tags(conn: &Connection) -> Result<BTreeMap<i64, Vec<i64>>, DbError> {
    let mut stmt =
        conn.prepare("SELECT trade_id, tag_id FROM trade_tags ORDER BY trade_id, tag_id")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
    let mut map: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
    for row in rows {
        let (trade, tag) = row?;
        map.entry(trade).or_default().push(tag);
    }
    Ok(map)
}

/// Tag ids per trade, for a page of the trade log.
pub fn ids_for_trades(
    conn: &Connection,
    trade_ids: &[i64],
) -> Result<BTreeMap<i64, Vec<i64>>, DbError> {
    let mut map: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
    if trade_ids.is_empty() {
        return Ok(map);
    }
    let marks = vec!["?"; trade_ids.len()].join(", ");
    let sql = format!(
        "SELECT trade_id, tag_id FROM trade_tags WHERE trade_id IN ({marks}) ORDER BY trade_id, tag_id"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(trade_ids), |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (trade, tag) = row?;
        map.entry(trade).or_default().push(tag);
    }
    Ok(map)
}
