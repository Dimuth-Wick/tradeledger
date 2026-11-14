//! The two steps the import screen calls:
//!
//! 1. `preview` — detect the file type, parse it, and show what importing
//!    would do (new vs. already-imported rows, row errors, the trades each
//!    instrument would end up with, unmatched sells, unconvertible fees).
//!    Nothing is written.
//! 2. `confirm` — parse again, save, and regroup the touched instruments.
//!
//! The file is parsed twice instead of keeping state between the calls, so a
//! confirm always saves exactly what's in the file, and an abandoned preview
//! leaves nothing behind.

use std::collections::BTreeMap;

use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;

use crate::db::{self, DbError};
use crate::grouping::persist::{self, Instrument, RegroupSummary};
use crate::grouping::{self, GroupError};
use crate::importers::{self, ExchangeImporter, ImportError, ImportSummary, ParseReport, RowError};
use crate::models::{Market, TimestampMs, TradeStatus, WithId};

#[derive(Debug, Error)]
pub enum FlowError {
    #[error("{0}")]
    Import(#[from] ImportError),
    #[error(transparent)]
    Db(#[from] DbError),
    #[error(transparent)]
    Group(#[from] GroupError),
}

/// What one instrument would look like after the import.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentPreview {
    pub exchange: String,
    pub market: Market,
    pub symbol: String,
    /// Rows in this file for the instrument that aren't stored yet.
    pub new_rows: u32,
    pub trades_after: u32,
    pub open_after: u32,
    /// Sells with no matching buy (bought before the export's date range).
    pub orphan_fills: u32,
    pub unassigned_funding: u32,
    /// Fees that can't be converted to the quote asset, summed per asset.
    pub unconverted_fees: BTreeMap<String, Decimal>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub kind: String,
    pub label: String,
    pub file_name: String,
    pub fills: u32,
    pub funding: u32,
    pub skipped: u32,
    pub already_imported: u32,
    pub errors: Vec<RowError>,
    pub instruments: Vec<InstrumentPreview>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub import: ImportSummary,
    pub regroup: RegroupSummary,
}

/// Detects and parses. An unknown file lists what is supported.
fn parse(content: &str) -> Result<(&'static dyn ExchangeImporter, ParseReport), ImportError> {
    let data = content.as_bytes();
    let importer = importers::detect(data)?;
    Ok((importer, importer.parse(data)?))
}

/// Friendlier text for "unknown format": name the supported files.
pub fn unknown_format_message() -> String {
    let names: Vec<&str> = importers::all().iter().map(|i| i.label()).collect();
    format!(
        "This file type isn't recognized. Supported: {}.",
        names.join(", ")
    )
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

pub fn preview(
    conn: &Connection,
    file_name: &str,
    content: &str,
) -> Result<ImportPreview, FlowError> {
    let (importer, report) = parse(content)?;

    // Split the file's rows into new vs. already stored.
    let mut new_fills = Vec::new();
    let mut new_funding = Vec::new();
    let mut already = 0usize;
    for f in &report.fills {
        if db::fills::hash_exists(conn, &f.dedupe_hash)? {
            already += 1;
        } else {
            new_fills.push(f.clone());
        }
    }
    for e in &report.funding {
        if db::funding::hash_exists(conn, &e.dedupe_hash)? {
            already += 1;
        } else {
            new_funding.push(e.clone());
        }
    }

    let mut instruments = Vec::new();
    for inst in persist::touched_instruments(&report) {
        instruments.push(preview_instrument(conn, &inst, &new_fills, &new_funding)?);
    }

    Ok(ImportPreview {
        kind: importer.kind().to_string(),
        label: importer.label().to_string(),
        file_name: file_name.to_string(),
        fills: count(report.fills.len()),
        funding: count(report.funding.len()),
        skipped: report.skipped,
        already_imported: count(already),
        errors: report.errors,
        instruments,
    })
}

/// Adds not-yet-stored rows with temporary ids after the stored ones, so
/// ties in time keep the file's row order. Returns how many were added.
fn append_with_temp_ids<'a, T: Clone + 'a>(
    rows: &mut Vec<WithId<T>>,
    new: impl Iterator<Item = &'a T>,
) -> usize {
    let first = rows.iter().map(|r| r.id).max().unwrap_or(0) + 1;
    let before = rows.len();
    rows.extend((first..).zip(new).map(|(id, item)| WithId {
        id,
        item: item.clone(),
    }));
    rows.len() - before
}

/// Groups (stored rows + this file's new rows) in memory to show the outcome.
fn preview_instrument(
    conn: &Connection,
    inst: &Instrument,
    new_fills: &[crate::models::Fill],
    new_funding: &[crate::models::FundingEvent],
) -> Result<InstrumentPreview, FlowError> {
    let (exchange, market, symbol) = (inst.exchange.as_str(), inst.market, inst.symbol.as_str());
    let mut fills = db::fills::list_for_instrument(conn, exchange, market, symbol)?;
    let mut funding = match market {
        Market::Usdm => db::funding::list_for_symbol(conn, exchange, symbol)?,
        Market::Spot => Vec::new(),
    };

    let mut new_rows = append_with_temp_ids(
        &mut fills,
        new_fills
            .iter()
            .filter(|f| f.exchange == exchange && f.market == market && f.symbol == symbol),
    );
    if market == Market::Usdm {
        new_rows += append_with_temp_ids(
            &mut funding,
            new_funding
                .iter()
                .filter(|e| e.exchange == exchange && e.symbol == symbol),
        );
    }

    // Same rules as the real regroup, so the preview matches the outcome.
    let overrides = db::overrides::load(conn, exchange, market, symbol)?;
    let result = grouping::group_instrument_with(&fills, &funding, &overrides)?;
    let mut unconverted_fees = BTreeMap::new();
    for g in &result.trades {
        for (asset, amount) in &g.unconverted_fees {
            *unconverted_fees
                .entry(asset.clone())
                .or_insert(Decimal::ZERO) += amount;
        }
    }
    Ok(InstrumentPreview {
        exchange: inst.exchange.clone(),
        market,
        symbol: inst.symbol.clone(),
        new_rows: count(new_rows),
        trades_after: count(result.trades.len()),
        open_after: count(
            result
                .trades
                .iter()
                .filter(|g| g.trade.status == TradeStatus::Open)
                .count(),
        ),
        orphan_fills: count(result.orphans.len()),
        unassigned_funding: count(result.unassigned_funding.len()),
        unconverted_fees,
    })
}

/// Saves the file's valid rows and regroups every instrument it touched.
/// Rows with errors are left out (the preview listed them).
pub fn confirm(
    conn: &Connection,
    file_name: &str,
    content: &str,
    imported_at: TimestampMs,
) -> Result<ImportResult, FlowError> {
    let (importer, report) = parse(content)?;
    let import = importers::store(conn, importer, file_name, imported_at, &report)?;
    let regroup = persist::regroup_all(conn, &persist::touched_instruments(&report))?;
    Ok(ImportResult { import, regroup })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use rust_decimal_macros::dec;

    const SPOT: &str = include_str!("../../tests/fixtures/binance-spot-trades.csv");
    const FUTURES: &str = include_str!("../../tests/fixtures/binance-futures-trades.csv");
    const TX: &str = include_str!("../../tests/fixtures/binance-futures-transactions.csv");

    fn inst<'a>(p: &'a ImportPreview, symbol: &str) -> &'a InstrumentPreview {
        p.instruments.iter().find(|i| i.symbol == symbol).unwrap()
    }

    #[test]
    fn preview_of_spot_file_on_empty_db() {
        let conn = open_in_memory().unwrap();
        let p = preview(&conn, "spot.csv", SPOT).unwrap();
        assert_eq!(p.kind, "binance-spot");
        assert_eq!((p.fills, p.funding, p.already_imported), (8, 0, 0));
        assert!(p.errors.is_empty());
        assert_eq!(p.instruments.len(), 3);

        let eth = inst(&p, "ETHUSDT");
        assert_eq!((eth.new_rows, eth.trades_after, eth.open_after), (3, 1, 0));
        let sol = inst(&p, "SOLUSDT");
        assert_eq!(sol.unconverted_fees.get("BNB"), Some(&dec!(0.0006125)));
        assert_eq!(inst(&p, "BTCUSDT").open_after, 1);
    }

    #[test]
    fn preview_writes_nothing() {
        let conn = open_in_memory().unwrap();
        preview(&conn, "spot.csv", SPOT).unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM fills) + (SELECT COUNT(*) FROM trades)
                      + (SELECT COUNT(*) FROM import_batches)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn confirm_saves_and_groups_then_reimport_adds_nothing() {
        let conn = open_in_memory().unwrap();
        let r = confirm(&conn, "spot.csv", SPOT, 1).unwrap();
        assert_eq!((r.import.inserted, r.import.duplicates), (8, 0));
        assert_eq!(r.regroup.trades_created, 3);

        let p = preview(&conn, "spot.csv", SPOT).unwrap();
        assert_eq!(p.already_imported, 8);
        assert_eq!(inst(&p, "ETHUSDT").new_rows, 0);
        assert_eq!(
            inst(&p, "ETHUSDT").trades_after,
            1,
            "same trade, not doubled"
        );

        let again = confirm(&conn, "spot.csv", SPOT, 2).unwrap();
        assert_eq!((again.import.inserted, again.import.duplicates), (0, 8));
        assert_eq!(again.regroup.trades_created, 0);
    }

    #[test]
    fn funding_preview_counts_unassigned_until_trades_exist() {
        let conn = open_in_memory().unwrap();
        let before = preview(&conn, "tx.csv", TX).unwrap();
        assert_eq!(before.funding, 6);
        assert_eq!(before.skipped, 3);
        assert_eq!(inst(&before, "BTCUSDT").unassigned_funding, 3);

        confirm(&conn, "futures.csv", FUTURES, 1).unwrap();
        let after = preview(&conn, "tx.csv", TX).unwrap();
        assert_eq!(inst(&after, "BTCUSDT").unassigned_funding, 0);
        assert_eq!(inst(&after, "ETHUSDT").trades_after, 2);
    }

    #[test]
    fn preview_shows_orphan_sells_and_row_errors() {
        let conn = open_in_memory().unwrap();
        let csv = "Date(UTC),Pair,Side,Price,Executed,Amount,Fee\n\
            2026-03-01 10:00:00,ADAUSDT,SELL,0.50,100ADA,50USDT,0.05USDT\n\
            2026-03-01 10:00:01,ADAUSDT,SELL,abc,100ADA,50USDT,0.05USDT\n";
        let p = preview(&conn, "ada.csv", csv).unwrap();
        assert_eq!(p.fills, 1);
        assert_eq!(p.errors.len(), 1);
        assert_eq!(p.errors[0].line, 3);
        let ada = inst(&p, "ADAUSDT");
        assert_eq!((ada.orphan_fills, ada.trades_after), (1, 0));
    }

    #[test]
    fn unknown_file_is_an_error() {
        let conn = open_in_memory().unwrap();
        let err = preview(&conn, "x.csv", "a,b\n1,2\n").unwrap_err();
        assert!(matches!(err, FlowError::Import(ImportError::UnknownFormat)));
        assert!(unknown_format_message().contains("Binance spot trade history"));
    }
}
