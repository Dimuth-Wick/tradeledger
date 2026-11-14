//! Parsing helpers shared by all importers.

use std::collections::HashMap;
use std::str::FromStr;

use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};

use super::{ImportError, RowError};
use crate::models::{Side, TimestampMs};

/// "Date(UTC)" → "date(utc)", " Fee Coin" → "feecoin". Also drops a UTF-8
/// BOM that Excel puts at the start of the file.
pub fn normalize_header(h: &str) -> String {
    h.trim_start_matches('\u{feff}')
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Opens CSV bytes and reads the header line.
pub fn open(data: &[u8]) -> Result<(Header, csv::Reader<&[u8]>), ImportError> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(data);
    let header = Header(rdr.headers()?.iter().map(normalize_header).collect());
    Ok((header, rdr))
}

pub fn read_headers(data: &[u8]) -> Result<Vec<String>, ImportError> {
    Ok(open(data)?.0.0)
}

/// Column positions looked up by (normalized) header name.
pub struct Header(pub Vec<String>);

impl Header {
    pub fn has(&self, name: &str) -> bool {
        self.0.iter().any(|h| h == name)
    }

    /// Position of the first header matching any alias.
    pub fn find(&self, aliases: &[&str]) -> Option<usize> {
        aliases
            .iter()
            .find_map(|a| self.0.iter().position(|h| h == a))
    }

    /// Like `find`, but a missing column is an error naming the file type.
    pub fn require(
        &self,
        expected: &'static str,
        aliases: &[&'static str],
    ) -> Result<usize, ImportError> {
        self.find(aliases).ok_or(ImportError::MissingColumn {
            expected,
            column: aliases[0],
        })
    }
}

/// Calls `row` for every data record. Row-level problems (bad value, wrong
/// number of columns, invalid UTF-8) are collected with their line number
/// instead of stopping the import.
pub fn each_row(
    mut rdr: csv::Reader<&[u8]>,
    mut row: impl FnMut(&csv::StringRecord) -> Result<(), String>,
) -> Vec<RowError> {
    let mut errors = Vec::new();
    for result in rdr.records() {
        match result {
            Ok(record) => {
                if let Err(message) = row(&record) {
                    let line = record.position().map_or(0, |p| p.line());
                    errors.push(RowError { line, message });
                }
            }
            Err(e) => errors.push(RowError {
                line: e.position().map_or(0, |p| p.line()),
                message: e.to_string(),
            }),
        }
    }
    errors
}

/// Field text by position; empty string if the row is short.
pub fn field(record: &csv::StringRecord, idx: usize) -> &str {
    record.get(idx).unwrap_or("")
}

/// Exact decimal from text. Never goes through f64.
pub fn decimal(name: &str, text: &str) -> Result<Decimal, String> {
    if text.is_empty() {
        return Err(format!("{name} is empty"));
    }
    Decimal::from_str(text).map_err(|_| format!("{name} is not a number: {text:?}"))
}

/// "0.30000000ETH" → (0.3, "ETH"). Binance glues the asset onto the number.
pub fn amount_with_asset(name: &str, text: &str) -> Result<(Decimal, String), String> {
    let split = text
        .find(|c: char| c.is_ascii_alphabetic())
        .ok_or_else(|| format!("{name} has no asset: {text:?}"))?;
    let (number, asset) = text.split_at(split);
    Ok((decimal(name, number.trim())?, asset.trim().to_uppercase()))
}

/// "2026-01-05 09:15:02" (UTC) → ms since epoch.
pub fn utc_time(text: &str) -> Result<TimestampMs, String> {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S")
        .map(|t| t.and_utc().timestamp_millis())
        .map_err(|_| format!("time is not YYYY-MM-DD HH:MM:SS: {text:?}"))
}

pub fn side(text: &str) -> Result<Side, String> {
    match text.to_ascii_uppercase().as_str() {
        "BUY" => Ok(Side::Buy),
        "SELL" => Ok(Side::Sell),
        _ => Err(format!("side must be BUY or SELL: {text:?}")),
    }
}

/// Builds `dedupe_hash` values for one file.
///
/// The key is made of *parsed* values (normalized decimals), so "61000.0" and
/// "61000.00" give the same hash. Identical rows within one file get an
/// occurrence number (1, 2, …), so two genuinely identical fills are both
/// kept, while re-importing the file reproduces the same numbers.
pub struct Deduper {
    kind: &'static str,
    seen: HashMap<String, u32>,
}

impl Deduper {
    pub fn new(kind: &'static str) -> Self {
        Self {
            kind,
            seen: HashMap::new(),
        }
    }

    /// `parts` = the row's identifying values, in a fixed order per importer.
    pub fn hash(&mut self, parts: &[String]) -> String {
        let key = format!("{}|{}", self.kind, parts.join("|"));
        let occurrence = self.seen.entry(key.clone()).or_insert(0);
        *occurrence += 1;
        let digest = Sha256::digest(format!("{key}|#{occurrence}").as_bytes());
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// Canonical text for a decimal inside a dedupe key ("2000.250" → "2000.25").
pub fn key_dec(d: Decimal) -> String {
    d.normalize().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn header_normalization() {
        assert_eq!(normalize_header("\u{feff}Date(UTC)"), "date(utc)");
        assert_eq!(normalize_header(" Fee Coin "), "feecoin");
    }

    #[test]
    fn splits_number_and_asset() {
        assert_eq!(
            amount_with_asset("Executed", "0.30000000ETH"),
            Ok((dec!(0.3), "ETH".into()))
        );
        assert_eq!(
            amount_with_asset("Fee", "0.00012500 bnb"),
            Ok((dec!(0.000125), "BNB".into()))
        );
        assert!(amount_with_asset("Fee", "0.0001").is_err());
        assert!(amount_with_asset("Fee", "abcETH").is_err());
    }

    #[test]
    fn parses_utc_time_to_millis() {
        // 2026-01-05 09:15:02 UTC
        assert_eq!(utc_time("2026-01-05 09:15:02"), Ok(1_767_604_502_000));
        assert!(utc_time("05/01/2026 09:15").is_err());
    }

    #[test]
    fn decimals_are_exact_and_strict() {
        assert_eq!(decimal("Price", "0.00001234"), Ok(dec!(0.00001234)));
        assert!(decimal("Price", "").is_err());
        assert!(decimal("Price", "1,5").is_err());
    }

    #[test]
    fn dedupe_hash_is_stable_and_counts_identical_rows() {
        let parts = vec!["a".to_string(), key_dec(dec!(1.50))];
        let mut first_pass = Deduper::new("test");
        let h1 = first_pass.hash(&parts);
        let h2 = first_pass.hash(&parts); // identical twin row
        assert_ne!(h1, h2);
        assert_eq!(h1.len(), 64);

        let mut second_pass = Deduper::new("test");
        let same = vec!["a".to_string(), key_dec(dec!(1.5))];
        assert_eq!(second_pass.hash(&same), h1);
        assert_eq!(second_pass.hash(&same), h2);

        // Same values from a different file type must not collide.
        assert_ne!(Deduper::new("other").hash(&parts), h1);
    }
}
