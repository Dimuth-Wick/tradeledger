//! Exchange executions. Inserts skip rows whose `dedupe_hash` already exists,
//! so importing the same CSV twice adds nothing.
//!
//! Which trade(s) a fill belongs to is stored in `fill_allocations` (see
//! `db::allocations`), because a flip fill belongs to two trades.

use rusqlite::{Connection, Row, params};

use super::sql::{SqlDec, dec, opt_dec};
use super::{DbError, InsertOutcome};
use crate::models::{Fill, Market, WithId};

pub fn insert(conn: &Connection, f: &Fill) -> Result<InsertOutcome, DbError> {
    let changed = conn.execute(
        "INSERT INTO fills
            (exchange, market, symbol, side, qty, price, fee, fee_asset,
             realized_pnl, time, dedupe_hash, import_batch_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(dedupe_hash) DO NOTHING",
        params![
            f.exchange,
            f.market,
            f.symbol,
            f.side,
            SqlDec(f.qty),
            SqlDec(f.price),
            SqlDec(f.fee),
            f.fee_asset,
            f.realized_pnl.map(SqlDec),
            f.time,
            f.dedupe_hash,
            f.import_batch_id
        ],
    )?;
    // 0 rows changed = the ON CONFLICT clause skipped it.
    Ok(if changed == 0 {
        InsertOutcome::Duplicate
    } else {
        InsertOutcome::Inserted(conn.last_insert_rowid())
    })
}

/// All fills of one instrument in time order: the input of the grouping engine.
/// `id` breaks ties so fills with the same timestamp keep import order.
pub fn list_for_instrument(
    conn: &Connection,
    exchange: &str,
    market: Market,
    symbol: &str,
) -> Result<Vec<WithId<Fill>>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM fills
         WHERE exchange = ?1 AND market = ?2 AND symbol = ?3
         ORDER BY time, id",
    )?;
    let rows = stmt.query_map(params![exchange, market, symbol], from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Fills used by a trade (a flip fill shows up in both of its trades).
pub fn list_for_trade(conn: &Connection, trade_id: i64) -> Result<Vec<WithId<Fill>>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT f.* FROM fills f
         JOIN fill_allocations a ON a.fill_id = f.id
         WHERE a.trade_id = ?1
         ORDER BY f.time, f.id",
    )?;
    let rows = stmt.query_map([trade_id], from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn from_row(r: &Row<'_>) -> rusqlite::Result<WithId<Fill>> {
    Ok(WithId {
        id: r.get("id")?,
        item: Fill {
            exchange: r.get("exchange")?,
            market: r.get("market")?,
            symbol: r.get("symbol")?,
            side: r.get("side")?,
            qty: dec(r, "qty")?,
            price: dec(r, "price")?,
            fee: dec(r, "fee")?,
            fee_asset: r.get("fee_asset")?,
            realized_pnl: opt_dec(r, "realized_pnl")?,
            time: r.get("time")?,
            dedupe_hash: r.get("dedupe_hash")?,
            import_batch_id: r.get("import_batch_id")?,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{allocations, open_in_memory, test_data, trades};
    use crate::models::Side;
    use rust_decimal_macros::dec;

    #[test]
    fn insert_then_duplicate_hash_is_skipped() {
        let conn = open_in_memory().unwrap();
        let first = insert(&conn, &test_data::fill("h1", 1)).unwrap();
        assert!(matches!(first, InsertOutcome::Inserted(_)));

        // Same hash, even with different values, is treated as the same row.
        let mut again = test_data::fill("h1", 1);
        again.qty = dec!(99);
        assert_eq!(insert(&conn, &again).unwrap(), InsertOutcome::Duplicate);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM fills", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn list_for_instrument_filters_and_sorts_by_time() {
        let conn = open_in_memory().unwrap();
        insert(&conn, &test_data::fill("late", 300)).unwrap();
        insert(&conn, &test_data::fill("early", 100)).unwrap();
        let mut other = test_data::fill("other-symbol", 200);
        other.symbol = "SOLUSDT".into();
        insert(&conn, &other).unwrap();

        let got = list_for_instrument(&conn, "binance", Market::Spot, "ETHUSDT").unwrap();
        let hashes: Vec<_> = got.iter().map(|f| f.item.dedupe_hash.as_str()).collect();
        assert_eq!(hashes, ["early", "late"]);
    }

    #[test]
    fn round_trip_keeps_all_fields() {
        let conn = open_in_memory().unwrap();
        let mut f = test_data::fill("rt", 5);
        f.side = Side::Sell;
        f.realized_pnl = Some(dec!(-1.234567));
        let InsertOutcome::Inserted(id) = insert(&conn, &f).unwrap() else {
            panic!("expected insert");
        };
        let got = list_for_instrument(&conn, "binance", Market::Spot, "ETHUSDT").unwrap();
        assert_eq!(got, vec![WithId { id, item: f }]);
    }

    #[test]
    fn list_for_trade_follows_allocations() {
        let conn = open_in_memory().unwrap();
        let trade_id = trades::insert(&conn, &test_data::trade()).unwrap();
        let InsertOutcome::Inserted(fill_id) = insert(&conn, &test_data::fill("x", 1)).unwrap()
        else {
            panic!("expected insert");
        };
        assert!(list_for_trade(&conn, trade_id).unwrap().is_empty());

        allocations::insert(&conn, fill_id, trade_id, dec!(0.5), true).unwrap();
        assert_eq!(list_for_trade(&conn, trade_id).unwrap()[0].id, fill_id);
    }
}
