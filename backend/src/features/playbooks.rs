//! Playbooks: a named setup with a checklist of rules. Each trade can follow
//! one playbook and tick the rules it respected; stats then compare trades
//! that followed every rule with those that skipped one.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::database::playbooks::{self as repo, Playbook};
use crate::database::{self as db, DbError};
use crate::features::reports::{self, ReportRow};
use crate::models::{Trade, TradeStatus, WithId};

const MAX_NAME: usize = 60;
const MAX_DESCRIPTION: usize = 1000;
const MAX_RULE: usize = 200;
const MAX_RULES: usize = 30;

#[derive(Debug, Error)]
pub enum PlaybookError {
    #[error("name can't be empty")]
    EmptyName,
    #[error("name is too long (max {MAX_NAME} characters)")]
    NameTooLong,
    #[error("a playbook named \"{0}\" already exists")]
    DuplicateName(String),
    #[error("description is too long (max {MAX_DESCRIPTION} characters)")]
    DescriptionTooLong,
    #[error("too many rules (max {MAX_RULES})")]
    TooManyRules,
    #[error("rule {0} is empty")]
    EmptyRule(usize),
    #[error("rule {0} is too long (max {MAX_RULE} characters)")]
    RuleTooLong(usize),
    #[error("rule \"{0}\" is listed twice")]
    DuplicateRule(String),
    #[error("rule {0} doesn't belong to this playbook")]
    ForeignRule(i64),
    #[error("{0} not found")]
    NotFound(String),
    #[error(transparent)]
    Db(#[from] DbError),
}

impl From<rusqlite::Error> for PlaybookError {
    fn from(e: rusqlite::Error) -> Self {
        PlaybookError::Db(e.into())
    }
}

/// A rule as edited in the UI: `id` = existing rule (keeps its ticks on
/// trades), `None` = new rule.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleInput {
    pub id: Option<i64>,
    pub text: String,
}

fn clean_name(conn: &Connection, name: &str, except: Option<i64>) -> Result<String, PlaybookError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(PlaybookError::EmptyName);
    }
    if name.chars().count() > MAX_NAME {
        return Err(PlaybookError::NameTooLong);
    }
    if repo::name_taken(conn, name, except)? {
        return Err(PlaybookError::DuplicateName(name.to_string()));
    }
    Ok(name.to_string())
}

fn clean_description(d: &str) -> Result<String, PlaybookError> {
    let d = d.trim();
    if d.chars().count() > MAX_DESCRIPTION {
        return Err(PlaybookError::DescriptionTooLong);
    }
    Ok(d.to_string())
}

/// Trimmed rule texts; numbers in errors are 1-based like the UI list.
fn clean_rules(texts: &[&str]) -> Result<Vec<String>, PlaybookError> {
    if texts.len() > MAX_RULES {
        return Err(PlaybookError::TooManyRules);
    }
    let mut seen = BTreeSet::new();
    texts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let t = t.trim();
            if t.is_empty() {
                return Err(PlaybookError::EmptyRule(i + 1));
            }
            if t.chars().count() > MAX_RULE {
                return Err(PlaybookError::RuleTooLong(i + 1));
            }
            if !seen.insert(t.to_lowercase()) {
                return Err(PlaybookError::DuplicateRule(t.to_string()));
            }
            Ok(t.to_string())
        })
        .collect()
}

pub fn list(conn: &Connection) -> Result<Vec<Playbook>, PlaybookError> {
    Ok(repo::list(conn)?)
}

pub fn create(
    conn: &Connection,
    name: &str,
    description: &str,
    rules: &[String],
) -> Result<i64, PlaybookError> {
    let name = clean_name(conn, name, None)?;
    let description = clean_description(description)?;
    let texts: Vec<&str> = rules.iter().map(String::as_str).collect();
    let rules = clean_rules(&texts)?;
    let tx = conn.unchecked_transaction()?;
    let id = repo::insert(&tx, &name, &description)?;
    for (pos, text) in rules.iter().enumerate() {
        repo::insert_rule(&tx, id, text, pos as i64)?;
    }
    tx.commit()?;
    Ok(id)
}

/// Saves name, description and the full rule list in this order. Rules left
/// out are deleted (with their ticks); kept ones keep their ticks.
pub fn update(
    conn: &Connection,
    id: i64,
    name: &str,
    description: &str,
    rules: &[RuleInput],
) -> Result<(), PlaybookError> {
    if !repo::exists(conn, id)? {
        return Err(PlaybookError::NotFound(format!("playbook {id}")));
    }
    let name = clean_name(conn, name, Some(id))?;
    let description = clean_description(description)?;
    let texts: Vec<&str> = rules.iter().map(|r| r.text.as_str()).collect();
    let texts = clean_rules(&texts)?;
    let existing: BTreeSet<i64> = repo::rules(conn, id)?.iter().map(|r| r.id).collect();
    let kept: BTreeSet<i64> = rules.iter().filter_map(|r| r.id).collect();
    if let Some(bad) = kept.iter().find(|r| !existing.contains(r)) {
        return Err(PlaybookError::ForeignRule(*bad));
    }

    let tx = conn.unchecked_transaction()?;
    repo::update(&tx, id, &name, &description)?;
    for gone in existing.difference(&kept) {
        repo::delete_rule(&tx, *gone)?;
    }
    for (pos, (input, text)) in rules.iter().zip(&texts).enumerate() {
        match input.id {
            Some(rule_id) => repo::update_rule(&tx, rule_id, text, pos as i64)?,
            None => {
                repo::insert_rule(&tx, id, text, pos as i64)?;
            }
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), PlaybookError> {
    if !repo::delete(conn, id)? {
        return Err(PlaybookError::NotFound(format!("playbook {id}")));
    }
    Ok(())
}

/// A trade's playbook and the rules it ticked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradePlaybook {
    pub playbook_id: Option<i64>,
    pub checked: Vec<i64>,
}

pub fn for_trade(conn: &Connection, trade_id: i64) -> Result<TradePlaybook, PlaybookError> {
    let playbook_id = repo::trade_playbook(conn, trade_id)?
        .ok_or_else(|| PlaybookError::NotFound(format!("trade {trade_id}")))?;
    // Only ticks of the current playbook's rules count (a merge can bring
    // ticks from another playbook along).
    let rules: BTreeSet<i64> = match playbook_id {
        Some(p) => repo::rules(conn, p)?.iter().map(|r| r.id).collect(),
        None => BTreeSet::new(),
    };
    let checked = repo::checked_rules(conn, trade_id)?
        .into_iter()
        .filter(|r| rules.contains(r))
        .collect();
    Ok(TradePlaybook {
        playbook_id,
        checked,
    })
}

/// Links the trade to a playbook (or none) and stores its ticks, all at once.
pub fn set_for_trade(
    conn: &Connection,
    trade_id: i64,
    playbook_id: Option<i64>,
    checked: &[i64],
) -> Result<TradePlaybook, PlaybookError> {
    if repo::trade_playbook(conn, trade_id)?.is_none() {
        return Err(PlaybookError::NotFound(format!("trade {trade_id}")));
    }
    let rules: BTreeSet<i64> = match playbook_id {
        Some(p) => {
            if !repo::exists(conn, p)? {
                return Err(PlaybookError::NotFound(format!("playbook {p}")));
            }
            repo::rules(conn, p)?.iter().map(|r| r.id).collect()
        }
        None => BTreeSet::new(),
    };
    let ticks: BTreeSet<i64> = checked.iter().copied().collect();
    if let Some(bad) = ticks.iter().find(|r| !rules.contains(r)) {
        return Err(PlaybookError::ForeignRule(*bad));
    }
    let ticks: Vec<i64> = ticks.into_iter().collect();
    let tx = conn.unchecked_transaction()?;
    repo::set_trade_playbook(&tx, trade_id, playbook_id)?;
    repo::set_checked(&tx, trade_id, &ticks)?;
    tx.commit()?;
    Ok(TradePlaybook {
        playbook_id,
        checked: ticks,
    })
}

/// Results of one playbook's closed trades.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybookStats {
    pub playbook_id: i64,
    pub all: ReportRow,
    /// Trades that ticked every rule.
    pub followed: ReportRow,
    /// Trades that skipped at least one rule.
    pub broke: ReportRow,
    /// Average share of rules ticked, percent. None without trades or rules.
    pub adherence: Option<Decimal>,
}

/// Pure: stats per playbook from trades, rule lists and ticks.
pub fn compute_stats(
    playbooks: &[Playbook],
    trades: &[WithId<Trade>],
    ticks: &BTreeMap<i64, Vec<i64>>,
) -> Vec<PlaybookStats> {
    playbooks
        .iter()
        .map(|p| {
            let rules: BTreeSet<i64> = p.rules.iter().map(|r| r.id).collect();
            let mine: Vec<&WithId<Trade>> = trades
                .iter()
                .filter(|t| {
                    t.item.status == TradeStatus::Closed && t.item.playbook_id == Some(p.id)
                })
                .collect();
            let ticked = |t: &WithId<Trade>| {
                ticks
                    .get(&t.id)
                    .map_or(0, |ids| ids.iter().filter(|r| rules.contains(r)).count())
            };
            let (followed, broke): (Vec<&WithId<Trade>>, Vec<&WithId<Trade>>) = mine
                .iter()
                .partition(|t| !rules.is_empty() && ticked(t) == rules.len());
            let adherence = (!rules.is_empty() && !mine.is_empty()).then(|| {
                let share: Decimal = mine
                    .iter()
                    .map(|t| Decimal::from(ticked(t) as u64) / Decimal::from(rules.len() as u64))
                    .sum();
                (share * Decimal::ONE_HUNDRED / Decimal::from(mine.len() as u64))
                    .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
                    .normalize()
            });
            PlaybookStats {
                playbook_id: p.id,
                all: reports::row(p.id.to_string(), p.name.clone(), &mine),
                followed: reports::row("followed", "Followed all rules", &followed),
                broke: reports::row("broke", "Skipped a rule", &broke),
                adherence,
            }
        })
        .collect()
}

pub fn stats(conn: &Connection) -> Result<Vec<PlaybookStats>, PlaybookError> {
    let playbooks = repo::list(conn)?;
    let trades = db::trades::list(conn)?;
    let ticks = repo::all_checked(conn)?;
    Ok(compute_stats(&playbooks, &trades, &ticks))
}

#[cfg(test)]
mod tests;
