//! Sanity checks for the fake CSV files in `test-data/`.
//!
//! The importers' tests depend on these files, so a broken fixture (wrong
//! header after an edit, a row with a missing column) should fail here with a
//! clear message instead of as a confusing parser error later.

use std::fs;
use std::path::PathBuf;

/// `test-data/` at the repo root (one level above `backend/`).
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("test-data")
}

/// Expected header per fixture. Keep in sync with docs/binance-csv.md.
const FIXTURES: &[(&str, &str, usize)] = &[
    (
        "binance-spot-trades.csv",
        "Date(UTC),Pair,Side,Price,Executed,Amount,Fee",
        8,
    ),
    (
        "binance-futures-trades.csv",
        "Date(UTC),Symbol,Side,Price,Quantity,Amount,Fee,Fee Coin,Realized Profit,Quote Asset",
        8,
    ),
    (
        "binance-futures-transactions.csv",
        "Date(UTC),Type,Amount,Asset,Symbol",
        9,
    ),
];

#[test]
fn fixtures_have_expected_header_and_row_count() {
    for (name, header, rows) in FIXTURES {
        let path = fixtures_dir().join(name);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let mut lines = text.lines().filter(|l| !l.trim().is_empty());

        assert_eq!(lines.next(), Some(*header), "{name}: header changed");
        let data: Vec<&str> = lines.collect();
        assert_eq!(data.len(), *rows, "{name}: row count changed");

        let columns = header.split(',').count();
        for (i, line) in data.iter().enumerate() {
            // Fixtures never contain quoted commas, so a plain split is exact.
            assert_eq!(
                line.split(',').count(),
                columns,
                "{name} line {}: wrong number of columns",
                i + 2
            );
        }
    }
}

#[test]
fn fixture_dates_are_utc_timestamps_in_2026() {
    // Fake data uses 2026 dates; a real export pasted in by mistake would
    // almost certainly have other dates and trip this check.
    for (name, _, _) in FIXTURES {
        let text = fs::read_to_string(fixtures_dir().join(name)).unwrap();
        for line in text.lines().skip(1).filter(|l| !l.trim().is_empty()) {
            let date = line.split(',').next().unwrap();
            assert!(
                date.len() == 19 && date.starts_with("2026-"),
                "{name}: unexpected date {date:?}"
            );
        }
    }
}
