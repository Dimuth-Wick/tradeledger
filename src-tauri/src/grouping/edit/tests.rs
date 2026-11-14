//! Merge / split / reset tests (in-memory DB, fake data).

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::*;
use crate::db::{InsertOutcome, open_in_memory};
use crate::grouping::persist::regroup_instrument;
use crate::models::{Direction, Fill, Market, TradeStatus};
use crate::trade_entry::{self, TradeInput};

fn fill(hash: &str, time: i64, side: Side, qty: Decimal, price: Decimal) -> Fill {
    Fill {
        exchange: "binance".into(),
        market: Market::Usdm,
        symbol: "BTCUSDT".into(),
        side,
        qty,
        price,
        fee: Decimal::ZERO,
        fee_asset: "USDT".into(),
        realized_pnl: None,
        time,
        dedupe_hash: hash.into(),
        import_batch_id: None,
    }
}

fn btc() -> Instrument {
    Instrument {
        exchange: "binance".into(),
        market: Market::Usdm,
        symbol: "BTCUSDT".into(),
    }
}

fn insert(conn: &Connection, f: Fill) -> i64 {
    match db::fills::insert(conn, &f).unwrap() {
        InsertOutcome::Inserted(id) => id,
        InsertOutcome::Duplicate => panic!("duplicate"),
    }
}

fn trades(conn: &Connection) -> Vec<WithId<Trade>> {
    db::trades::list_imported_for_instrument(conn, "binance", Market::Usdm, "BTCUSDT")
        .unwrap()
        .into_iter()
        .map(|(t, _)| t)
        .collect()
}

/// Two long round trips: +10 then +20.
fn two_longs(conn: &Connection) {
    insert(conn, fill("a1", 1, Side::Buy, dec!(1), dec!(100)));
    insert(conn, fill("a2", 2, Side::Sell, dec!(1), dec!(110)));
    insert(conn, fill("b1", 3, Side::Buy, dec!(1), dec!(100)));
    insert(conn, fill("b2", 4, Side::Sell, dec!(1), dec!(120)));
    regroup_instrument(conn, &btc()).unwrap();
}

#[test]
fn merge_joins_two_trades_and_moves_the_journal() {
    let conn = open_in_memory().unwrap();
    two_longs(&conn);
    let [first, second] = <[_; 2]>::try_from(trades(&conn)).unwrap();

    // Journal on both trades.
    let mut a = first.item.clone();
    a.notes = "first idea".into();
    db::trades::update(&conn, first.id, &a).unwrap();
    let mut b = second.item.clone();
    b.notes = "re-entry".into();
    b.stop_loss = Some(dec!(95));
    db::trades::update(&conn, second.id, &b).unwrap();
    conn.execute_batch(&format!(
        "INSERT INTO tag_groups (id, name) VALUES (1, 'Setup');
         INSERT INTO tags (id, group_id, name) VALUES (1, 1, 'Breakout');
         INSERT INTO trade_tags (trade_id, tag_id) VALUES ({}, 1);",
        second.id
    ))
    .unwrap();

    let summary = merge_with_previous(&conn, second.id).unwrap();
    assert_eq!(summary.trades_removed, 1);

    let after = trades(&conn);
    assert_eq!(after.len(), 1);
    let t = &after[0];
    assert_eq!(t.id, first.id, "the earlier trade survives");
    assert_eq!(t.item.gross_pnl, dec!(30));
    assert_eq!(t.item.qty, dec!(2));
    assert_eq!(t.item.close_time, Some(4));
    assert_eq!(t.item.notes, "first idea\n\nre-entry");
    assert_eq!(
        t.item.stop_loss,
        Some(dec!(95)),
        "plan taken from the later trade"
    );
    let tagged: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM trade_tags WHERE trade_id = ?1",
            [first.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tagged, 1);
    assert_eq!(trade_fills(&conn, t.id).unwrap().len(), 4);
}

#[test]
fn merge_survives_later_regroups() {
    let conn = open_in_memory().unwrap();
    two_longs(&conn);
    let second = trades(&conn)[1].id;
    merge_with_previous(&conn, second).unwrap();

    // A new, unrelated round trip arrives with the next import.
    insert(&conn, fill("c1", 5, Side::Buy, dec!(1), dec!(100)));
    insert(&conn, fill("c2", 6, Side::Sell, dec!(1), dec!(101)));
    regroup_instrument(&conn, &btc()).unwrap();

    let gross: Vec<Decimal> = trades(&conn).iter().map(|t| t.item.gross_pnl).collect();
    assert_eq!(gross, [dec!(30), dec!(1)]);
}

#[test]
fn split_at_a_fill_carries_the_open_position_at_average_cost() {
    // buy 1 @100, buy 1 @120, sell 2 @130 → one trade, gross +40.
    let conn = open_in_memory().unwrap();
    insert(&conn, fill("s1", 1, Side::Buy, dec!(1), dec!(100)));
    let second_buy = insert(&conn, fill("s2", 2, Side::Buy, dec!(1), dec!(120)));
    insert(&conn, fill("s3", 3, Side::Sell, dec!(2), dec!(130)));
    regroup_instrument(&conn, &btc()).unwrap();
    let id = trades(&conn)[0].id;

    split_at_fill(&conn, id, second_buy).unwrap();
    let t = trades(&conn);
    assert_eq!(t.len(), 2);
    // First part: 1 @100 bought, nothing sold yet, closed by the split.
    assert_eq!(
        t[0].id, id,
        "first part keeps the original trade (and its journal)"
    );
    assert_eq!(t[0].item.status, TradeStatus::Closed);
    assert_eq!(t[0].item.close_time, Some(1));
    assert_eq!(t[0].item.gross_pnl, Decimal::ZERO);
    // Second part: carried 1 @100 + 1 @120 → avg 110, sold 2 @130 → +40.
    assert_eq!(t[1].item.avg_entry, dec!(110));
    assert_eq!(t[1].item.gross_pnl, dec!(40));
    assert_eq!(t[1].item.open_time, 2);
}

#[test]
fn split_undoes_a_merge() {
    let conn = open_in_memory().unwrap();
    two_longs(&conn);
    merge_with_previous(&conn, trades(&conn)[1].id).unwrap();
    let merged = trades(&conn)[0].id;
    let fills = trade_fills(&conn, merged).unwrap();
    let b1 = fills[2].fill_id;

    split_at_fill(&conn, merged, b1).unwrap();
    let gross: Vec<Decimal> = trades(&conn).iter().map(|t| t.item.gross_pnl).collect();
    assert_eq!(gross, [dec!(10), dec!(20)]);
}

#[test]
fn reset_removes_all_rules() {
    let conn = open_in_memory().unwrap();
    two_longs(&conn);
    merge_with_previous(&conn, trades(&conn)[1].id).unwrap();
    assert_eq!(trades(&conn).len(), 1);

    reset_grouping(&conn, trades(&conn)[0].id).unwrap();
    assert_eq!(trades(&conn).len(), 2);
}

#[test]
fn flip_fill_shows_its_partial_quantity() {
    let conn = open_in_memory().unwrap();
    insert(&conn, fill("f1", 1, Side::Buy, dec!(1), dec!(100)));
    insert(&conn, fill("f2", 2, Side::Sell, dec!(3), dec!(110)));
    regroup_instrument(&conn, &btc()).unwrap();
    let t = trades(&conn);
    let long_fills = trade_fills(&conn, t[0].id).unwrap();
    assert_eq!(
        (
            long_fills[1].fill_qty,
            long_fills[1].qty,
            long_fills[1].is_entry
        ),
        (dec!(3), dec!(1), false)
    );
    let short_fills = trade_fills(&conn, t[1].id).unwrap();
    assert_eq!(
        (short_fills[0].qty, short_fills[0].is_entry),
        (dec!(2), true)
    );
}

#[test]
fn fully_used_fill_shows_equal_quantities_even_with_padded_zeros() {
    let conn = open_in_memory().unwrap();
    insert(&conn, fill("p1", 1, Side::Buy, dec!(0.010), dec!(100)));
    insert(&conn, fill("p2", 2, Side::Sell, dec!(0.0100), dec!(110)));
    regroup_instrument(&conn, &btc()).unwrap();
    let fills = trade_fills(&conn, trades(&conn)[0].id).unwrap();
    for f in fills {
        assert_eq!(f.qty.to_string(), f.fill_qty.to_string());
    }
}

#[test]
fn invalid_requests_are_rejected_with_clear_errors() {
    let conn = open_in_memory().unwrap();
    insert(&conn, fill("x1", 1, Side::Buy, dec!(1), dec!(100)));
    insert(&conn, fill("x2", 2, Side::Sell, dec!(1), dec!(110)));
    insert(&conn, fill("y1", 3, Side::Sell, dec!(1), dec!(110)));
    let y2 = insert(&conn, fill("y2", 4, Side::Buy, dec!(1), dec!(100)));
    regroup_instrument(&conn, &btc()).unwrap();
    let [long, short] = <[_; 2]>::try_from(trades(&conn)).unwrap();
    assert_eq!(short.item.direction, Direction::Short);

    assert!(matches!(
        merge_with_previous(&conn, long.id),
        Err(EditError::NoPrevious)
    ));
    assert!(matches!(
        merge_with_previous(&conn, short.id),
        Err(EditError::DirectionMismatch("short", "long"))
    ));
    assert!(matches!(
        split_at_fill(&conn, long.id, y2),
        Err(EditError::FillNotInTrade)
    ));
    let first_fill = trade_fills(&conn, long.id).unwrap()[0].fill_id;
    assert!(matches!(
        split_at_fill(&conn, long.id, first_fill),
        Err(EditError::FirstFill)
    ));
    assert!(matches!(
        merge_with_previous(&conn, 999),
        Err(EditError::NotFound(999))
    ));

    let manual = trade_entry::create(
        &conn,
        &TradeInput {
            exchange: "binance".into(),
            market: Market::Usdm,
            symbol: "BTCUSDT".into(),
            direction: Direction::Long,
            open_time: 1,
            close_time: None,
            qty: dec!(1),
            avg_entry: dec!(100),
            avg_exit: None,
            leverage: None,
            fees: Decimal::ZERO,
            funding: Decimal::ZERO,
            planned_entry: None,
            stop_loss: None,
            take_profit: None,
            notes: String::new(),
            playbook_id: None,
        },
    )
    .unwrap();
    assert!(matches!(
        merge_with_previous(&conn, manual.id),
        Err(EditError::Manual)
    ));
    assert!(matches!(
        reset_grouping(&conn, manual.id),
        Err(EditError::Manual)
    ));

    // Nothing was changed by the failed attempts.
    assert_eq!(trades(&conn).len(), 2);
    let rules: i64 = conn
        .query_row("SELECT COUNT(*) FROM grouping_overrides", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rules, 0);
}
