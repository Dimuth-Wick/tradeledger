//! Regroup-to-DB tests (in-memory DB, fake fixture data).

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::*;
use crate::db::{InsertOutcome, open_in_memory};
use crate::importers::{
    self, BinanceSpot, BinanceUsdmTrades, BinanceUsdmTransactions, ExchangeImporter,
};
use crate::models::{Direction, Fill, Side, WithId};
use crate::trade_entry::{self, TradeInput};

const SPOT: &[u8] = include_bytes!("../../../../tests/fixtures/binance-spot-trades.csv");
const FUTURES: &[u8] = include_bytes!("../../../../tests/fixtures/binance-futures-trades.csv");
const TX: &[u8] = include_bytes!("../../../../tests/fixtures/binance-futures-transactions.csv");

/// Imports one file and regroups what it touched, like the import command will.
fn import(conn: &Connection, importer: &dyn ExchangeImporter, data: &[u8]) -> RegroupSummary {
    let report = importer.parse(data).unwrap();
    importers::store(conn, importer, "fixture.csv", 1, &report).unwrap();
    regroup_all(conn, &touched_instruments(&report)).unwrap()
}

fn import_all(conn: &Connection) -> RegroupSummary {
    let mut total = import(conn, &BinanceSpot, SPOT);
    total += import(conn, &BinanceUsdmTrades, FUTURES);
    total += import(conn, &BinanceUsdmTransactions, TX);
    total
}

fn trades_of(conn: &Connection, market: Market, symbol: &str) -> Vec<WithId<Trade>> {
    db::trades::list_imported_for_instrument(conn, "binance", market, symbol)
        .unwrap()
        .into_iter()
        .map(|(t, _)| t)
        .collect()
}

fn group_key(conn: &Connection, id: i64) -> Option<String> {
    conn.query_row("SELECT group_key FROM trades WHERE id = ?1", [id], |r| {
        r.get(0)
    })
    .unwrap()
}

#[test]
fn fixtures_become_seven_trades_with_allocations_and_funding() {
    let conn = open_in_memory().unwrap();
    import_all(&conn);
    assert_eq!(db::trades::list(&conn).unwrap().len(), 7);

    let eth = trades_of(&conn, Market::Usdm, "ETHUSDT");
    assert_eq!(eth.len(), 2);
    assert_eq!(eth[0].item.direction, Direction::Short);
    assert_eq!(eth[0].item.net_pnl, dec!(19.2905));
    assert_eq!(eth[1].item.net_pnl, dec!(5.48505));

    // The flip fill is allocated to both ETH trades.
    let short_fills = db::fills::list_for_trade(&conn, eth[0].id).unwrap();
    let long_fills = db::fills::list_for_trade(&conn, eth[1].id).unwrap();
    assert_eq!(short_fills.len(), 2);
    assert_eq!(long_fills.len(), 2);
    assert_eq!(short_fills[1].id, long_fills[0].id);

    // Funding is linked and included.
    let btc = &trades_of(&conn, Market::Usdm, "BTCUSDT")[0];
    assert_eq!(db::funding::list_for_trade(&conn, btc.id).unwrap().len(), 3);
    assert_eq!(btc.item.net_pnl, dec!(7.961748));
}

#[test]
fn funding_file_imported_after_trades_updates_the_trades() {
    let conn = open_in_memory().unwrap();
    import(&conn, &BinanceUsdmTrades, FUTURES);
    let before = trades_of(&conn, Market::Usdm, "BTCUSDT")[0].clone();
    assert_eq!(before.item.funding, Decimal::ZERO);

    let summary = import(&conn, &BinanceUsdmTransactions, TX);
    assert_eq!(summary.trades_created, 0);
    let after = &trades_of(&conn, Market::Usdm, "BTCUSDT")[0];
    assert_eq!(after.id, before.id);
    assert_eq!(after.item.funding, dec!(0.05185));
}

#[test]
fn spot_and_futures_with_the_same_symbol_stay_separate() {
    let conn = open_in_memory().unwrap();
    import_all(&conn);
    let spot_eth = &trades_of(&conn, Market::Spot, "ETHUSDT")[0];
    assert_eq!(
        spot_eth.item.funding,
        Decimal::ZERO,
        "no futures funding on spot"
    );
    assert_eq!(
        db::fills::list_for_trade(&conn, spot_eth.id).unwrap().len(),
        3
    );
}

#[test]
fn regrouping_again_changes_nothing() {
    let conn = open_in_memory().unwrap();
    import_all(&conn);
    let before = db::trades::list(&conn).unwrap();

    let all: Vec<Instrument> = before
        .iter()
        .map(|t| Instrument {
            exchange: t.item.exchange.clone(),
            market: t.item.market,
            symbol: t.item.symbol.clone(),
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let summary = regroup_all(&conn, &all).unwrap();
    assert_eq!(
        (
            summary.trades_created,
            summary.trades_updated,
            summary.trades_removed
        ),
        (0, 7, 0)
    );
    assert_eq!(db::trades::list(&conn).unwrap(), before);
}

#[test]
fn reimporting_the_same_file_keeps_trade_ids() {
    let conn = open_in_memory().unwrap();
    import(&conn, &BinanceUsdmTrades, FUTURES);
    let ids: Vec<i64> = db::trades::list(&conn)
        .unwrap()
        .iter()
        .map(|t| t.id)
        .collect();
    let summary = import(&conn, &BinanceUsdmTrades, FUTURES);
    assert_eq!(summary.trades_created, 0);
    let again: Vec<i64> = db::trades::list(&conn)
        .unwrap()
        .iter()
        .map(|t| t.id)
        .collect();
    assert_eq!(ids, again);
}

#[test]
fn journal_data_and_tags_survive_regroup() {
    let conn = open_in_memory().unwrap();
    import(&conn, &BinanceUsdmTrades, FUTURES);
    let btc = trades_of(&conn, Market::Usdm, "BTCUSDT")[0].clone();

    // The user journals the trade.
    let mut edited = btc.item.clone();
    edited.notes = "Breakout retest, entered late".into();
    edited.stop_loss = Some(dec!(60500));
    edited.take_profit = Some(dec!(63000));
    edited.leverage = Some(10);
    db::trades::update(&conn, btc.id, &edited).unwrap();
    conn.execute_batch(&format!(
        "INSERT INTO tag_groups (id, name) VALUES (1, 'Setup');
         INSERT INTO tags (id, group_id, name) VALUES (1, 1, 'Breakout');
         INSERT INTO trade_tags (trade_id, tag_id) VALUES ({}, 1);",
        btc.id
    ))
    .unwrap();

    // Funding arrives later → regroup.
    import(&conn, &BinanceUsdmTransactions, TX);

    let after = db::trades::get(&conn, btc.id).unwrap().unwrap().item;
    assert_eq!(after.notes, "Breakout retest, entered late");
    assert_eq!(after.stop_loss, Some(dec!(60500)));
    assert_eq!(after.leverage, Some(10));
    assert_eq!(after.planned_rr, Some(dec!(4))); // (63000 − 61000) / 500
    assert_eq!(after.r_multiple, Some(dec!(1.70))); // 850.5 / 500 = 1.701
    assert_eq!(after.funding, dec!(0.05185)); // numbers refreshed
    let tags: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM trade_tags WHERE trade_id = ?1",
            [btc.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tags, 1);
}

// ───────────── synthetic: older fill imported later ─────────────

fn fill(hash: &str, time: i64, side: Side, qty: Decimal, price: Decimal) -> Fill {
    Fill {
        exchange: "binance".into(),
        market: Market::Usdm,
        symbol: "XRPUSDT".into(),
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

fn xrp() -> Instrument {
    Instrument {
        exchange: "binance".into(),
        market: Market::Usdm,
        symbol: "XRPUSDT".into(),
    }
}

fn insert_fill(conn: &Connection, f: &Fill) -> i64 {
    match db::fills::insert(conn, f).unwrap() {
        InsertOutcome::Inserted(id) => id,
        InsertOutcome::Duplicate => panic!("duplicate"),
    }
}

#[test]
fn older_fill_imported_later_extends_the_trade_and_keeps_notes() {
    let conn = open_in_memory().unwrap();
    insert_fill(&conn, &fill("A", 200, Side::Buy, dec!(1), dec!(100)));
    insert_fill(&conn, &fill("B", 300, Side::Sell, dec!(1), dec!(110)));
    regroup_instrument(&conn, &xrp()).unwrap();
    let original = trades_of(&conn, Market::Usdm, "XRPUSDT")[0].clone();
    assert_eq!(group_key(&conn, original.id).as_deref(), Some("A"));
    let mut noted = original.item.clone();
    noted.notes = "keep me".into();
    db::trades::update(&conn, original.id, &noted).unwrap();

    // An earlier buy shows up (e.g. from an older export).
    insert_fill(&conn, &fill("E", 100, Side::Buy, dec!(1), dec!(90)));
    let summary = regroup_instrument(&conn, &xrp()).unwrap();
    assert_eq!(
        (
            summary.trades_created,
            summary.trades_updated,
            summary.trades_removed
        ),
        (0, 1, 0)
    );

    let trades = trades_of(&conn, Market::Usdm, "XRPUSDT");
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].id, original.id);
    assert_eq!(trades[0].item.notes, "keep me");
    assert_eq!(trades[0].item.status, TradeStatus::Open); // 2 bought, 1 sold
    assert_eq!(group_key(&conn, original.id).as_deref(), Some("E"));
}

#[test]
fn trade_is_removed_when_its_fills_are_gone() {
    let conn = open_in_memory().unwrap();
    insert_fill(&conn, &fill("A", 1, Side::Buy, dec!(1), dec!(100)));
    insert_fill(&conn, &fill("B", 2, Side::Sell, dec!(1), dec!(110)));
    regroup_instrument(&conn, &xrp()).unwrap();

    conn.execute("DELETE FROM fills", []).unwrap();
    let summary = regroup_instrument(&conn, &xrp()).unwrap();
    assert_eq!(summary.trades_removed, 1);
    assert!(trades_of(&conn, Market::Usdm, "XRPUSDT").is_empty());
}

#[test]
fn manual_trades_are_never_touched() {
    let conn = open_in_memory().unwrap();
    let manual = trade_entry::create(
        &conn,
        &TradeInput {
            position_size: None,
            auto_fees: false,
            exchange: "binance".into(),
            market: Market::Usdm,
            symbol: "XRPUSDT".into(),
            direction: Direction::Long,
            open_time: 1,
            close_time: None,
            qty: dec!(5),
            avg_entry: dec!(0.5),
            avg_exit: None,
            leverage: None,
            fees: Decimal::ZERO,
            funding: Decimal::ZERO,
            planned_entry: None,
            stop_loss: None,
            take_profit: None,
            notes: "manual".into(),
            playbook_id: None,
        },
    )
    .unwrap();
    insert_fill(&conn, &fill("A", 1, Side::Buy, dec!(1), dec!(100)));
    regroup_instrument(&conn, &xrp()).unwrap();
    regroup_instrument(&conn, &xrp()).unwrap();

    assert_eq!(db::trades::get(&conn, manual.id).unwrap(), Some(manual));
    assert_eq!(db::trades::list(&conn).unwrap().len(), 2);
}

// ───────────── merge_journal ─────────────

#[test]
fn merge_keeps_journal_fields_and_drops_r_when_plan_no_longer_fits() {
    let computed = Trade {
        avg_entry: dec!(100),
        avg_exit: Some(dec!(110)),
        notes: String::new(),
        ..db::test_data::trade()
    };
    let existing = Trade {
        notes: "n".into(),
        planned_entry: None,
        stop_loss: Some(dec!(105)), // above entry for a long → no valid risk
        take_profit: Some(dec!(120)),
        playbook_id: Some(3),
        ..db::test_data::trade()
    };
    let merged = merge_journal(computed, &existing);
    assert_eq!(merged.notes, "n");
    assert_eq!(merged.playbook_id, Some(3));
    assert_eq!(merged.planned_rr, None);
    assert_eq!(merged.r_multiple, None);
    assert_eq!(
        merged.avg_entry,
        dec!(100),
        "numbers come from the new grouping"
    );
}

#[test]
fn touched_instruments_include_funding_symbols_as_futures() {
    let funding = BinanceUsdmTransactions.parse(TX).unwrap();
    let symbols: Vec<(Market, String)> = touched_instruments(&funding)
        .into_iter()
        .map(|i| (i.market, i.symbol))
        .collect();
    assert_eq!(
        symbols,
        [
            (Market::Usdm, "BTCUSDT".to_string()),
            (Market::Usdm, "ETHUSDT".to_string()),
            (Market::Usdm, "SOLUSDT".to_string()),
        ]
    );
    let spot = BinanceSpot.parse(SPOT).unwrap();
    assert_eq!(touched_instruments(&spot).len(), 3);
}
