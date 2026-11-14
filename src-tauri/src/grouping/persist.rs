//! Saving grouping results to the DB without losing the user's journal data.
//!
//! Regrouping happens after every import, so the same trade is rebuilt many
//! times. To keep its notes, risk plan, playbook and tags (which hang off the
//! trade's id), each imported trade has a `group_key`: the dedupe hash of the
//! fill that opened it. A rebuilt trade is matched to an existing one by:
//!   1. same opening fill (same group_key), else
//!   2. the existing trade's opening fill is one of the new trade's fills
//!      (e.g. an older fill was imported later and now opens the trade).
//!
//! Matched → updated in place (same id, journal kept, numbers recomputed).
//! Unmatched new → inserted. Unmatched old → deleted (its fills are gone or
//! now belong to a trade that already claimed another match).
//! Manual trades are never touched.

use std::collections::{BTreeSet, HashMap};

use rusqlite::Connection;
use serde::Serialize;

use super::group_instrument;
use crate::db::{self, DbError};
use crate::importers::ParseReport;
use crate::models::{Market, Trade, TradeStatus};
use crate::risk;

/// One exchange + market + symbol: the unit the grouping engine works on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instrument {
    pub exchange: String,
    pub market: Market,
    pub symbol: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegroupSummary {
    pub trades_created: u32,
    pub trades_updated: u32,
    pub trades_removed: u32,
    /// Spot sell quantity with no matching buy (see `grouping::Orphan`).
    pub orphan_fills: u32,
    pub unassigned_funding: u32,
}

impl std::ops::AddAssign for RegroupSummary {
    fn add_assign(&mut self, o: Self) {
        self.trades_created += o.trades_created;
        self.trades_updated += o.trades_updated;
        self.trades_removed += o.trades_removed;
        self.orphan_fills += o.orphan_fills;
        self.unassigned_funding += o.unassigned_funding;
    }
}

/// Instruments affected by a parsed file, i.e. the ones to regroup after saving it.
pub fn touched_instruments(report: &ParseReport) -> BTreeSet<Instrument> {
    let fills = report.fills.iter().map(|f| Instrument {
        exchange: f.exchange.clone(),
        market: f.market,
        symbol: f.symbol.clone(),
    });
    // Funding only exists on futures.
    let funding = report.funding.iter().map(|e| Instrument {
        exchange: e.exchange.clone(),
        market: Market::Usdm,
        symbol: e.symbol.clone(),
    });
    fills.chain(funding).collect()
}

/// Keeps what the user typed on the old version of a trade; the numbers come
/// from the fresh grouping. R values are recomputed because the averages may
/// have changed; a plan that no longer makes sense (stop now on the wrong
/// side) gives no R instead of failing the whole import.
pub fn merge_journal(computed: Trade, existing: &Trade) -> Trade {
    let mut t = Trade {
        planned_entry: existing.planned_entry,
        stop_loss: existing.stop_loss,
        take_profit: existing.take_profit,
        notes: existing.notes.clone(),
        playbook_id: existing.playbook_id,
        // Exports don't contain leverage, so only the user can have set it.
        leverage: existing.leverage.or(computed.leverage),
        ..computed
    };
    let plan_entry = t.planned_entry.unwrap_or(t.avg_entry);
    t.planned_rr = match (t.stop_loss, t.take_profit) {
        (Some(stop), Some(target)) => risk::planned_rr(t.direction, plan_entry, stop, target).ok(),
        _ => None,
    };
    t.r_multiple = match (t.stop_loss, t.avg_exit, t.status) {
        (Some(stop), Some(exit), TradeStatus::Closed) => {
            risk::r_multiple(t.direction, plan_entry, stop, t.avg_entry, exit).ok()
        }
        _ => None,
    };
    t
}

/// Rebuilds one instrument's imported trades from its fills and funding.
/// All-or-nothing (one transaction).
pub fn regroup_instrument(conn: &Connection, inst: &Instrument) -> Result<RegroupSummary, DbError> {
    let (exchange, market, symbol) = (inst.exchange.as_str(), inst.market, inst.symbol.as_str());
    let tx = conn.unchecked_transaction()?;

    let fills = db::fills::list_for_instrument(&tx, exchange, market, symbol)?;
    let funding = match market {
        Market::Usdm => db::funding::list_for_symbol(&tx, exchange, symbol)?,
        Market::Spot => Vec::new(),
    };
    let result =
        group_instrument(&fills, &funding).expect("fills were queried for a single instrument");
    let hash_of: HashMap<i64, &str> = fills
        .iter()
        .map(|f| (f.id, f.item.dedupe_hash.as_str()))
        .collect();

    let existing = db::trades::list_imported_for_instrument(&tx, exchange, market, symbol)?;
    let by_key: HashMap<&str, usize> = existing
        .iter()
        .enumerate()
        .filter_map(|(i, (_, key))| key.as_deref().map(|k| (k, i)))
        .collect();

    // New trade i → index of the existing trade it continues (if any).
    let keys: Vec<&str> = result
        .trades
        .iter()
        .map(|g| hash_of[&g.legs[0].fill_id])
        .collect();
    let mut matched: Vec<Option<usize>> = keys.iter().map(|k| by_key.get(k).copied()).collect();
    let mut claimed = vec![false; existing.len()];
    for &e in matched.iter().flatten() {
        claimed[e] = true;
    }
    for (i, g) in result.trades.iter().enumerate() {
        if matched[i].is_some() {
            continue;
        }
        matched[i] = g.legs.iter().find_map(|leg| {
            by_key
                .get(hash_of[&leg.fill_id])
                .copied()
                .filter(|&e| !claimed[e])
        });
        if let Some(e) = matched[i] {
            claimed[e] = true;
        }
    }

    let mut summary = RegroupSummary {
        orphan_fills: result.orphans.len() as u32,
        unassigned_funding: result.unassigned_funding.len() as u32,
        ..RegroupSummary::default()
    };

    // Free the keys first: a key can move from one trade to another, and the
    // UNIQUE index would reject the intermediate state.
    for (i, (trade, _)) in existing.iter().enumerate() {
        if claimed[i] {
            db::trades::set_group_key(&tx, trade.id, None)?;
        } else {
            db::trades::delete(&tx, trade.id)?;
            summary.trades_removed += 1;
        }
    }
    db::allocations::delete_for_instrument(&tx, exchange, market, symbol)?;
    if market == Market::Usdm {
        db::funding::clear_trade_links(&tx, exchange, symbol)?;
    }

    for (i, g) in result.trades.iter().enumerate() {
        let id = match matched[i] {
            Some(e) => {
                let old = &existing[e].0;
                db::trades::update(&tx, old.id, &merge_journal(g.trade.clone(), &old.item))?;
                summary.trades_updated += 1;
                old.id
            }
            None => {
                summary.trades_created += 1;
                db::trades::insert(&tx, &g.trade)?
            }
        };
        db::trades::set_group_key(&tx, id, Some(keys[i]))?;
        for leg in &g.legs {
            db::allocations::insert(&tx, leg.fill_id, id, leg.qty, leg.is_entry)?;
        }
        for &funding_id in &g.funding_ids {
            db::funding::set_trade(&tx, funding_id, id)?;
        }
    }

    tx.commit()?;
    Ok(summary)
}

/// Regroups several instruments (e.g. everything touched by one import).
pub fn regroup_all<'a>(
    conn: &Connection,
    instruments: impl IntoIterator<Item = &'a Instrument>,
) -> Result<RegroupSummary, DbError> {
    let mut total = RegroupSummary::default();
    for inst in instruments {
        total += regroup_instrument(conn, inst)?;
    }
    Ok(total)
}

#[cfg(test)]
mod tests;
