//! Fill grouping engine: turns raw exchange fills into trades.
//!
//! Pure functions only (no DB), so every rule is unit-tested. Works on ONE
//! instrument at a time (exchange + market + symbol).
//!
//! Rules (plan §4):
//! - Walk fills in time order, tracking the open position.
//! - Position 0 → the next fill opens a new trade (long if buy, short if sell).
//! - Position back to exactly 0 → the trade is closed.
//! - A fill that crosses zero (long 0.5, sell 0.8) is split: 0.5 closes the
//!   trade, 0.3 opens a new one in the other direction. Fees are split by
//!   quantity; Binance's realized PnL stays with the closing part.
//! - Spot can't go short: selling more than is held leaves "orphan" quantity
//!   (usually bought before the export's date range), reported, not grouped.
//!
//! Money (all in the quote asset, e.g. USDT):
//! - Gross PnL uses the average-cost method: each exit earns
//!   (exit price − average cost of what's still open) × qty, sign by direction.
//! - Fees: quote-asset fees as-is; base-asset fees × that fill's price; other
//!   assets (BNB) can't be priced offline → reported separately, not in `fees`.
//! - Spot fee paid in the base asset shrinks the position (buy 0.5 ETH with a
//!   0.0005 ETH fee → you hold 0.4995), so the trade closes when 0.4995 is sold.
//! - Funding events between open and close (inclusive) belong to the trade;
//!   `Trade.funding` = −(sum of Binance amounts), i.e. positive = paid.
//! - Net PnL = gross − fees − funding.

pub mod persist;

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use thiserror::Error;

use crate::models::{
    Direction, Fill, FundingEvent, Market, Side, TimestampMs, Trade, TradeStatus, WithId,
};

/// Stored averages and PnL are rounded to this many decimals (crypto
/// precision); internal math keeps full precision until the end.
pub const MONEY_DP: u32 = 8;

/// Quote assets recognized when splitting a symbol into base + quote.
/// Longest first, so "FDUSD" wins over "USD"-like shorter matches.
const QUOTES: &[&str] = &[
    "FDUSD", "USDT", "USDC", "BUSD", "TUSD", "USDP", "DAI", "EUR", "TRY", "BRL", "BTC", "ETH",
    "BNB",
];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GroupError {
    #[error("fills from different instruments can't be grouped together: {0} vs {1}")]
    MixedInstruments(String, String),
}

/// The part of one fill used by one trade. A flip fill appears in two trades.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leg {
    pub fill_id: i64,
    /// Quantity of the fill used here (in the fill's own units).
    pub qty: Decimal,
    /// true = adds to the position, false = reduces it.
    pub is_entry: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupedTrade {
    pub trade: Trade,
    pub legs: Vec<Leg>,
    pub funding_ids: Vec<i64>,
    /// Fees in assets that can't be converted to the quote asset, e.g. BNB.
    pub unconverted_fees: BTreeMap<String, Decimal>,
}

/// Quantity that couldn't be matched (spot sell without enough holdings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orphan {
    pub fill_id: i64,
    pub qty: Decimal,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GroupResult {
    pub trades: Vec<GroupedTrade>,
    pub orphans: Vec<Orphan>,
    /// Funding events that fall outside every trade.
    pub unassigned_funding: Vec<i64>,
}

/// "ETHUSDT" → ("ETH", "USDT"). `None` if no known quote asset matches.
pub fn split_symbol(symbol: &str) -> Option<(&str, &str)> {
    QUOTES.iter().find_map(|q| {
        symbol
            .strip_suffix(q)
            .filter(|base| !base.is_empty())
            .map(|base| (base, *q))
    })
}

fn round(d: Decimal) -> Decimal {
    d.round_dp(MONEY_DP).normalize()
}

/// Running totals for the trade being built.
struct Builder {
    direction: Direction,
    open_time: TimestampMs,
    last_time: TimestampMs,
    /// Open position size, including the spot base-fee effect.
    position: Decimal,
    /// Average-cost bookkeeping for what's still open (fill units).
    open_qty: Decimal,
    open_cost: Decimal,
    // For the displayed averages.
    entry_qty: Decimal,
    entry_quote: Decimal,
    exit_qty: Decimal,
    exit_quote: Decimal,
    gross: Decimal,
    fees: Decimal,
    unconverted: BTreeMap<String, Decimal>,
    legs: Vec<Leg>,
}

impl Builder {
    fn new(direction: Direction, time: TimestampMs) -> Self {
        Self {
            direction,
            open_time: time,
            last_time: time,
            position: Decimal::ZERO,
            open_qty: Decimal::ZERO,
            open_cost: Decimal::ZERO,
            entry_qty: Decimal::ZERO,
            entry_quote: Decimal::ZERO,
            exit_qty: Decimal::ZERO,
            exit_quote: Decimal::ZERO,
            gross: Decimal::ZERO,
            fees: Decimal::ZERO,
            unconverted: BTreeMap::new(),
            legs: Vec::new(),
        }
    }

    fn sign(&self) -> Decimal {
        match self.direction {
            Direction::Long => Decimal::ONE,
            Direction::Short => Decimal::NEGATIVE_ONE,
        }
    }

    fn add_fee(&mut self, fee: Decimal, asset: &str, price: Decimal, base: &str, quote: &str) {
        if fee.is_zero() {
            return;
        }
        if asset == quote {
            self.fees += fee;
        } else if asset == base {
            self.fees += fee * price;
        } else {
            *self.unconverted.entry(asset.to_string()).or_default() += fee;
        }
    }

    fn entry(&mut self, price: Decimal, qty: Decimal, position_change: Decimal) {
        self.position += position_change;
        self.open_qty += qty;
        self.open_cost += price * qty;
        self.entry_qty += qty;
        self.entry_quote += price * qty;
    }

    fn exit(&mut self, price: Decimal, qty: Decimal, position_change: Decimal) {
        let avg_cost = self.open_cost / self.open_qty;
        self.gross += self.sign() * (price - avg_cost) * qty;
        self.position -= position_change;
        self.open_cost -= avg_cost * qty;
        self.open_qty -= qty;
        self.exit_qty += qty;
        self.exit_quote += price * qty;
    }

    fn finish(self, first: &Fill) -> GroupedTrade {
        let closed = self.position.is_zero();
        let trade = Trade {
            exchange: first.exchange.clone(),
            market: first.market,
            symbol: first.symbol.clone(),
            direction: self.direction,
            status: if closed {
                TradeStatus::Closed
            } else {
                TradeStatus::Open
            },
            open_time: self.open_time,
            close_time: closed.then_some(self.last_time),
            qty: self.entry_qty.normalize(),
            avg_entry: round(self.entry_quote / self.entry_qty),
            avg_exit: (!self.exit_qty.is_zero()).then(|| round(self.exit_quote / self.exit_qty)),
            leverage: None,
            gross_pnl: round(self.gross),
            fees: round(self.fees),
            funding: Decimal::ZERO,
            net_pnl: round(self.gross - self.fees),
            planned_entry: None,
            stop_loss: None,
            take_profit: None,
            planned_rr: None,
            r_multiple: None,
            notes: String::new(),
            playbook_id: None,
            is_manual: false,
        };
        GroupedTrade {
            trade,
            legs: self.legs,
            funding_ids: Vec::new(),
            unconverted_fees: self.unconverted,
        }
    }
}

/// Groups one instrument's fills into trades and attaches its funding.
pub fn group_instrument(
    fills: &[WithId<Fill>],
    funding: &[WithId<FundingEvent>],
) -> Result<GroupResult, GroupError> {
    let Some(first) = fills.first() else {
        return Ok(GroupResult {
            unassigned_funding: funding.iter().map(|f| f.id).collect(),
            ..GroupResult::default()
        });
    };
    let key = |f: &Fill| format!("{}/{}/{}", f.exchange, f.market.as_str(), f.symbol);
    let instrument = key(&first.item);
    if let Some(other) = fills.iter().find(|f| key(&f.item) != instrument) {
        return Err(GroupError::MixedInstruments(instrument, key(&other.item)));
    }

    // Time order; id keeps the file order for fills in the same second.
    let mut sorted: Vec<&WithId<Fill>> = fills.iter().collect();
    sorted.sort_by_key(|f| (f.item.time, f.id));

    let market = first.item.market;
    let (base, quote) = split_symbol(&first.item.symbol).unwrap_or(("", ""));
    let mut result = GroupResult::default();
    let mut current: Option<Builder> = None;

    for f in sorted {
        let fill = &f.item;
        let is_buy = fill.side == Side::Buy;
        // Spot: a fee paid in the base asset comes out of the coins you hold.
        let base_fee = if market == Market::Spot && fill.fee_asset == base {
            fill.fee
        } else {
            Decimal::ZERO
        };
        // How much this fill moves the position (always positive).
        let effective = if is_buy {
            fill.qty - base_fee
        } else {
            fill.qty + base_fee
        };

        let mut left_eff = effective;
        let mut left_qty = fill.qty;
        while left_eff > Decimal::ZERO {
            let builder = match current.as_mut() {
                Some(b) => b,
                None if market == Market::Spot && !is_buy => {
                    result.orphans.push(Orphan {
                        fill_id: f.id,
                        qty: left_qty.normalize(),
                    });
                    break;
                }
                None => {
                    let dir = if is_buy {
                        Direction::Long
                    } else {
                        Direction::Short
                    };
                    current.insert(Builder::new(dir, fill.time))
                }
            };

            let is_entry = (builder.direction == Direction::Long) == is_buy;
            let part_eff = if is_entry {
                left_eff
            } else {
                left_eff.min(builder.position)
            };
            // Same share of the fill's own quantity (differs only with base fees).
            let part_qty = if part_eff == left_eff {
                left_qty
            } else {
                left_qty * part_eff / left_eff
            };
            let part_fee = fill.fee * part_qty / fill.qty;

            if is_entry {
                builder.entry(fill.price, part_qty, part_eff);
            } else {
                builder.exit(fill.price, part_qty, part_eff);
            }
            builder.add_fee(part_fee, &fill.fee_asset, fill.price, base, quote);
            builder.last_time = fill.time;
            builder.legs.push(Leg {
                fill_id: f.id,
                qty: part_qty.normalize(),
                is_entry,
            });

            left_eff -= part_eff;
            left_qty -= part_qty;
            if builder.position.is_zero() {
                let done = current.take().expect("builder exists");
                result.trades.push(done.finish(fill));
            }
        }
    }
    if let Some(open) = current.take() {
        result.trades.push(open.finish(&first.item));
    }

    attach_funding(&mut result, funding);
    Ok(result)
}

/// Each funding event goes to the trade open at that moment (open and close
/// times inclusive), first match wins.
fn attach_funding(result: &mut GroupResult, funding: &[WithId<FundingEvent>]) {
    let mut events: Vec<&WithId<FundingEvent>> = funding.iter().collect();
    events.sort_by_key(|e| (e.item.time, e.id));
    for e in events {
        let t = e.item.time;
        let owner = result
            .trades
            .iter_mut()
            .find(|g| g.trade.open_time <= t && g.trade.close_time.is_none_or(|close| t <= close));
        match owner {
            Some(g) => {
                // Binance: negative = paid. Trade.funding: positive = paid.
                g.trade.funding -= e.item.amount;
                g.trade.net_pnl += e.item.amount;
                g.funding_ids.push(e.id);
            }
            None => result.unassigned_funding.push(e.id),
        }
    }
    for g in &mut result.trades {
        g.trade.funding = g.trade.funding.normalize();
        g.trade.net_pnl = round(g.trade.net_pnl);
    }
}

#[cfg(test)]
mod tests;
