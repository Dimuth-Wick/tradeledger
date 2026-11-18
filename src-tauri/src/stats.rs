//! Performance statistics over a set of trades (plan §4).
//!
//! `compute` is a pure function over trades, so every formula is unit-tested
//! with hand-checked numbers. `query` applies the same filters as the trade
//! log, so the dashboard and the log always agree.
//!
//! Money stays `Decimal` throughout. Only closed trades count towards PnL
//! figures; open trades are counted but have no final result yet.

use std::cmp::Ordering;

use rusqlite::Connection;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::Serialize;

use crate::db::DbError;
use crate::models::{TimestampMs, Trade, TradeStatus};
use crate::trade_query::{self, TradeQuery};

/// Percentages and ratios are shown with 2 decimals.
const RATIO_DP: u32 = 2;
/// Money averages keep crypto precision.
const MONEY_DP: u32 = 8;

/// Edges of the R-multiple histogram: < −2, −2…−1, −1…0, 0…1, 1…2, 2…3, ≥ 3.
const R_EDGES: [i64; 6] = [-2, -1, 0, 1, 2, 3];

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityPoint {
    /// Close time of the trade that produced this point.
    pub time: TimestampMs,
    /// Cumulative closed net PnL after that trade.
    pub equity: Decimal,
    /// Net PnL of that trade alone (for the win/loss bars).
    pub pnl: Decimal,
    pub trade_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RBucket {
    pub label: String,
    /// Inclusive lower edge; `None` = open-ended.
    pub from: Option<i64>,
    /// Exclusive upper edge; `None` = open-ended.
    pub to: Option<i64>,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub trades: u32,
    pub open: u32,
    pub closed: u32,
    pub wins: u32,
    pub losses: u32,
    pub breakeven: u32,
    /// Percent 0–100 of closed trades that won. `None` with no closed trades.
    pub win_rate: Option<Decimal>,
    pub gross_pnl: Decimal,
    pub fees: Decimal,
    pub funding: Decimal,
    pub net_pnl: Decimal,
    pub avg_win: Option<Decimal>,
    /// Negative number (e.g. −3.125).
    pub avg_loss: Option<Decimal>,
    pub largest_win: Option<Decimal>,
    pub largest_loss: Option<Decimal>,
    /// avg win ÷ |avg loss|.
    pub payoff_ratio: Option<Decimal>,
    /// Σ wins ÷ |Σ losses|. `None` when there are no losses (∞ if there
    /// were wins, undefined otherwise; see `wins`).
    pub profit_factor: Option<Decimal>,
    /// Average net PnL per closed trade.
    pub expectancy: Option<Decimal>,
    /// Average R-multiple of closed trades that have one.
    pub avg_r: Option<Decimal>,
    /// How many closed trades have an R-multiple (need a stop loss).
    pub trades_with_r: u32,
    /// Largest peak-to-trough drop of the equity curve, as a positive amount.
    pub max_drawdown: Decimal,
    pub max_drawdown_start: Option<TimestampMs>,
    pub max_drawdown_end: Option<TimestampMs>,
    pub max_win_streak: u32,
    pub max_loss_streak: u32,
    /// Average time from open to close of closed trades.
    pub avg_hold_ms: Option<i64>,
    pub equity_curve: Vec<EquityPoint>,
    pub r_buckets: Vec<RBucket>,
}

fn ratio(value: Decimal) -> Decimal {
    value
        .round_dp_with_strategy(RATIO_DP, RoundingStrategy::MidpointAwayFromZero)
        .normalize()
}

fn money(value: Decimal) -> Decimal {
    value
        .round_dp_with_strategy(MONEY_DP, RoundingStrategy::MidpointAwayFromZero)
        .normalize()
}

fn mean(sum: Decimal, n: u32) -> Option<Decimal> {
    (n > 0).then(|| sum / Decimal::from(n))
}

fn empty_buckets() -> Vec<RBucket> {
    let mut buckets = vec![RBucket {
        label: format!("< {}R", R_EDGES[0]),
        from: None,
        to: Some(R_EDGES[0]),
        count: 0,
    }];
    for pair in R_EDGES.windows(2) {
        buckets.push(RBucket {
            label: format!("{}R to {}R", pair[0], pair[1]),
            from: Some(pair[0]),
            to: Some(pair[1]),
            count: 0,
        });
    }
    let last = R_EDGES[R_EDGES.len() - 1];
    buckets.push(RBucket {
        label: format!("≥ {last}R"),
        from: Some(last),
        to: None,
        count: 0,
    });
    buckets
}

/// Index of the bucket for an R value (lower edge inclusive).
fn bucket_index(r: Decimal) -> usize {
    R_EDGES
        .iter()
        .position(|&edge| r < Decimal::from(edge))
        .unwrap_or(R_EDGES.len())
}

/// Statistics for `trades` (any order). Ids are only used for the equity curve.
pub fn compute(trades: &[(i64, &Trade)]) -> Stats {
    let mut closed: Vec<(i64, &Trade)> = trades
        .iter()
        .copied()
        .filter(|(_, t)| t.status == TradeStatus::Closed)
        .collect();
    // Results happen at close time; id keeps same-time closes stable.
    closed.sort_by_key(|(id, t)| (t.close_time.unwrap_or(t.open_time), *id));

    let (mut wins, mut losses, mut breakeven) = (0u32, 0u32, 0u32);
    let (mut win_sum, mut loss_sum) = (Decimal::ZERO, Decimal::ZERO);
    let (mut gross, mut fees, mut funding, mut net) =
        (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);
    let (mut largest_win, mut largest_loss): (Option<Decimal>, Option<Decimal>) = (None, None);
    let (mut r_sum, mut r_count) = (Decimal::ZERO, 0u32);
    let mut buckets = empty_buckets();
    let (mut hold_sum, mut hold_count) = (0i128, 0i64);

    // Equity curve + drawdown. The curve starts at 0 before the first trade,
    // so an opening loss counts as a drawdown from that starting point.
    let mut equity = Decimal::ZERO;
    let (mut peak, mut peak_time): (Decimal, Option<TimestampMs>) = (Decimal::ZERO, None);
    let mut max_dd = Decimal::ZERO;
    let (mut dd_start, mut dd_end) = (None, None);
    let mut curve = Vec::with_capacity(closed.len());

    let (mut streak_w, mut streak_l, mut max_w, mut max_l) = (0u32, 0u32, 0u32, 0u32);

    for &(id, t) in &closed {
        let time = t.close_time.unwrap_or(t.open_time);
        gross += t.gross_pnl;
        fees += t.fees;
        funding += t.funding;
        net += t.net_pnl;

        match t.net_pnl.cmp(&Decimal::ZERO) {
            Ordering::Greater => {
                wins += 1;
                win_sum += t.net_pnl;
                largest_win = Some(largest_win.map_or(t.net_pnl, |m| m.max(t.net_pnl)));
                streak_w += 1;
                streak_l = 0;
            }
            Ordering::Less => {
                losses += 1;
                loss_sum += t.net_pnl;
                largest_loss = Some(largest_loss.map_or(t.net_pnl, |m| m.min(t.net_pnl)));
                streak_l += 1;
                streak_w = 0;
            }
            Ordering::Equal => {
                breakeven += 1;
                streak_w = 0;
                streak_l = 0;
            }
        }
        max_w = max_w.max(streak_w);
        max_l = max_l.max(streak_l);

        if let Some(r) = t.r_multiple {
            r_sum += r;
            r_count += 1;
            buckets[bucket_index(r)].count += 1;
        }
        if let Some(close) = t.close_time {
            hold_sum += i128::from(close - t.open_time);
            hold_count += 1;
        }

        equity += t.net_pnl;
        curve.push(EquityPoint {
            time,
            equity: money(equity),
            pnl: money(t.net_pnl),
            trade_id: id,
        });
        if equity > peak {
            peak = equity;
            peak_time = Some(time);
        }
        let drawdown = peak - equity;
        if drawdown > max_dd {
            max_dd = drawdown;
            // No peak yet = falling from the starting balance: the
            // drawdown starts where the first trade opened.
            dd_start = peak_time.or(closed.first().map(|(_, f)| f.open_time));
            dd_end = Some(time);
        }
    }

    let closed_count = wins + losses + breakeven;
    let avg_win = mean(win_sum, wins);
    let avg_loss = mean(loss_sum, losses);
    let trade_count = u32::try_from(trades.len()).unwrap_or(u32::MAX);

    Stats {
        trades: trade_count,
        open: trade_count - closed_count,
        closed: closed_count,
        wins,
        losses,
        breakeven,
        win_rate: (closed_count > 0).then(|| {
            ratio(Decimal::from(wins) * Decimal::ONE_HUNDRED / Decimal::from(closed_count))
        }),
        gross_pnl: money(gross),
        fees: money(fees),
        funding: money(funding),
        net_pnl: money(net),
        avg_win: avg_win.map(money),
        avg_loss: avg_loss.map(money),
        largest_win: largest_win.map(money),
        largest_loss: largest_loss.map(money),
        payoff_ratio: match (avg_win, avg_loss) {
            (Some(w), Some(l)) => Some(ratio(w / l.abs())),
            _ => None,
        },
        profit_factor: (losses > 0).then(|| ratio(win_sum / loss_sum.abs())),
        expectancy: mean(net, closed_count).map(money),
        avg_r: mean(r_sum, r_count).map(ratio),
        trades_with_r: r_count,
        max_drawdown: money(max_dd),
        max_drawdown_start: dd_start,
        max_drawdown_end: dd_end,
        max_win_streak: max_w,
        max_loss_streak: max_l,
        avg_hold_ms: (hold_count > 0)
            .then(|| i64::try_from(hold_sum / i128::from(hold_count)).unwrap_or(i64::MAX)),
        equity_curve: curve,
        r_buckets: buckets,
    }
}

/// Stats for the trades matching the trade log filters (sort/page ignored).
pub fn query(conn: &Connection, q: &TradeQuery) -> Result<Stats, DbError> {
    let trades = trade_query::filtered(conn, q)?;
    let refs: Vec<(i64, &Trade)> = trades.iter().map(|t| (t.id, &t.item)).collect();
    Ok(compute(&refs))
}

#[cfg(test)]
mod tests;
