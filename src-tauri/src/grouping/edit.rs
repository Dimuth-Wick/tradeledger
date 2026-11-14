//! Manual fixes to automatic grouping: merge, split, reset.
//!
//! Each action saves a rule (`grouping_overrides`) and regroups the
//! instrument in the same transaction, so the change survives every future
//! import and either fully happens or not at all.

use rusqlite::{Connection, params};
use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;

use super::persist::{Instrument, RegroupSummary, regroup_in_tx};
use crate::db::{self, DbError};
use crate::models::{OverrideKind, Side, TimestampMs, Trade, WithId};

#[derive(Debug, Error)]
pub enum EditError {
    #[error("trade {0} not found")]
    NotFound(i64),
    #[error("manual trades aren't built from fills, so they can't be merged or split")]
    Manual,
    #[error("there is no earlier trade on this coin to merge with")]
    NoPrevious,
    #[error(
        "only trades in the same direction can be merged (this is {0}, the previous one is {1})"
    )]
    DirectionMismatch(&'static str, &'static str),
    #[error("that fill doesn't belong to this trade")]
    FillNotInTrade,
    #[error("a trade can't be split at its first fill")]
    FirstFill,
    #[error(transparent)]
    Db(#[from] DbError),
}

impl From<rusqlite::Error> for EditError {
    fn from(e: rusqlite::Error) -> Self {
        EditError::Db(e.into())
    }
}

/// One fill as used by a trade (for the "Fills" panel and "Split here").
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeFill {
    pub fill_id: i64,
    pub time: TimestampMs,
    pub side: Side,
    pub price: Decimal,
    /// The fill's full quantity.
    pub fill_qty: Decimal,
    /// The part used by this trade (smaller for a flip fill).
    pub qty: Decimal,
    pub is_entry: bool,
    pub fee: Decimal,
    pub fee_asset: String,
}

fn instrument_of(t: &Trade) -> Instrument {
    Instrument {
        exchange: t.exchange.clone(),
        market: t.market,
        symbol: t.symbol.clone(),
    }
}

/// An imported trade (merge/split/reset only make sense for those).
fn imported(conn: &Connection, id: i64) -> Result<WithId<Trade>, EditError> {
    let t = db::trades::get(conn, id)?.ok_or(EditError::NotFound(id))?;
    if t.item.is_manual {
        return Err(EditError::Manual);
    }
    Ok(t)
}

fn group_key(conn: &Connection, id: i64) -> Result<Option<String>, EditError> {
    Ok(
        conn.query_row("SELECT group_key FROM trades WHERE id = ?1", [id], |r| {
            r.get(0)
        })?,
    )
}

pub fn trade_fills(conn: &Connection, trade_id: i64) -> Result<Vec<TradeFill>, EditError> {
    db::trades::get(conn, trade_id)?.ok_or(EditError::NotFound(trade_id))?;
    let fills = db::fills::list_for_trade(conn, trade_id)?;
    let allocs = db::allocations::list_for_trade(conn, trade_id)?;
    Ok(fills
        .into_iter()
        .zip(allocs)
        .map(|(f, a)| TradeFill {
            fill_id: f.id,
            time: f.item.time,
            side: f.item.side,
            price: f.item.price,
            // Normalized like `qty` ("0.010" → "0.01"), so the UI can tell a
            // partly used fill by simple comparison.
            fill_qty: f.item.qty.normalize(),
            qty: a.qty.normalize(),
            is_entry: a.is_entry,
            fee: f.item.fee,
            fee_asset: f.item.fee_asset,
        })
        .collect())
}

/// Joins a trade to the trade just before it (same coin and direction).
/// The later trade's journal moves to the earlier one: notes are appended,
/// its plan/playbook/leverage fill any gaps, tags and screenshots move.
pub fn merge_with_previous(conn: &Connection, trade_id: i64) -> Result<RegroupSummary, EditError> {
    let tx = conn.unchecked_transaction()?;
    let later = imported(&tx, trade_id)?;
    let inst = instrument_of(&later.item);
    let key = group_key(&tx, trade_id)?.ok_or(EditError::NoPrevious)?;

    let all =
        db::trades::list_imported_for_instrument(&tx, &inst.exchange, inst.market, &inst.symbol)?;
    let pos = all
        .iter()
        .position(|(t, _)| t.id == trade_id)
        .ok_or(EditError::NotFound(trade_id))?;
    if pos == 0 {
        return Err(EditError::NoPrevious);
    }
    let earlier = &all[pos - 1].0;
    if earlier.item.direction != later.item.direction {
        return Err(EditError::DirectionMismatch(
            later.item.direction.as_str(),
            earlier.item.direction.as_str(),
        ));
    }

    move_journal(&tx, earlier, &later)?;
    db::overrides::set(
        &tx,
        &inst.exchange,
        inst.market,
        &inst.symbol,
        &key,
        OverrideKind::Merge,
    )?;
    let summary = regroup_in_tx(&tx, &inst)?;
    tx.commit()?;
    Ok(summary)
}

fn move_journal(
    tx: &Connection,
    into: &WithId<Trade>,
    from: &WithId<Trade>,
) -> Result<(), EditError> {
    let (a, b) = (&into.item, &from.item);
    let notes = match (a.notes.trim().is_empty(), b.notes.trim().is_empty()) {
        (_, true) => a.notes.clone(),
        (true, false) => b.notes.clone(),
        (false, false) => format!("{}\n\n{}", a.notes, b.notes),
    };
    // Take the whole plan from one trade: mixing one trade's stop with the
    // other's target would make a plan nobody made.
    let has_plan =
        |t: &Trade| t.planned_entry.is_some() || t.stop_loss.is_some() || t.take_profit.is_some();
    let plan = if has_plan(a) { a } else { b };
    let merged = Trade {
        notes,
        planned_entry: plan.planned_entry,
        stop_loss: plan.stop_loss,
        take_profit: plan.take_profit,
        playbook_id: a.playbook_id.or(b.playbook_id),
        leverage: a.leverage.or(b.leverage),
        ..a.clone()
    };
    db::trades::update(tx, into.id, &merged)?;

    // Re-point everything hanging off the later trade. OR IGNORE skips links
    // the earlier trade already has (same tag twice); those leftovers are
    // removed when the later trade is deleted by the regroup.
    for sql in [
        "UPDATE OR IGNORE trade_tags SET trade_id = ?1 WHERE trade_id = ?2",
        "UPDATE screenshots SET trade_id = ?1 WHERE trade_id = ?2",
        "UPDATE OR IGNORE custom_values SET trade_id = ?1 WHERE trade_id = ?2",
        "UPDATE OR IGNORE playbook_checks SET trade_id = ?1 WHERE trade_id = ?2",
    ] {
        tx.execute(sql, params![into.id, from.id])?;
    }
    Ok(())
}

/// Starts a new trade at `fill_id`. Any position still open at that moment
/// carries over to the new trade at its average cost. Also undoes a merge.
pub fn split_at_fill(
    conn: &Connection,
    trade_id: i64,
    fill_id: i64,
) -> Result<RegroupSummary, EditError> {
    let tx = conn.unchecked_transaction()?;
    let trade = imported(&tx, trade_id)?;
    let fills = db::fills::list_for_trade(&tx, trade_id)?;
    let pos = fills
        .iter()
        .position(|f| f.id == fill_id)
        .ok_or(EditError::FillNotInTrade)?;
    if pos == 0 {
        return Err(EditError::FirstFill);
    }
    let inst = instrument_of(&trade.item);
    let hash = &fills[pos].item.dedupe_hash;
    db::overrides::set(
        &tx,
        &inst.exchange,
        inst.market,
        &inst.symbol,
        hash,
        OverrideKind::Split,
    )?;
    let summary = regroup_in_tx(&tx, &inst)?;
    tx.commit()?;
    Ok(summary)
}

/// Removes every manual rule on this trade's coin and regroups automatically.
pub fn reset_grouping(conn: &Connection, trade_id: i64) -> Result<RegroupSummary, EditError> {
    let tx = conn.unchecked_transaction()?;
    let inst = instrument_of(&imported(&tx, trade_id)?.item);
    db::overrides::clear(&tx, &inst.exchange, inst.market, &inst.symbol)?;
    let summary = regroup_in_tx(&tx, &inst)?;
    tx.commit()?;
    Ok(summary)
}

#[cfg(test)]
mod tests;
