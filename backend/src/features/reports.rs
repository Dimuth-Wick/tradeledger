//! Reports: the stats split into groups (by tag, symbol, weekday, hour,
//! market or side), to see where the edge is and where money leaks.
//!
//! Every row runs the same `stats::compute` as the dashboard, so the
//! formulas are identical. Only closed trades are grouped. Weekday and hour
//! use the **open** time (when the decision was made) in the saved zone.

use std::collections::BTreeMap;

use chrono::{Datelike, Timelike};
use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::database::{self as db, DbError, tags::TagGroup};
use crate::features::calendar::{self, Zone};
use crate::features::stats;
use crate::features::trade_query::{self, TradeQuery};
use crate::models::{Direction, Market, Trade, TradeStatus, WithId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupBy {
    Tag,
    Symbol,
    Weekday,
    Hour,
    Market,
    Direction,
    Playbook,
}

/// One group's numbers (closed trades only).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRow {
    /// Stable id of the group, e.g. "tag:3", "BTCUSDT", "mon", "14".
    pub key: String,
    pub label: String,
    pub trades: u32,
    pub wins: u32,
    pub losses: u32,
    pub win_rate: Option<Decimal>,
    pub net_pnl: Decimal,
    /// Net PnL per trade (expectancy).
    pub avg_pnl: Option<Decimal>,
    /// None when there are no losses (∞ if wins > 0).
    pub profit_factor: Option<Decimal>,
    pub avg_r: Option<Decimal>,
    pub trades_with_r: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub group_by: GroupBy,
    pub rows: Vec<ReportRow>,
    /// All closed trades together (by tag, a trade can sit in several rows,
    /// so the rows can add up to more than this).
    pub total: ReportRow,
    /// Matching trades left out because they are still open.
    pub open_trades: u32,
}

pub(crate) fn row(
    key: impl Into<String>,
    label: impl Into<String>,
    trades: &[&WithId<Trade>],
) -> ReportRow {
    let refs: Vec<(i64, &Trade)> = trades.iter().map(|t| (t.id, &t.item)).collect();
    let s = stats::compute(&refs);
    ReportRow {
        key: key.into(),
        label: label.into(),
        trades: s.closed,
        wins: s.wins,
        losses: s.losses,
        win_rate: s.win_rate,
        net_pnl: s.net_pnl,
        avg_pnl: s.expectancy,
        profit_factor: s.profit_factor,
        avg_r: s.avg_r,
        trades_with_r: s.trades_with_r,
    }
}

const WEEKDAYS: [(&str, &str); 7] = [
    ("mon", "Monday"),
    ("tue", "Tuesday"),
    ("wed", "Wednesday"),
    ("thu", "Thursday"),
    ("fri", "Friday"),
    ("sat", "Saturday"),
    ("sun", "Sunday"),
];

fn market_label(m: Market) -> &'static str {
    match m {
        Market::Spot => "Spot",
        Market::Usdm => "Futures",
    }
}

fn direction_label(d: Direction) -> &'static str {
    match d {
        Direction::Long => "Long",
        Direction::Short => "Short",
    }
}

/// Biggest profit first; ties by label so the order is stable.
fn by_net_desc(rows: &mut [ReportRow]) {
    rows.sort_by(|a, b| {
        b.net_pnl
            .cmp(&a.net_pnl)
            .then_with(|| a.label.cmp(&b.label))
    });
}

/// Rows keyed by a text value (symbol, market, side), best first.
fn group_by_key<'a>(
    closed: &[&'a WithId<Trade>],
    key: impl Fn(&Trade) -> (String, String),
) -> Vec<ReportRow> {
    let mut groups: BTreeMap<String, (String, Vec<&'a WithId<Trade>>)> = BTreeMap::new();
    for t in closed {
        let (k, label) = key(&t.item);
        groups
            .entry(k)
            .or_insert_with(|| (label, Vec::new()))
            .1
            .push(t);
    }
    let mut rows: Vec<ReportRow> = groups
        .into_iter()
        .map(|(k, (label, ts))| row(k, label, &ts))
        .collect();
    by_net_desc(&mut rows);
    rows
}

/// Names needed to label tag and playbook groups.
#[derive(Debug, Default)]
pub struct Lookup {
    pub tag_groups: Vec<TagGroup>,
    /// Trade id → tag ids.
    pub trade_tags: BTreeMap<i64, Vec<i64>>,
    /// Playbook id → name.
    pub playbooks: BTreeMap<i64, String>,
}

/// Builds the report from already-filtered trades (pure, for tests).
///
/// `tag_group`: only that group's tags get rows (e.g. just "Mistake");
/// "No tag" then means none from that group.
pub fn build(
    trades: &[WithId<Trade>],
    by: GroupBy,
    zone: Zone,
    lookup: &Lookup,
    tag_group: Option<i64>,
) -> Report {
    let closed: Vec<&WithId<Trade>> = trades
        .iter()
        .filter(|t| t.item.status == TradeStatus::Closed)
        .collect();
    let open_trades = (trades.len() - closed.len()) as u32;

    let rows = match by {
        GroupBy::Symbol => group_by_key(&closed, |t| (t.symbol.clone(), t.symbol.clone())),
        GroupBy::Market => group_by_key(&closed, |t| {
            (
                t.market.as_str().to_string(),
                market_label(t.market).to_string(),
            )
        }),
        GroupBy::Direction => group_by_key(&closed, |t| {
            (
                t.direction.as_str().to_string(),
                direction_label(t.direction).to_string(),
            )
        }),
        // Every weekday and hour gets a row, even empty, so gaps show.
        GroupBy::Weekday => {
            let mut buckets: [Vec<&WithId<Trade>>; 7] = Default::default();
            for t in &closed {
                let wd = zone.local_time(t.item.open_time).weekday();
                buckets[wd.num_days_from_monday() as usize].push(t);
            }
            WEEKDAYS
                .iter()
                .zip(buckets.iter())
                .map(|((k, label), ts)| row(*k, *label, ts))
                .collect()
        }
        GroupBy::Hour => {
            let mut buckets: [Vec<&WithId<Trade>>; 24] = Default::default();
            for t in &closed {
                buckets[zone.local_time(t.item.open_time).hour() as usize].push(t);
            }
            buckets
                .iter()
                .enumerate()
                .map(|(h, ts)| {
                    row(
                        h.to_string(),
                        format!("{h:02}:00–{:02}:00", (h + 1) % 24),
                        ts,
                    )
                })
                .collect()
        }
        GroupBy::Playbook => {
            let mut groups: BTreeMap<i64, Vec<&WithId<Trade>>> = BTreeMap::new();
            let mut none = Vec::new();
            for t in &closed {
                match t
                    .item
                    .playbook_id
                    .filter(|id| lookup.playbooks.contains_key(id))
                {
                    Some(id) => groups.entry(id).or_default().push(t),
                    None => none.push(*t),
                }
            }
            let mut rows: Vec<ReportRow> = groups
                .iter()
                .map(|(id, ts)| row(format!("playbook:{id}"), lookup.playbooks[id].clone(), ts))
                .collect();
            by_net_desc(&mut rows);
            if !none.is_empty() {
                rows.push(row("none", "No playbook", &none));
            }
            rows
        }
        GroupBy::Tag => {
            // Tag id → "Group · Tag" for the tags that get rows.
            let mut known: BTreeMap<i64, String> = BTreeMap::new();
            for g in lookup
                .tag_groups
                .iter()
                .filter(|g| tag_group.is_none_or(|id| id == g.id))
            {
                for tag in &g.tags {
                    known.insert(tag.id, format!("{} · {}", g.name, tag.name));
                }
            }
            let mut groups: BTreeMap<i64, Vec<&WithId<Trade>>> = BTreeMap::new();
            let mut untagged = Vec::new();
            for t in &closed {
                let ids: Vec<i64> = lookup
                    .trade_tags
                    .get(&t.id)
                    .map(|ids| {
                        ids.iter()
                            .copied()
                            .filter(|id| known.contains_key(id))
                            .collect()
                    })
                    .unwrap_or_default();
                if ids.is_empty() {
                    untagged.push(*t);
                }
                for id in ids {
                    groups.entry(id).or_default().push(t);
                }
            }
            let mut rows: Vec<ReportRow> = groups
                .iter()
                .map(|(id, ts)| row(format!("tag:{id}"), known[id].clone(), ts))
                .collect();
            by_net_desc(&mut rows);
            if !untagged.is_empty() {
                rows.push(row("none", "No tag", &untagged));
            }
            rows
        }
    };

    Report {
        group_by: by,
        rows,
        total: row("total", "All closed trades", &closed),
        open_trades,
    }
}

/// Report for the trades matching the dashboard/log filters.
pub fn query(
    conn: &Connection,
    q: &TradeQuery,
    by: GroupBy,
    tag_group: Option<i64>,
) -> Result<Report, DbError> {
    let trades = trade_query::filtered(conn, q)?;
    let zone = calendar::zone(conn)?;
    let mut lookup = Lookup::default();
    match by {
        GroupBy::Tag => {
            lookup.tag_groups = db::tags::list(conn)?;
            lookup.trade_tags = db::tags::all_trade_tags(conn)?;
        }
        GroupBy::Playbook => {
            lookup.playbooks = db::playbooks::list(conn)?
                .into_iter()
                .map(|p| (p.id, p.name))
                .collect();
        }
        _ => {}
    }
    Ok(build(&trades, by, zone, &lookup, tag_group))
}

#[cfg(test)]
mod tests;
