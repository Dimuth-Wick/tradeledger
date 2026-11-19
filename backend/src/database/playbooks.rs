//! Playbooks, their rules, and which rules each trade followed. Plain SQL
//! only; validation lives in `crate::features::playbooks`.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::DbError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: i64,
    pub text: String,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Playbook {
    pub id: i64,
    pub name: String,
    pub description: String,
    /// In checklist order.
    pub rules: Vec<Rule>,
    /// Trades linked to this playbook.
    pub used_by: u32,
}

pub fn rules(conn: &Connection, playbook_id: i64) -> Result<Vec<Rule>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, text, position FROM playbook_rules WHERE playbook_id = ?1 ORDER BY position, id",
    )?;
    let rows = stmt.query_map([playbook_id], |r| {
        Ok(Rule {
            id: r.get(0)?,
            text: r.get(1)?,
            position: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn list(conn: &Connection) -> Result<Vec<Playbook>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.description,
                (SELECT COUNT(*) FROM trades t WHERE t.playbook_id = p.id)
           FROM playbooks p ORDER BY p.name COLLATE NOCASE, p.id",
    )?;
    let heads = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, u32>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    heads
        .into_iter()
        .map(|(id, name, description, used_by)| {
            Ok(Playbook {
                id,
                name,
                description,
                rules: rules(conn, id)?,
                used_by,
            })
        })
        .collect()
}

pub fn exists(conn: &Connection, id: i64) -> Result<bool, DbError> {
    Ok(conn
        .query_row("SELECT 1 FROM playbooks WHERE id = ?1", [id], |_| Ok(()))
        .optional()?
        .is_some())
}

/// Is `name` used by another playbook (ignoring case)?
pub fn name_taken(conn: &Connection, name: &str, except: Option<i64>) -> Result<bool, DbError> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM playbooks WHERE name = ?1 COLLATE NOCASE AND id IS NOT ?2",
            params![name, except],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub fn insert(conn: &Connection, name: &str, description: &str) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO playbooks (name, description) VALUES (?1, ?2)",
        params![name, description],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, name: &str, description: &str) -> Result<bool, DbError> {
    Ok(conn.execute(
        "UPDATE playbooks SET name = ?1, description = ?2 WHERE id = ?3",
        params![name, description, id],
    )? > 0)
}

/// Its rules and checks go too; linked trades just lose the link.
pub fn delete(conn: &Connection, id: i64) -> Result<bool, DbError> {
    Ok(conn.execute("DELETE FROM playbooks WHERE id = ?1", [id])? > 0)
}

pub fn insert_rule(
    conn: &Connection,
    playbook_id: i64,
    text: &str,
    position: i64,
) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO playbook_rules (playbook_id, text, position) VALUES (?1, ?2, ?3)",
        params![playbook_id, text, position],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_rule(conn: &Connection, id: i64, text: &str, position: i64) -> Result<(), DbError> {
    conn.execute(
        "UPDATE playbook_rules SET text = ?1, position = ?2 WHERE id = ?3",
        params![text, position, id],
    )?;
    Ok(())
}

/// Also removes the rule's ticks on every trade.
pub fn delete_rule(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM playbook_rules WHERE id = ?1", [id])?;
    Ok(())
}

pub fn trade_playbook(conn: &Connection, trade_id: i64) -> Result<Option<Option<i64>>, DbError> {
    Ok(conn
        .query_row(
            "SELECT playbook_id FROM trades WHERE id = ?1",
            [trade_id],
            |r| r.get(0),
        )
        .optional()?)
}

pub fn set_trade_playbook(
    conn: &Connection,
    trade_id: i64,
    playbook_id: Option<i64>,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE trades SET playbook_id = ?1 WHERE id = ?2",
        params![playbook_id, trade_id],
    )?;
    Ok(())
}

/// Rule ids ticked on a trade.
pub fn checked_rules(conn: &Connection, trade_id: i64) -> Result<Vec<i64>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT rule_id FROM playbook_checks WHERE trade_id = ?1 AND checked = 1 ORDER BY rule_id",
    )?;
    let rows = stmt.query_map([trade_id], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Replaces the trade's ticks with exactly `rule_ids`.
pub fn set_checked(conn: &Connection, trade_id: i64, rule_ids: &[i64]) -> Result<(), DbError> {
    conn.execute(
        "DELETE FROM playbook_checks WHERE trade_id = ?1",
        [trade_id],
    )?;
    let mut stmt = conn
        .prepare("INSERT INTO playbook_checks (trade_id, rule_id, checked) VALUES (?1, ?2, 1)")?;
    for id in rule_ids {
        stmt.execute(params![trade_id, id])?;
    }
    Ok(())
}

/// Ticked rule ids of every trade (for stats).
pub fn all_checked(conn: &Connection) -> Result<BTreeMap<i64, Vec<i64>>, DbError> {
    let mut stmt =
        conn.prepare("SELECT trade_id, rule_id FROM playbook_checks WHERE checked = 1")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
    let mut map: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
    for row in rows {
        let (trade, rule) = row?;
        map.entry(trade).or_default().push(rule);
    }
    Ok(map)
}
