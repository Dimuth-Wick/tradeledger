//! Which part of each fill belongs to which trade (written by the grouping
//! engine). Usually one row per fill; a flip fill has two rows, e.g. 0.5
//! closing the short and 0.3 opening the long.

use rusqlite::{Connection, params};
use rust_decimal::Decimal;

use super::DbError;
use super::sql::{SqlDec, dec};
use crate::models::Market;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allocation {
    pub fill_id: i64,
    pub trade_id: i64,
    pub qty: Decimal,
    /// true = this part opened/added to the position, false = reduced it.
    pub is_entry: bool,
}

pub fn insert(
    conn: &Connection,
    fill_id: i64,
    trade_id: i64,
    qty: Decimal,
    is_entry: bool,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO fill_allocations (fill_id, trade_id, qty, is_entry) VALUES (?1, ?2, ?3, ?4)",
        params![fill_id, trade_id, SqlDec(qty), is_entry],
    )?;
    Ok(())
}

/// Removes every allocation of one instrument's fills (before regrouping it).
pub fn delete_for_instrument(
    conn: &Connection,
    exchange: &str,
    market: Market,
    symbol: &str,
) -> Result<(), DbError> {
    conn.execute(
        "DELETE FROM fill_allocations WHERE fill_id IN
            (SELECT id FROM fills WHERE exchange = ?1 AND market = ?2 AND symbol = ?3)",
        params![exchange, market, symbol],
    )?;
    Ok(())
}

pub fn list_for_trade(conn: &Connection, trade_id: i64) -> Result<Vec<Allocation>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT a.* FROM fill_allocations a JOIN fills f ON f.id = a.fill_id
         WHERE a.trade_id = ?1 ORDER BY f.time, f.id",
    )?;
    let rows = stmt.query_map([trade_id], |r| {
        Ok(Allocation {
            fill_id: r.get("fill_id")?,
            trade_id: r.get("trade_id")?,
            qty: dec(r, "qty")?,
            is_entry: r.get("is_entry")?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{InsertOutcome, fills, open_in_memory, test_data, trades};
    use rust_decimal_macros::dec;

    fn fill(conn: &Connection, hash: &str) -> i64 {
        match fills::insert(conn, &test_data::fill(hash, 1)).unwrap() {
            InsertOutcome::Inserted(id) => id,
            InsertOutcome::Duplicate => panic!("duplicate"),
        }
    }

    #[test]
    fn one_fill_can_belong_to_two_trades() {
        let conn = open_in_memory().unwrap();
        let (t1, t2) = (
            trades::insert(&conn, &test_data::trade()).unwrap(),
            trades::insert(&conn, &test_data::trade()).unwrap(),
        );
        let f = fill(&conn, "flip");
        insert(&conn, f, t1, dec!(0.5), false).unwrap();
        insert(&conn, f, t2, dec!(0.3), true).unwrap();

        assert_eq!(
            list_for_trade(&conn, t2).unwrap(),
            vec![Allocation {
                fill_id: f,
                trade_id: t2,
                qty: dec!(0.3),
                is_entry: true
            }]
        );
    }

    #[test]
    fn deleting_a_trade_removes_its_allocations_but_keeps_the_fill() {
        let conn = open_in_memory().unwrap();
        let t = trades::insert(&conn, &test_data::trade()).unwrap();
        let f = fill(&conn, "a");
        insert(&conn, f, t, dec!(0.5), true).unwrap();

        trades::delete(&conn, t).unwrap();
        assert!(list_for_trade(&conn, t).unwrap().is_empty());
        assert_eq!(
            fills::list_for_instrument(&conn, "binance", Market::Spot, "ETHUSDT")
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn delete_for_instrument_only_touches_that_instrument() {
        let conn = open_in_memory().unwrap();
        let t = trades::insert(&conn, &test_data::trade()).unwrap();
        let eth = fill(&conn, "eth");
        let mut sol = test_data::fill("sol", 1);
        sol.symbol = "SOLUSDT".into();
        let InsertOutcome::Inserted(sol) = fills::insert(&conn, &sol).unwrap() else {
            panic!()
        };
        insert(&conn, eth, t, dec!(1), true).unwrap();
        insert(&conn, sol, t, dec!(1), true).unwrap();

        delete_for_instrument(&conn, "binance", Market::Spot, "ETHUSDT").unwrap();
        let left: Vec<i64> = list_for_trade(&conn, t)
            .unwrap()
            .iter()
            .map(|a| a.fill_id)
            .collect();
        assert_eq!(left, [sol]);
    }

    #[test]
    fn allocation_to_missing_trade_is_rejected() {
        let conn = open_in_memory().unwrap();
        let f = fill(&conn, "fk");
        assert!(insert(&conn, f, 12345, dec!(1), true).is_err());
    }
}
