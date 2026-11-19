//! Futures funding payments. Deduplicated like fills.

use rusqlite::{Connection, Row, params};

use super::sql::{SqlDec, dec};
use super::{DbError, InsertOutcome};
use crate::models::{FundingEvent, WithId};

pub fn insert(conn: &Connection, e: &FundingEvent) -> Result<InsertOutcome, DbError> {
    let changed = conn.execute(
        "INSERT INTO funding_events
            (exchange, symbol, amount, time, dedupe_hash, trade_id, import_batch_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(dedupe_hash) DO NOTHING",
        params![
            e.exchange,
            e.symbol,
            SqlDec(e.amount),
            e.time,
            e.dedupe_hash,
            e.trade_id,
            e.import_batch_id
        ],
    )?;
    Ok(if changed == 0 {
        InsertOutcome::Duplicate
    } else {
        InsertOutcome::Inserted(conn.last_insert_rowid())
    })
}

/// True if a funding event with this dedupe hash is already stored.
pub fn hash_exists(conn: &Connection, hash: &str) -> Result<bool, DbError> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM funding_events WHERE dedupe_hash = ?1)",
        [hash],
        |r| r.get(0),
    )?)
}

/// All funding of one futures symbol, in time order (input for grouping).
pub fn list_for_symbol(
    conn: &Connection,
    exchange: &str,
    symbol: &str,
) -> Result<Vec<WithId<FundingEvent>>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM funding_events WHERE exchange = ?1 AND symbol = ?2 ORDER BY time, id",
    )?;
    let rows = stmt.query_map(params![exchange, symbol], from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Unlinks all funding of a symbol from trades (before regrouping it).
pub fn clear_trade_links(conn: &Connection, exchange: &str, symbol: &str) -> Result<(), DbError> {
    conn.execute(
        "UPDATE funding_events SET trade_id = NULL WHERE exchange = ?1 AND symbol = ?2",
        params![exchange, symbol],
    )?;
    Ok(())
}

pub fn set_trade(conn: &Connection, funding_id: i64, trade_id: i64) -> Result<(), DbError> {
    conn.execute(
        "UPDATE funding_events SET trade_id = ?1 WHERE id = ?2",
        params![trade_id, funding_id],
    )?;
    Ok(())
}

pub fn list_for_trade(
    conn: &Connection,
    trade_id: i64,
) -> Result<Vec<WithId<FundingEvent>>, DbError> {
    let mut stmt =
        conn.prepare("SELECT * FROM funding_events WHERE trade_id = ?1 ORDER BY time, id")?;
    let rows = stmt.query_map([trade_id], from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn from_row(r: &Row<'_>) -> rusqlite::Result<WithId<FundingEvent>> {
    Ok(WithId {
        id: r.get("id")?,
        item: FundingEvent {
            exchange: r.get("exchange")?,
            symbol: r.get("symbol")?,
            amount: dec(r, "amount")?,
            time: r.get("time")?,
            dedupe_hash: r.get("dedupe_hash")?,
            trade_id: r.get("trade_id")?,
            import_batch_id: r.get("import_batch_id")?,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{open_in_memory, test_data, trades};

    #[test]
    fn insert_dedupes_and_lists_by_trade() {
        let conn = open_in_memory().unwrap();
        let trade_id = trades::insert(&conn, &test_data::trade()).unwrap();

        let mut e = test_data::funding("f1");
        e.trade_id = Some(trade_id);
        let InsertOutcome::Inserted(id) = insert(&conn, &e).unwrap() else {
            panic!("expected insert");
        };
        assert_eq!(insert(&conn, &e).unwrap(), InsertOutcome::Duplicate);

        assert_eq!(
            list_for_trade(&conn, trade_id).unwrap(),
            vec![WithId { id, item: e }]
        );
    }
}
