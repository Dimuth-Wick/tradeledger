//! CSV export of trades, for spreadsheets or other tools.
//!
//! Same filters as the trade log, oldest first. Money stays exact (the
//! decimal text Rust stores). Times are UTC in ISO 8601.

use std::collections::BTreeMap;

use chrono::{DateTime, SecondsFormat};
use rusqlite::Connection;
use thiserror::Error;

use crate::db::{self, DbError};
use crate::models::TimestampMs;
use crate::trade_query::{self, TradeQuery};

const HEADERS: [&str; 24] = [
    "id",
    "exchange",
    "market",
    "symbol",
    "side",
    "status",
    "open_time_utc",
    "close_time_utc",
    "qty",
    "avg_entry",
    "avg_exit",
    "leverage",
    "gross_pnl",
    "fees",
    "funding",
    "net_pnl",
    "planned_entry",
    "stop_loss",
    "take_profit",
    "planned_rr",
    "r_multiple",
    "tags",
    "playbook",
    "notes",
];

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("could not write CSV: {0}")]
    Csv(#[from] csv::Error),
    #[error(transparent)]
    Db(#[from] DbError),
}

fn utc(ms: TimestampMs) -> String {
    DateTime::from_timestamp_millis(ms)
        .map(|t| t.to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_default()
}

/// Spreadsheets run cells starting with = + - @ as formulas; a note like
/// "=HYPERLINK(...)" must stay text. Only for free-text columns.
fn safe_text(s: &str) -> String {
    match s.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{s}"),
        _ => s.to_string(),
    }
}

fn opt<T: ToString>(v: Option<T>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

/// CSV text for the trades matching `q`.
pub fn trades_csv(conn: &Connection, q: &TradeQuery) -> Result<(String, u32), ExportError> {
    let mut trades = trade_query::filtered(conn, q)?;
    trades.sort_by_key(|t| (t.item.open_time, t.id));

    let tag_names: BTreeMap<i64, String> = db::tags::list(conn)?
        .into_iter()
        .flat_map(|g| {
            g.tags
                .into_iter()
                .map(move |t| (t.id, format!("{}: {}", g.name, t.name)))
        })
        .collect();
    let trade_tags = db::tags::all_trade_tags(conn)?;
    let playbooks: BTreeMap<i64, String> = db::playbooks::list(conn)?
        .into_iter()
        .map(|p| (p.id, p.name))
        .collect();

    let mut w = csv::Writer::from_writer(Vec::new());
    w.write_record(HEADERS)?;
    for t in &trades {
        let x = &t.item;
        let tags: Vec<&str> = trade_tags
            .get(&t.id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| tag_names.get(id))
                    .map(String::as_str)
                    .collect()
            })
            .unwrap_or_default();
        let playbook = x
            .playbook_id
            .and_then(|id| playbooks.get(&id))
            .cloned()
            .unwrap_or_default();
        w.write_record([
            t.id.to_string(),
            safe_text(&x.exchange),
            x.market.as_str().to_string(),
            safe_text(&x.symbol),
            x.direction.as_str().to_string(),
            x.status.as_str().to_string(),
            utc(x.open_time),
            x.close_time.map(utc).unwrap_or_default(),
            x.qty.to_string(),
            x.avg_entry.to_string(),
            opt(x.avg_exit),
            opt(x.leverage),
            x.gross_pnl.to_string(),
            x.fees.to_string(),
            x.funding.to_string(),
            x.net_pnl.to_string(),
            opt(x.planned_entry),
            opt(x.stop_loss),
            opt(x.take_profit),
            opt(x.planned_rr),
            opt(x.r_multiple),
            safe_text(&tags.join("; ")),
            safe_text(&playbook),
            safe_text(&x.notes),
        ])?;
    }
    let bytes = w
        .into_inner()
        .map_err(|e| ExportError::Csv(e.into_error().into()))?;
    let text = String::from_utf8(bytes).expect("csv writer outputs utf-8");
    Ok((text, trades.len() as u32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{open_in_memory, test_data};
    use crate::models::Trade;
    use rust_decimal_macros::dec;

    #[test]
    fn exports_trades_with_tags_playbook_and_safe_notes() {
        let conn = open_in_memory().unwrap();
        let later = Trade {
            open_time: 1_700_100_000_000,
            notes: "=HYPERLINK(\"http://x\")".into(),
            net_pnl: dec!(-4.5),
            ..test_data::trade()
        };
        let a = db::trades::insert(&conn, &later).unwrap();
        let b = db::trades::insert(&conn, &test_data::trade()).unwrap(); // opened earlier
        let g = db::tags::create_group(&conn, "Setup", "#396cd8").unwrap();
        let tag = db::tags::create_tag(&conn, g, "Breakout", "").unwrap();
        db::tags::set_for_trade(&conn, b, &[tag]).unwrap();
        let p = db::playbooks::insert(&conn, "Breakout retest", "").unwrap();
        db::playbooks::set_trade_playbook(&conn, b, Some(p)).unwrap();

        let (text, rows) = trades_csv(&conn, &TradeQuery::default()).unwrap();
        assert_eq!(rows, 2);
        let mut r = csv::Reader::from_reader(text.as_bytes());
        assert_eq!(r.headers().unwrap().iter().collect::<Vec<_>>(), HEADERS);
        let recs: Vec<csv::StringRecord> = r.records().map(Result::unwrap).collect();

        // Oldest first: b then a.
        assert_eq!(&recs[0][0], b.to_string());
        assert_eq!(&recs[0][3], "BTCUSDT");
        assert_eq!(&recs[0][6], "2023-11-14T22:13:20Z"); // 1_700_000_000_000 ms
        assert_eq!(&recs[0][21], "Setup: Breakout");
        assert_eq!(&recs[0][22], "Breakout retest");

        assert_eq!(&recs[1][0], a.to_string());
        assert_eq!(&recs[1][15], "-4.5"); // numbers are never escaped
        assert_eq!(&recs[1][23], "'=HYPERLINK(\"http://x\")"); // notes are
    }

    #[test]
    fn export_uses_the_trade_log_filters() {
        let conn = open_in_memory().unwrap();
        db::trades::insert(&conn, &test_data::trade()).unwrap();
        let q = TradeQuery {
            symbol: Some("ETH".into()),
            ..TradeQuery::default()
        };
        let (text, rows) = trades_csv(&conn, &q).unwrap();
        assert_eq!(rows, 0);
        assert_eq!(text.lines().count(), 1); // header only
    }
}
