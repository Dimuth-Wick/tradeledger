//! Grouping tests. Fixture expectations come from docs/binance-csv.md.

use rust_decimal_macros::dec;

use super::*;
use crate::importers::{BinanceSpot, BinanceUsdmTrades, BinanceUsdmTransactions, ExchangeImporter};

const SPOT: &[u8] = include_bytes!("../../../tests/fixtures/binance-spot-trades.csv");
const FUTURES: &[u8] = include_bytes!("../../../tests/fixtures/binance-futures-trades.csv");
const TX: &[u8] = include_bytes!("../../../tests/fixtures/binance-futures-transactions.csv");

/// Gives parsed rows ids 1, 2, 3… like the DB would.
fn with_ids<T>(items: Vec<T>) -> Vec<WithId<T>> {
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| WithId {
            id: i as i64 + 1,
            item,
        })
        .collect()
}

fn fills_for(data: &[u8], importer: &dyn ExchangeImporter, symbol: &str) -> Vec<WithId<Fill>> {
    with_ids(importer.parse(data).unwrap().fills)
        .into_iter()
        .filter(|f| f.item.symbol == symbol)
        .collect()
}

fn funding_for(symbol: &str) -> Vec<WithId<FundingEvent>> {
    with_ids(BinanceUsdmTransactions.parse(TX).unwrap().funding)
        .into_iter()
        .filter(|f| f.item.symbol == symbol)
        .collect()
}

/// Synthetic futures fill (fake data), fee in USDT.
fn fut(id: i64, time: i64, side: Side, qty: Decimal, price: Decimal) -> WithId<Fill> {
    WithId {
        id,
        item: Fill {
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
            dedupe_hash: format!("h{id}"),
            import_batch_id: None,
        },
    }
}

fn spot(id: i64, time: i64, side: Side, qty: Decimal, price: Decimal) -> WithId<Fill> {
    let mut f = fut(id, time, side, qty, price);
    f.item.market = Market::Spot;
    f.item.symbol = "ETHUSDT".into();
    f
}

fn funding_event(id: i64, time: i64, amount: Decimal) -> WithId<FundingEvent> {
    WithId {
        id,
        item: FundingEvent {
            exchange: "binance".into(),
            symbol: "BTCUSDT".into(),
            amount,
            time,
            dedupe_hash: format!("f{id}"),
            trade_id: None,
            import_batch_id: None,
        },
    }
}

use Side::{Buy, Sell};

// ───────────────────────────── fixtures ─────────────────────────────

#[test]
fn s1_eth_partial_buys_with_base_asset_fee_close_on_net_quantity() {
    let fills = fills_for(SPOT, &BinanceSpot, "ETHUSDT");
    let r = group_instrument(&fills, &[]).unwrap();
    assert_eq!(r.trades.len(), 1);
    assert!(r.orphans.is_empty());

    let t = &r.trades[0].trade;
    assert_eq!(t.direction, Direction::Long);
    assert_eq!(t.status, TradeStatus::Closed);
    assert_eq!(t.qty, dec!(0.5));
    assert_eq!(t.avg_entry, dec!(2000.35)); // (0.3×2000.25 + 0.2×2000.5) / 0.5
    assert_eq!(t.avg_exit, Some(dec!(2100)));
    assert_eq!(t.gross_pnl, dec!(49.775175)); // (2100 − 2000.35) × 0.4995
    // ETH fees valued at their fill price (1.000175) + USDT sell fee (1.04895).
    assert_eq!(t.fees, dec!(2.049125));
    assert_eq!(t.net_pnl, dec!(47.72605));
    // Cross-check with cash flow: 1048.95 received − 1000.175 paid − 1.04895 fee.
    assert_eq!(t.net_pnl, dec!(1048.95) - dec!(1000.175) - dec!(1.04895));
    assert_eq!(r.trades[0].legs.len(), 3);
}

#[test]
fn s2_sol_identical_rows_and_bnb_fees() {
    let fills = fills_for(SPOT, &BinanceSpot, "SOLUSDT");
    let r = group_instrument(&fills, &[]).unwrap();
    assert_eq!(r.trades.len(), 1);
    let g = &r.trades[0];
    assert_eq!(g.legs.len(), 4, "both identical buys are used");
    assert_eq!(g.trade.qty, dec!(2.5));
    assert_eq!(g.trade.avg_entry, dec!(95.12));
    assert_eq!(g.trade.gross_pnl, dec!(-6.8)); // (92.4 − 95.12) × 2.5
    assert_eq!(g.trade.fees, Decimal::ZERO, "BNB can't be priced offline");
    assert_eq!(g.unconverted_fees.get("BNB"), Some(&dec!(0.0006125)));
    assert_eq!(g.trade.net_pnl, dec!(-6.8));
}

#[test]
fn s3_btc_single_buy_stays_open() {
    let fills = fills_for(SPOT, &BinanceSpot, "BTCUSDT");
    let r = group_instrument(&fills, &[]).unwrap();
    let t = &r.trades[0].trade;
    assert_eq!(t.status, TradeStatus::Open);
    assert_eq!(t.close_time, None);
    assert_eq!(t.avg_exit, None);
    assert_eq!(t.gross_pnl, Decimal::ZERO);
    assert_eq!(t.fees, dec!(0.0605)); // 0.000001 BTC × 60500
    assert_eq!(t.net_pnl, dec!(-0.0605));
}

#[test]
fn f1_btc_long_with_funding() {
    let r = group_instrument(
        &fills_for(FUTURES, &BinanceUsdmTrades, "BTCUSDT"),
        &funding_for("BTCUSDT"),
    )
    .unwrap();
    assert_eq!(r.trades.len(), 1);
    let g = &r.trades[0];
    assert_eq!(g.trade.direction, Direction::Long);
    assert_eq!(g.trade.gross_pnl, dec!(8.505));
    assert_eq!(g.trade.fees, dec!(0.491402));
    assert_eq!(g.trade.funding, dec!(0.05185)); // paid
    assert_eq!(g.trade.net_pnl, dec!(7.961748));
    assert_eq!(g.funding_ids.len(), 3);
    assert!(r.unassigned_funding.is_empty());
}

#[test]
fn f2_eth_flip_splits_the_fill_into_two_trades() {
    let fills = fills_for(FUTURES, &BinanceUsdmTrades, "ETHUSDT");
    let flip_id = fills[1].id; // the BUY 0.8
    let r = group_instrument(&fills, &funding_for("ETHUSDT")).unwrap();
    assert_eq!(r.trades.len(), 2);

    let short = &r.trades[0];
    assert_eq!(short.trade.direction, Direction::Short);
    assert_eq!(short.trade.qty, dec!(0.5));
    assert_eq!(short.trade.avg_exit, Some(dec!(2010)));
    assert_eq!(short.trade.gross_pnl, dec!(20));
    assert_eq!(short.trade.fees, dec!(0.812)); // 0.41 + 0.6432 × 0.5/0.8
    assert_eq!(short.trade.funding, dec!(-0.1025)); // received
    assert_eq!(short.trade.net_pnl, dec!(19.2905)); // 20 − 0.812 + 0.1025
    assert_eq!(short.trade.close_time, Some(fills[1].item.time));
    assert!(short.legs.contains(&Leg {
        fill_id: flip_id,
        qty: dec!(0.5),
        is_entry: false
    }));

    let long = &r.trades[1];
    assert_eq!(long.trade.direction, Direction::Long);
    assert_eq!(long.trade.open_time, fills[1].item.time);
    assert_eq!(long.trade.qty, dec!(0.3));
    assert_eq!(long.trade.avg_entry, dec!(2010));
    assert_eq!(long.trade.gross_pnl, dec!(6));
    assert_eq!(long.trade.fees, dec!(0.4848)); // 0.6432 × 0.3/0.8 + 0.2436
    assert_eq!(long.trade.funding, dec!(0.03015)); // paid
    assert_eq!(long.trade.net_pnl, dec!(5.48505));
    assert_eq!(
        long.legs[0],
        Leg {
            fill_id: flip_id,
            qty: dec!(0.3),
            is_entry: true
        }
    );
}

#[test]
fn f3_sol_losing_short() {
    let r = group_instrument(
        &fills_for(FUTURES, &BinanceUsdmTrades, "SOLUSDT"),
        &funding_for("SOLUSDT"),
    )
    .unwrap();
    let t = &r.trades[0].trade;
    assert_eq!(t.direction, Direction::Short);
    assert_eq!(t.gross_pnl, dec!(-22));
    assert_eq!(t.fees, dec!(0.7688));
    assert_eq!(t.funding, dec!(-0.095));
    assert_eq!(t.net_pnl, dec!(-22.6738));
}

#[test]
fn our_gross_pnl_matches_binance_realized_profit() {
    for symbol in ["BTCUSDT", "ETHUSDT", "SOLUSDT"] {
        let fills = fills_for(FUTURES, &BinanceUsdmTrades, symbol);
        let binance: Decimal = fills.iter().filter_map(|f| f.item.realized_pnl).sum();
        let ours: Decimal = group_instrument(&fills, &[])
            .unwrap()
            .trades
            .iter()
            .map(|g| g.trade.gross_pnl)
            .sum();
        assert_eq!(ours, binance, "{symbol}");
    }
}

// ───────────────────────────── synthetic ─────────────────────────────

#[test]
fn scale_in_after_partial_exit_uses_average_cost() {
    // buy 1 @100, sell 0.5 @110 (+5), buy 0.5 @120 (avg cost now 110), sell 1 @130 (+20)
    let fills = vec![
        fut(1, 1, Buy, dec!(1), dec!(100)),
        fut(2, 2, Sell, dec!(0.5), dec!(110)),
        fut(3, 3, Buy, dec!(0.5), dec!(120)),
        fut(4, 4, Sell, dec!(1), dec!(130)),
    ];
    let r = group_instrument(&fills, &[]).unwrap();
    assert_eq!(r.trades.len(), 1);
    let t = &r.trades[0].trade;
    assert_eq!(t.gross_pnl, dec!(25)); // = cash flow: 185 out − 160 in
    assert_eq!(t.qty, dec!(1.5));
    assert_eq!(t.avg_entry, dec!(106.66666667)); // (100 + 60) / 1.5, rounded
    assert_eq!(t.avg_exit, Some(dec!(123.33333333))); // (55 + 130) / 1.5
}

#[test]
fn exact_close_does_not_open_a_new_trade() {
    let fills = vec![
        fut(1, 1, Sell, dec!(2), dec!(50)),
        fut(2, 2, Buy, dec!(2), dec!(45)),
    ];
    let r = group_instrument(&fills, &[]).unwrap();
    assert_eq!(r.trades.len(), 1);
    assert_eq!(r.trades[0].trade.gross_pnl, dec!(10));
}

#[test]
fn several_round_trips_make_several_trades() {
    let fills = vec![
        fut(1, 1, Buy, dec!(1), dec!(10)),
        fut(2, 2, Sell, dec!(1), dec!(11)),
        fut(3, 3, Sell, dec!(1), dec!(12)),
        fut(4, 4, Buy, dec!(1), dec!(13)),
        fut(5, 5, Buy, dec!(1), dec!(14)),
    ];
    let r = group_instrument(&fills, &[]).unwrap();
    let summary: Vec<_> = r
        .trades
        .iter()
        .map(|g| (g.trade.direction, g.trade.status, g.trade.gross_pnl))
        .collect();
    assert_eq!(
        summary,
        [
            (Direction::Long, TradeStatus::Closed, dec!(1)),
            (Direction::Short, TradeStatus::Closed, dec!(-1)),
            (Direction::Long, TradeStatus::Open, dec!(0)),
        ]
    );
}

#[test]
fn flip_splits_fees_proportionally() {
    let mut flip = fut(2, 2, Sell, dec!(3), dec!(100));
    flip.item.fee = dec!(0.3);
    let fills = vec![fut(1, 1, Buy, dec!(1), dec!(90)), flip];
    let r = group_instrument(&fills, &[]).unwrap();
    assert_eq!(r.trades[0].trade.fees, dec!(0.1));
    assert_eq!(r.trades[1].trade.fees, dec!(0.2));
    assert_eq!(r.trades[1].trade.direction, Direction::Short);
    assert_eq!(r.trades[1].trade.qty, dec!(2));
}

#[test]
fn input_order_does_not_matter() {
    let ordered = vec![
        fut(1, 1, Buy, dec!(1), dec!(10)),
        fut(2, 2, Sell, dec!(1), dec!(12)),
    ];
    let shuffled = vec![ordered[1].clone(), ordered[0].clone()];
    assert_eq!(
        group_instrument(&ordered, &[]).unwrap(),
        group_instrument(&shuffled, &[]).unwrap()
    );
}

#[test]
fn spot_sell_without_holdings_is_an_orphan() {
    let fills = vec![
        spot(1, 1, Sell, dec!(0.2), dec!(2000)),
        spot(2, 2, Buy, dec!(1), dec!(2000)),
        spot(3, 3, Sell, dec!(1.5), dec!(2100)),
    ];
    let r = group_instrument(&fills, &[]).unwrap();
    assert_eq!(r.trades.len(), 1);
    assert_eq!(r.trades[0].trade.status, TradeStatus::Closed);
    assert_eq!(r.trades[0].trade.gross_pnl, dec!(100));
    assert_eq!(
        r.orphans,
        [
            Orphan {
                fill_id: 1,
                qty: dec!(0.2)
            },
            Orphan {
                fill_id: 3,
                qty: dec!(0.5)
            },
        ]
    );
}

#[test]
fn funding_is_matched_by_time_inclusive() {
    let fills = vec![
        fut(1, 100, Buy, dec!(1), dec!(10)),
        fut(2, 200, Sell, dec!(1), dec!(10)),
        fut(3, 300, Buy, dec!(1), dec!(10)), // stays open
    ];
    let funding = vec![
        funding_event(10, 50, dec!(-1)),    // before any trade
        funding_event(11, 100, dec!(-2)),   // exactly at open
        funding_event(12, 200, dec!(-3)),   // exactly at close
        funding_event(13, 250, dec!(-4)),   // flat between trades
        funding_event(14, 9999, dec!(0.5)), // during the open trade
    ];
    let r = group_instrument(&fills, &funding).unwrap();
    assert_eq!(r.trades[0].funding_ids, [11, 12]);
    assert_eq!(r.trades[0].trade.funding, dec!(5));
    assert_eq!(r.trades[0].trade.net_pnl, dec!(-5));
    assert_eq!(r.trades[1].funding_ids, [14]);
    assert_eq!(r.trades[1].trade.funding, dec!(-0.5));
    assert_eq!(r.unassigned_funding, [10, 13]);
}

#[test]
fn empty_input_gives_no_trades() {
    let r = group_instrument(&[], &[funding_event(1, 1, dec!(-1))]).unwrap();
    assert!(r.trades.is_empty());
    assert_eq!(r.unassigned_funding, [1]);
}

#[test]
fn mixed_instruments_are_rejected() {
    let mut other = fut(2, 2, Sell, dec!(1), dec!(10));
    other.item.symbol = "ETHUSDT".into();
    let err = group_instrument(&[fut(1, 1, Buy, dec!(1), dec!(10)), other], &[]).unwrap_err();
    assert!(matches!(err, GroupError::MixedInstruments(..)));
}

// ───────────────────────────── overrides ─────────────────────────────

fn rules(merge: &[&str], split: &[&str]) -> Overrides {
    Overrides {
        merge_at: merge.iter().map(|s| s.to_string()).collect(),
        split_at: split.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn merge_rule_joins_same_direction_round_trips() {
    let fills = vec![
        fut(1, 1, Buy, dec!(1), dec!(10)),
        fut(2, 2, Sell, dec!(1), dec!(11)),
        fut(3, 3, Buy, dec!(1), dec!(10)),
        fut(4, 4, Sell, dec!(1), dec!(12)),
    ];
    let r = group_instrument_with(&fills, &[], &rules(&["h3"], &[])).unwrap();
    assert_eq!(r.trades.len(), 1);
    assert_eq!(r.trades[0].trade.gross_pnl, dec!(3));
    assert_eq!(r.trades[0].legs.len(), 4);
}

#[test]
fn merge_rule_is_ignored_across_a_direction_change() {
    let fills = vec![
        fut(1, 1, Buy, dec!(1), dec!(10)),
        fut(2, 2, Sell, dec!(1), dec!(11)),
        fut(3, 3, Sell, dec!(1), dec!(12)), // opens a short
        fut(4, 4, Buy, dec!(1), dec!(11)),
    ];
    let r = group_instrument_with(&fills, &[], &rules(&["h3"], &[])).unwrap();
    assert_eq!(r.trades.len(), 2);
}

#[test]
fn split_rule_at_a_natural_boundary_or_first_fill_changes_nothing() {
    let fills = vec![
        fut(1, 1, Buy, dec!(1), dec!(10)),
        fut(2, 2, Sell, dec!(1), dec!(11)),
        fut(3, 3, Buy, dec!(1), dec!(10)),
        fut(4, 4, Sell, dec!(1), dec!(12)),
    ];
    let plain = group_instrument(&fills, &[]).unwrap();
    let ruled = group_instrument_with(&fills, &[], &rules(&[], &["h1", "h3"])).unwrap();
    assert_eq!(plain, ruled);
}

#[test]
fn split_rule_keeps_total_pnl() {
    let fills = vec![
        fut(1, 1, Buy, dec!(1), dec!(100)),
        fut(2, 2, Sell, dec!(0.5), dec!(110)),
        fut(3, 3, Buy, dec!(1), dec!(90)),
        fut(4, 4, Sell, dec!(1.5), dec!(120)),
    ];
    let total = |r: &GroupResult| -> Decimal { r.trades.iter().map(|g| g.trade.gross_pnl).sum() };
    let plain = group_instrument(&fills, &[]).unwrap();
    let split = group_instrument_with(&fills, &[], &rules(&[], &["h3"])).unwrap();
    assert_eq!(split.trades.len(), 2);
    assert_eq!(total(&split), total(&plain));
    assert_eq!(split.trades[0].trade.gross_pnl, dec!(5)); // realized before the split
}

#[test]
fn symbol_split_knows_common_quotes() {
    assert_eq!(split_symbol("BTCUSDT"), Some(("BTC", "USDT")));
    assert_eq!(split_symbol("ETHBTC"), Some(("ETH", "BTC")));
    assert_eq!(split_symbol("SOLFDUSD"), Some(("SOL", "FDUSD")));
    assert_eq!(split_symbol("USDT"), None);
    assert_eq!(split_symbol("XYZ"), None);
}
