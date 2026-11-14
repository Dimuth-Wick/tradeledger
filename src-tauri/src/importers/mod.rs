//! Exchange CSV importers.
//!
//! Flow:  CSV bytes ──parse()──► ParseReport (fills + row errors, nothing saved)
//!                     └──store_fills()──► DB (one import batch, duplicates skipped)
//!
//! Parsing and saving are separate so the UI can show a preview first (T4.5).
//! Each exchange/file type implements `ExchangeImporter`; adding Bybit later
//! means adding one more implementation, nothing else changes.

pub mod binance_spot;
mod common;

use rusqlite::Connection;
use serde::Serialize;
use thiserror::Error;

use crate::db::{self, DbError, InsertOutcome};
use crate::models::{Fill, FundingEvent, ImportBatch, Market, TimestampMs};

pub use binance_spot::BinanceSpot;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("this doesn't look like a {expected} file: column {column:?} is missing")]
    MissingColumn {
        expected: &'static str,
        column: &'static str,
    },
    #[error("no importer recognizes this file")]
    UnknownFormat,
    #[error("could not read CSV: {0}")]
    Csv(#[from] csv::Error),
    #[error(transparent)]
    Db(#[from] DbError),
}

impl From<rusqlite::Error> for ImportError {
    fn from(e: rusqlite::Error) -> Self {
        ImportError::Db(e.into())
    }
}

/// A row that couldn't be parsed. The rest of the file is still parsed so
/// the user sees every problem at once.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowError {
    /// Line number in the file (header = line 1).
    pub line: u64,
    pub message: String,
}

/// Everything read from one file. Nothing is saved yet.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseReport {
    pub fills: Vec<Fill>,
    pub funding: Vec<FundingEvent>,
    pub errors: Vec<RowError>,
    /// Valid rows deliberately ignored (e.g. transfers in a transaction file).
    pub skipped: u32,
}

/// One supported file type, e.g. "Binance spot trade history".
pub trait ExchangeImporter: Sync {
    /// Stable id, part of every dedupe hash, e.g. `"binance-spot"`.
    /// Never change it once released, or re-imports stop being detected.
    fn kind(&self) -> &'static str;
    /// Human name for the UI.
    fn label(&self) -> &'static str;
    fn exchange(&self) -> &'static str;
    fn market(&self) -> Market;
    /// True if a file with these (normalized) headers is this importer's type.
    fn matches_headers(&self, headers: &[String]) -> bool;
    fn parse(&self, data: &[u8]) -> Result<ParseReport, ImportError>;
}

/// Every importer the app knows, in detection order.
pub fn all() -> &'static [&'static dyn ExchangeImporter] {
    &[&BinanceSpot]
}

/// Picks the importer whose header pattern matches the file.
pub fn detect(data: &[u8]) -> Result<&'static dyn ExchangeImporter, ImportError> {
    let headers = common::read_headers(data)?;
    all()
        .iter()
        .copied()
        .find(|imp| imp.matches_headers(&headers))
        .ok_or(ImportError::UnknownFormat)
}

/// Result of saving a parsed file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub batch_id: i64,
    pub rows_total: u32,
    pub inserted: u32,
    pub duplicates: u32,
}

/// Saves fills as one import batch. All-or-nothing: if anything fails, the
/// transaction rolls back and the DB is unchanged. Rows already imported
/// (same dedupe hash) are counted as duplicates, not inserted again.
pub fn store_fills(
    conn: &Connection,
    importer: &dyn ExchangeImporter,
    file_name: &str,
    imported_at: TimestampMs,
    fills: &[Fill],
) -> Result<ImportSummary, ImportError> {
    let tx = conn.unchecked_transaction()?;
    let rows_total = u32::try_from(fills.len()).unwrap_or(u32::MAX);
    let batch_id = db::import_batches::insert(
        &tx,
        &ImportBatch {
            exchange: importer.exchange().to_string(),
            market: importer.market(),
            file_name: file_name.to_string(),
            imported_at,
            rows_total,
            rows_inserted: 0,
            rows_duplicate: 0,
        },
    )?;

    let (mut inserted, mut duplicates) = (0u32, 0u32);
    for fill in fills {
        let fill = Fill {
            import_batch_id: Some(batch_id),
            ..fill.clone()
        };
        match db::fills::insert(&tx, &fill)? {
            InsertOutcome::Inserted(_) => inserted += 1,
            InsertOutcome::Duplicate => duplicates += 1,
        }
    }
    db::import_batches::set_counts(&tx, batch_id, inserted, duplicates)?;
    tx.commit()?;

    Ok(ImportSummary {
        batch_id,
        rows_total,
        inserted,
        duplicates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    const SPOT: &[u8] = include_bytes!("../../../tests/fixtures/binance-spot-trades.csv");
    const FUTURES: &[u8] = include_bytes!("../../../tests/fixtures/binance-futures-trades.csv");

    #[test]
    fn detects_spot_file_and_rejects_others() {
        assert_eq!(detect(SPOT).unwrap().kind(), "binance-spot");
        assert!(matches!(detect(FUTURES), Err(ImportError::UnknownFormat)));
        assert!(matches!(
            detect(b"foo,bar\n1,2\n"),
            Err(ImportError::UnknownFormat)
        ));
    }

    #[test]
    fn importing_the_same_file_twice_adds_nothing_the_second_time() {
        let conn = open_in_memory().unwrap();
        let report = BinanceSpot.parse(SPOT).unwrap();

        let first = store_fills(&conn, &BinanceSpot, "spot.csv", 1, &report.fills).unwrap();
        assert_eq!(
            (first.rows_total, first.inserted, first.duplicates),
            (8, 8, 0)
        );

        // Parse again from scratch, as a real re-import would.
        let again = BinanceSpot.parse(SPOT).unwrap();
        let second = store_fills(&conn, &BinanceSpot, "spot.csv", 2, &again.fills).unwrap();
        assert_eq!(
            (second.rows_total, second.inserted, second.duplicates),
            (8, 0, 8)
        );

        let batch = db::import_batches::get(&conn, second.batch_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            (batch.item.rows_inserted, batch.item.rows_duplicate),
            (0, 8)
        );

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM fills", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            count, 8,
            "identical twin rows are both kept, nothing doubled"
        );
    }

    #[test]
    fn stored_fills_point_to_their_batch() {
        let conn = open_in_memory().unwrap();
        let report = BinanceSpot.parse(SPOT).unwrap();
        let summary = store_fills(&conn, &BinanceSpot, "spot.csv", 1, &report.fills).unwrap();
        let fills =
            db::fills::list_for_instrument(&conn, "binance", Market::Spot, "ETHUSDT").unwrap();
        assert_eq!(fills.len(), 3);
        assert!(
            fills
                .iter()
                .all(|f| f.item.import_batch_id == Some(summary.batch_id))
        );
    }

    #[test]
    fn failed_store_leaves_db_unchanged() {
        let conn = open_in_memory().unwrap();
        let mut fills = BinanceSpot.parse(SPOT).unwrap().fills;
        // Points at a trade that doesn't exist → foreign key error mid-way.
        fills[3].trade_id = Some(999);
        assert!(store_fills(&conn, &BinanceSpot, "spot.csv", 1, &fills).is_err());

        let (fill_count, batch_count): (i64, i64) = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM fills), (SELECT COUNT(*) FROM import_batches)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((fill_count, batch_count), (0, 0));
    }
}
