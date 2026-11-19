//! The user's manual merge/split rules (see `grouping::Overrides`).

use rusqlite::{Connection, params};

use super::DbError;
use crate::features::grouping::Overrides;
use crate::models::{Market, OverrideKind};

/// Adds a rule, replacing any existing rule on the same fill.
pub fn set(
    conn: &Connection,
    exchange: &str,
    market: Market,
    symbol: &str,
    fill_hash: &str,
    kind: OverrideKind,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO grouping_overrides (exchange, market, symbol, fill_hash, kind)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (exchange, market, symbol, fill_hash) DO UPDATE SET kind = excluded.kind",
        params![exchange, market, symbol, fill_hash, kind],
    )?;
    Ok(())
}

pub fn load(
    conn: &Connection,
    exchange: &str,
    market: Market,
    symbol: &str,
) -> Result<Overrides, DbError> {
    let mut stmt = conn.prepare(
        "SELECT fill_hash, kind FROM grouping_overrides
         WHERE exchange = ?1 AND market = ?2 AND symbol = ?3",
    )?;
    let mut overrides = Overrides::default();
    let rows = stmt.query_map(params![exchange, market, symbol], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, OverrideKind>(1)?))
    })?;
    for row in rows {
        let (hash, kind) = row?;
        match kind {
            OverrideKind::Merge => overrides.merge_at.insert(hash),
            OverrideKind::Split => overrides.split_at.insert(hash),
        };
    }
    Ok(overrides)
}

/// Removes every rule of one instrument (back to automatic grouping).
pub fn clear(
    conn: &Connection,
    exchange: &str,
    market: Market,
    symbol: &str,
) -> Result<usize, DbError> {
    Ok(conn.execute(
        "DELETE FROM grouping_overrides WHERE exchange = ?1 AND market = ?2 AND symbol = ?3",
        params![exchange, market, symbol],
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::open_in_memory;

    #[test]
    fn set_replaces_and_load_splits_by_kind() {
        let conn = open_in_memory().unwrap();
        set(
            &conn,
            "binance",
            Market::Usdm,
            "BTCUSDT",
            "a",
            OverrideKind::Merge,
        )
        .unwrap();
        set(
            &conn,
            "binance",
            Market::Usdm,
            "BTCUSDT",
            "b",
            OverrideKind::Merge,
        )
        .unwrap();
        set(
            &conn,
            "binance",
            Market::Usdm,
            "BTCUSDT",
            "b",
            OverrideKind::Split,
        )
        .unwrap();
        set(
            &conn,
            "binance",
            Market::Spot,
            "BTCUSDT",
            "c",
            OverrideKind::Split,
        )
        .unwrap();

        let o = load(&conn, "binance", Market::Usdm, "BTCUSDT").unwrap();
        assert_eq!(o.merge_at.iter().collect::<Vec<_>>(), ["a"]);
        assert_eq!(o.split_at.iter().collect::<Vec<_>>(), ["b"]);

        assert_eq!(clear(&conn, "binance", Market::Usdm, "BTCUSDT").unwrap(), 2);
        assert_eq!(
            load(&conn, "binance", Market::Usdm, "BTCUSDT").unwrap(),
            Overrides::default()
        );
        assert_eq!(
            load(&conn, "binance", Market::Spot, "BTCUSDT")
                .unwrap()
                .split_at
                .len(),
            1
        );
    }
}
