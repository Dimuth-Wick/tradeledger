//! Trade log query: filter, sort, paginate, and summarize the filtered set.
//!
//! Filters that SQL can do exactly (dates, symbol, market, side, status,
//! tags) run in SQL. Sorting by money values, the win/loss filter and paging
//! run here with `Decimal`, because PnL is stored as text and SQL would sort
//! "10" before "9" (or need floats, which round).

use std::cmp::Ordering;

use rusqlite::Connection;
use rusqlite::types::Value;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::db::{self, DbError};
use crate::models::{Direction, Market, TimestampMs, Trade, TradeStatus, WithId};

pub const DEFAULT_PAGE_SIZE: u32 = 25;
pub const MAX_PAGE_SIZE: u32 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// Closed with net PnL > 0.
    Win,
    /// Closed with net PnL < 0.
    Loss,
    /// Closed with net PnL exactly 0.
    Breakeven,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortField {
    #[default]
    OpenTime,
    CloseTime,
    Symbol,
    NetPnl,
    RMultiple,
}

fn default_true() -> bool {
    true
}

fn default_page_size() -> u32 {
    DEFAULT_PAGE_SIZE
}

/// What the trade log sends. Every filter is optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeQuery {
    /// Opened at or after this time (ms, inclusive).
    pub from: Option<TimestampMs>,
    /// Opened before this time (ms, exclusive).
    pub to: Option<TimestampMs>,
    /// Part of the symbol, any case ("btc" matches "BTCUSDT").
    pub symbol: Option<String>,
    pub market: Option<Market>,
    pub direction: Option<Direction>,
    pub status: Option<TradeStatus>,
    /// Implies closed trades only.
    pub outcome: Option<Outcome>,
    /// Trades having ANY of these tags. Empty = no tag filter.
    #[serde(default)]
    pub tag_ids: Vec<i64>,
    #[serde(default)]
    pub sort: SortField,
    #[serde(default = "default_true")]
    pub descending: bool,
    /// 0-based.
    #[serde(default)]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

impl Default for TradeQuery {
    fn default() -> Self {
        Self {
            from: None,
            to: None,
            symbol: None,
            market: None,
            direction: None,
            status: None,
            outcome: None,
            tag_ids: Vec::new(),
            sort: SortField::OpenTime,
            descending: true,
            page: 0,
            page_size: DEFAULT_PAGE_SIZE,
        }
    }
}

/// One page of results plus totals over ALL filtered trades.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradePage {
    pub items: Vec<WithId<Trade>>,
    /// Number of trades matching the filters (all pages).
    pub total: u32,
    /// The page actually returned (clamped if the request was past the end).
    pub page: u32,
    pub page_size: u32,
    pub page_count: u32,
    /// Sum of net PnL of the matching closed trades.
    pub closed_net_pnl: Decimal,
    pub wins: u32,
    pub losses: u32,
}

fn outcome_of(t: &Trade) -> Option<Outcome> {
    if t.status != TradeStatus::Closed {
        return None;
    }
    Some(match t.net_pnl.cmp(&Decimal::ZERO) {
        Ordering::Greater => Outcome::Win,
        Ordering::Less => Outcome::Loss,
        Ordering::Equal => Outcome::Breakeven,
    })
}

/// Builds the SQL filter. Values are bound as parameters, never formatted
/// into the SQL text.
fn where_clause(q: &TradeQuery) -> (String, Vec<Value>) {
    let mut conds: Vec<String> = vec!["1 = 1".into()];
    let mut vals: Vec<Value> = Vec::new();
    if let Some(from) = q.from {
        conds.push("open_time >= ?".into());
        vals.push(from.into());
    }
    if let Some(to) = q.to {
        conds.push("open_time < ?".into());
        vals.push(to.into());
    }
    if let Some(s) = q.symbol.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        // instr() instead of LIKE: no need to escape % and _ in user input.
        conds.push("instr(symbol, ?) > 0".into());
        vals.push(s.to_uppercase().into());
    }
    if let Some(m) = q.market {
        conds.push("market = ?".into());
        vals.push(m.as_str().to_string().into());
    }
    if let Some(d) = q.direction {
        conds.push("direction = ?".into());
        vals.push(d.as_str().to_string().into());
    }
    let status = if q.outcome.is_some() {
        Some(TradeStatus::Closed)
    } else {
        q.status
    };
    if let Some(s) = status {
        conds.push("status = ?".into());
        vals.push(s.as_str().to_string().into());
    }
    if !q.tag_ids.is_empty() {
        let marks = vec!["?"; q.tag_ids.len()].join(", ");
        conds.push(format!(
            "EXISTS (SELECT 1 FROM trade_tags tt WHERE tt.trade_id = trades.id AND tt.tag_id IN ({marks}))"
        ));
        vals.extend(q.tag_ids.iter().map(|&id| Value::from(id)));
    }
    (conds.join(" AND "), vals)
}

/// Missing values (open trade's close time, no R) always sort last,
/// whichever direction is chosen.
fn cmp_opt<T: Ord>(a: Option<T>, b: Option<T>, descending: bool) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => {
            let o = x.cmp(&y);
            if descending { o.reverse() } else { o }
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn sort(items: &mut [WithId<Trade>], field: SortField, descending: bool) {
    items.sort_by(|a, b| {
        let (x, y) = (&a.item, &b.item);
        let primary = match field {
            SortField::OpenTime => cmp_opt(Some(x.open_time), Some(y.open_time), descending),
            SortField::CloseTime => cmp_opt(x.close_time, y.close_time, descending),
            SortField::Symbol => cmp_opt(Some(&x.symbol), Some(&y.symbol), descending),
            SortField::NetPnl => cmp_opt(Some(x.net_pnl), Some(y.net_pnl), descending),
            SortField::RMultiple => cmp_opt(x.r_multiple, y.r_multiple, descending),
        };
        // Stable, predictable ties: newest first, then highest id.
        primary
            .then_with(|| y.open_time.cmp(&x.open_time))
            .then_with(|| b.id.cmp(&a.id))
    });
}

pub fn query(conn: &Connection, q: &TradeQuery) -> Result<TradePage, DbError> {
    let (where_sql, params) = where_clause(q);
    let mut items = db::trades::list_where(conn, &where_sql, &params)?;
    if let Some(wanted) = q.outcome {
        items.retain(|t| outcome_of(&t.item) == Some(wanted));
    }

    let (mut closed_net_pnl, mut wins, mut losses) = (Decimal::ZERO, 0u32, 0u32);
    for t in &items {
        match outcome_of(&t.item) {
            Some(Outcome::Win) => wins += 1,
            Some(Outcome::Loss) => losses += 1,
            _ => {}
        }
        if t.item.status == TradeStatus::Closed {
            closed_net_pnl += t.item.net_pnl;
        }
    }

    sort(&mut items, q.sort, q.descending);

    let total = u32::try_from(items.len()).unwrap_or(u32::MAX);
    let page_size = q.page_size.clamp(1, MAX_PAGE_SIZE);
    let page_count = total.div_ceil(page_size).max(1);
    let page = q.page.min(page_count - 1);
    let start = (page * page_size) as usize;
    let items: Vec<_> = items
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .collect();

    Ok(TradePage {
        items,
        total,
        page,
        page_size,
        page_count,
        closed_net_pnl: closed_net_pnl.normalize(),
        wins,
        losses,
    })
}

#[cfg(test)]
mod tests;
