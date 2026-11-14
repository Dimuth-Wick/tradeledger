//! Trades: create, read, update, delete, list.

use rusqlite::types::ToSql;
use rusqlite::{Connection, OptionalExtension, Row};

use super::DbError;
use super::sql::{SqlDec, dec, opt_dec};
use crate::models::{Trade, WithId};

/// Column order used by both INSERT and UPDATE; `bind` passes values in the
/// same order. Keep the three in sync.
const INSERT_SQL: &str = "INSERT INTO trades
    (exchange, market, symbol, direction, status, open_time, close_time,
     qty, avg_entry, avg_exit, leverage, gross_pnl, fees, funding, net_pnl,
     planned_entry, stop_loss, take_profit, planned_rr, r_multiple,
     notes, playbook_id, is_manual)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
            ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)";

const UPDATE_SQL: &str = "UPDATE trades SET
    exchange = ?1, market = ?2, symbol = ?3, direction = ?4, status = ?5,
    open_time = ?6, close_time = ?7, qty = ?8, avg_entry = ?9, avg_exit = ?10,
    leverage = ?11, gross_pnl = ?12, fees = ?13, funding = ?14, net_pnl = ?15,
    planned_entry = ?16, stop_loss = ?17, take_profit = ?18, planned_rr = ?19,
    r_multiple = ?20, notes = ?21, playbook_id = ?22, is_manual = ?23
    WHERE id = ?24";

/// Builds the 23 trade parameters and hands them to `run`.
/// Shared so INSERT and UPDATE can't drift apart. The decimals are wrapped
/// here because the wrapped values must live until the query has run.
fn bind<R>(t: &Trade, run: impl FnOnce(&[&dyn ToSql]) -> R) -> R {
    let (qty, avg_entry, gross, fees, funding, net) = (
        SqlDec(t.qty),
        SqlDec(t.avg_entry),
        SqlDec(t.gross_pnl),
        SqlDec(t.fees),
        SqlDec(t.funding),
        SqlDec(t.net_pnl),
    );
    let avg_exit = t.avg_exit.map(SqlDec);
    let planned_entry = t.planned_entry.map(SqlDec);
    let stop_loss = t.stop_loss.map(SqlDec);
    let take_profit = t.take_profit.map(SqlDec);
    let planned_rr = t.planned_rr.map(SqlDec);
    let r_multiple = t.r_multiple.map(SqlDec);
    run(&[
        &t.exchange,
        &t.market,
        &t.symbol,
        &t.direction,
        &t.status,
        &t.open_time,
        &t.close_time,
        &qty,
        &avg_entry,
        &avg_exit,
        &t.leverage,
        &gross,
        &fees,
        &funding,
        &net,
        &planned_entry,
        &stop_loss,
        &take_profit,
        &planned_rr,
        &r_multiple,
        &t.notes,
        &t.playbook_id,
        &t.is_manual,
    ])
}

pub fn insert(conn: &Connection, t: &Trade) -> Result<i64, DbError> {
    bind(t, |p| conn.execute(INSERT_SQL, p))?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<WithId<Trade>>, DbError> {
    Ok(conn
        .query_row("SELECT * FROM trades WHERE id = ?1", [id], from_row)
        .optional()?)
}

/// Overwrites every field. Returns false if the trade doesn't exist.
pub fn update(conn: &Connection, id: i64, t: &Trade) -> Result<bool, DbError> {
    let changed = bind(t, |p| {
        let mut with_id = p.to_vec();
        with_id.push(&id);
        conn.execute(UPDATE_SQL, &*with_id)
    })?;
    Ok(changed > 0)
}

/// Deletes the trade. Its fills/funding are kept but unlinked; tags, notes,
/// screenshots rows etc. are removed (see ON DELETE rules in the schema).
/// Returns false if the trade doesn't exist.
pub fn delete(conn: &Connection, id: i64) -> Result<bool, DbError> {
    Ok(conn.execute("DELETE FROM trades WHERE id = ?1", [id])? > 0)
}

/// Newest first. Filtering and paging arrive with the trade log (T5.1).
pub fn list(conn: &Connection) -> Result<Vec<WithId<Trade>>, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM trades ORDER BY open_time DESC, id DESC")?;
    let rows = stmt.query_map([], from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn from_row(r: &Row<'_>) -> rusqlite::Result<WithId<Trade>> {
    Ok(WithId {
        id: r.get("id")?,
        item: Trade {
            exchange: r.get("exchange")?,
            market: r.get("market")?,
            symbol: r.get("symbol")?,
            direction: r.get("direction")?,
            status: r.get("status")?,
            open_time: r.get("open_time")?,
            close_time: r.get("close_time")?,
            qty: dec(r, "qty")?,
            avg_entry: dec(r, "avg_entry")?,
            avg_exit: opt_dec(r, "avg_exit")?,
            leverage: r.get("leverage")?,
            gross_pnl: dec(r, "gross_pnl")?,
            fees: dec(r, "fees")?,
            funding: dec(r, "funding")?,
            net_pnl: dec(r, "net_pnl")?,
            planned_entry: opt_dec(r, "planned_entry")?,
            stop_loss: opt_dec(r, "stop_loss")?,
            take_profit: opt_dec(r, "take_profit")?,
            planned_rr: opt_dec(r, "planned_rr")?,
            r_multiple: opt_dec(r, "r_multiple")?,
            notes: r.get("notes")?,
            playbook_id: r.get("playbook_id")?,
            is_manual: r.get("is_manual")?,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{InsertOutcome, fills, funding, open_in_memory, test_data};
    use crate::models::{Direction, TradeStatus};
    use rust_decimal_macros::dec;

    #[test]
    fn insert_and_get_round_trip() {
        let conn = open_in_memory().unwrap();
        let t = test_data::trade();
        let id = insert(&conn, &t).unwrap();
        assert_eq!(get(&conn, id).unwrap(), Some(WithId { id, item: t }));
    }

    #[test]
    fn open_manual_trade_with_no_plan_round_trips() {
        let conn = open_in_memory().unwrap();
        let mut t = test_data::trade();
        t.direction = Direction::Short;
        t.status = TradeStatus::Open;
        t.close_time = None;
        t.avg_exit = None;
        t.leverage = None;
        t.planned_entry = None;
        t.stop_loss = None;
        t.take_profit = None;
        t.planned_rr = None;
        t.r_multiple = None;
        t.is_manual = true;
        let id = insert(&conn, &t).unwrap();
        assert_eq!(get(&conn, id).unwrap().unwrap().item, t);
    }

    #[test]
    fn get_missing_is_none() {
        let conn = open_in_memory().unwrap();
        assert_eq!(get(&conn, 1).unwrap(), None);
    }

    #[test]
    fn update_overwrites_and_reports_missing() {
        let conn = open_in_memory().unwrap();
        let id = insert(&conn, &test_data::trade()).unwrap();

        let mut changed = test_data::trade();
        changed.notes = "Moved stop too early".into();
        changed.stop_loss = Some(dec!(36350.5));
        assert!(update(&conn, id, &changed).unwrap());
        assert_eq!(get(&conn, id).unwrap().unwrap().item, changed);

        assert!(!update(&conn, id + 100, &changed).unwrap());
    }

    #[test]
    fn delete_unlinks_fills_and_funding_and_removes_tags() {
        let conn = open_in_memory().unwrap();
        let trade_id = insert(&conn, &test_data::trade()).unwrap();

        let mut f = test_data::fill("f", 1);
        f.trade_id = Some(trade_id);
        let InsertOutcome::Inserted(fill_id) = fills::insert(&conn, &f).unwrap() else {
            panic!("expected insert");
        };
        let mut e = test_data::funding("e");
        e.trade_id = Some(trade_id);
        funding::insert(&conn, &e).unwrap();
        // Tag repo arrives in T5.3; raw SQL is enough to test the cascade.
        conn.execute_batch(&format!(
            "INSERT INTO tag_groups (id, name) VALUES (1, 'Mistake');
             INSERT INTO tags (id, group_id, name) VALUES (1, 1, 'FOMO');
             INSERT INTO trade_tags (trade_id, tag_id) VALUES ({trade_id}, 1);"
        ))
        .unwrap();

        assert!(delete(&conn, trade_id).unwrap());
        assert!(!delete(&conn, trade_id).unwrap());

        // Fill survives, unlinked.
        let fill_trade: Option<i64> = conn
            .query_row("SELECT trade_id FROM fills WHERE id = ?1", [fill_id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(fill_trade, None);
        assert!(funding::list_for_trade(&conn, trade_id).unwrap().is_empty());
        let links: i64 = conn
            .query_row("SELECT COUNT(*) FROM trade_tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(links, 0);
        let tags: i64 = conn
            .query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tags, 1, "the tag itself is kept");
    }

    #[test]
    fn list_is_newest_first() {
        let conn = open_in_memory().unwrap();
        let mut old = test_data::trade();
        old.open_time = 1_000;
        let mut new = test_data::trade();
        new.open_time = 2_000;
        let old_id = insert(&conn, &old).unwrap();
        let new_id = insert(&conn, &new).unwrap();

        let ids: Vec<i64> = list(&conn).unwrap().iter().map(|t| t.id).collect();
        assert_eq!(ids, [new_id, old_id]);
    }

    #[test]
    fn check_constraint_rejects_unknown_market() {
        let conn = open_in_memory().unwrap();
        let result = conn.execute(
            "INSERT INTO trades (exchange, market, symbol, direction, status, open_time, qty, avg_entry)
             VALUES ('binance', 'coinm', 'BTCUSD', 'long', 'open', 0, '1', '1')",
            [],
        );
        assert!(result.is_err());
    }
}
