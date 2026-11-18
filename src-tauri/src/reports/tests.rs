//! Report tests with fake trades. Expected numbers are worked out by hand.

use chrono::NaiveDateTime;
use rust_decimal_macros::dec;

use super::*;
use crate::db::open_in_memory;
use crate::db::tags::Tag;
use crate::db::test_data;

fn utc(s: &str) -> i64 {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M")
        .unwrap()
        .and_utc()
        .timestamp_millis()
}

fn trade(
    symbol: &str,
    market: Market,
    direction: Direction,
    open: &str,
    net: Decimal,
    r: Option<Decimal>,
) -> Trade {
    let o = utc(open);
    Trade {
        symbol: symbol.into(),
        market,
        direction,
        status: TradeStatus::Closed,
        open_time: o,
        close_time: Some(o + 3_600_000),
        net_pnl: net,
        r_multiple: r,
        ..test_data::trade()
    }
}

/// 1: BTC futures long  Mon 09:15  +10 (2R)
/// 2: BTC futures short Mon 14:00  −4  (−1R)
/// 3: ETH spot long     Wed 09:40  +6
/// 4: ETH futures long  Sun 23:30  −2  (Mon 05:00 in UTC+05:30)
/// 5: SOL open trade (left out)
fn sample() -> Vec<WithId<Trade>> {
    use Direction::*;
    use Market::*;
    let open = Trade {
        status: TradeStatus::Open,
        close_time: None,
        ..trade("SOLUSDT", Usdm, Long, "2026-03-03 10:00", dec!(0), None)
    };
    [
        trade(
            "BTCUSDT",
            Usdm,
            Long,
            "2026-03-02 09:15",
            dec!(10),
            Some(dec!(2)),
        ),
        trade(
            "BTCUSDT",
            Usdm,
            Short,
            "2026-03-02 14:00",
            dec!(-4),
            Some(dec!(-1)),
        ),
        trade("ETHUSDT", Spot, Long, "2026-03-04 09:40", dec!(6), None),
        trade("ETHUSDT", Usdm, Long, "2026-03-08 23:30", dec!(-2), None),
        open,
    ]
    .into_iter()
    .enumerate()
    .map(|(i, item)| WithId {
        id: i as i64 + 1,
        item,
    })
    .collect()
}

const UTC: Zone = Zone::Fixed(0);

fn simple(by: GroupBy, zone: Zone) -> Report {
    build(&sample(), by, zone, &Lookup::default(), None)
}

fn nets(r: &Report) -> Vec<(&str, u32, Decimal)> {
    r.rows
        .iter()
        .map(|x| (x.label.as_str(), x.trades, x.net_pnl))
        .collect()
}

#[test]
fn by_symbol_uses_the_stats_formulas() {
    let r = simple(GroupBy::Symbol, UTC);
    assert_eq!(nets(&r), [("BTCUSDT", 2, dec!(6)), ("ETHUSDT", 2, dec!(4))]);
    let btc = &r.rows[0];
    assert_eq!((btc.wins, btc.losses), (1, 1));
    assert_eq!(btc.win_rate, Some(dec!(50)));
    assert_eq!(btc.profit_factor, Some(dec!(2.5))); // 10 / 4
    assert_eq!(btc.avg_pnl, Some(dec!(3))); // 6 / 2
    assert_eq!(btc.avg_r, Some(dec!(0.5))); // (2 − 1) / 2
    assert_eq!((r.total.trades, r.total.net_pnl), (4, dec!(10)));
    assert_eq!(r.open_trades, 1);
}

#[test]
fn by_market_and_side_best_first() {
    assert_eq!(
        nets(&simple(GroupBy::Market, UTC)),
        [("Spot", 1, dec!(6)), ("Futures", 3, dec!(4))]
    );
    let side = simple(GroupBy::Direction, UTC);
    assert_eq!(nets(&side), [("Long", 3, dec!(14)), ("Short", 1, dec!(-4))]);
    assert_eq!(side.rows[0].key, "long");
}

#[test]
fn by_weekday_follows_the_time_zone() {
    let r = simple(GroupBy::Weekday, UTC);
    assert_eq!(r.rows.len(), 7); // every day listed, even empty ones
    let get = |r: &Report, k: &str| {
        let x = r.rows.iter().find(|x| x.key == k).unwrap();
        (x.trades, x.net_pnl)
    };
    assert_eq!(get(&r, "mon"), (2, dec!(6)));
    assert_eq!(get(&r, "wed"), (1, dec!(6)));
    assert_eq!(get(&r, "sun"), (1, dec!(-2)));
    assert_eq!(get(&r, "fri"), (0, dec!(0)));
    assert_eq!(r.rows[0].label, "Monday");

    // Trade 4 opened Sunday 23:30 UTC = Monday 05:00 in UTC+05:30.
    let ist = simple(GroupBy::Weekday, Zone::Fixed(330));
    assert_eq!(get(&ist, "mon"), (3, dec!(4)));
    assert_eq!(get(&ist, "sun"), (0, dec!(0)));
}

#[test]
fn by_hour_lists_all_24() {
    let r = simple(GroupBy::Hour, UTC);
    assert_eq!(r.rows.len(), 24);
    assert_eq!((r.rows[9].trades, r.rows[9].net_pnl), (2, dec!(16))); // 09:15 + 09:40
    assert_eq!(r.rows[14].net_pnl, dec!(-4));
    assert_eq!(r.rows[23].label, "23:00–00:00");
    assert_eq!(r.rows[0].win_rate, None); // empty hour
}

fn tag_setup() -> Lookup {
    let tag = |id, group_id, name: &str| Tag {
        id,
        group_id,
        name: name.into(),
        color: String::new(),
    };
    let groups = vec![
        TagGroup {
            id: 1,
            name: "Setup".into(),
            color: "#396cd8".into(),
            position: 0,
            tags: vec![tag(10, 1, "Breakout"), tag(11, 1, "Pullback")],
        },
        TagGroup {
            id: 2,
            name: "Mistake".into(),
            color: "#cf222e".into(),
            position: 1,
            tags: vec![tag(20, 2, "FOMO")],
        },
    ];
    // Trade 2 has two tags; trade 4 none.
    let links = BTreeMap::from([(1, vec![10]), (2, vec![10, 20]), (3, vec![11])]);
    Lookup {
        tag_groups: groups,
        trade_tags: links,
        ..Lookup::default()
    }
}

#[test]
fn by_tag_counts_a_trade_under_each_of_its_tags() {
    let r = build(&sample(), GroupBy::Tag, UTC, &tag_setup(), None);
    assert_eq!(
        nets(&r),
        [
            ("Setup · Breakout", 2, dec!(6)), // 1 + 2
            ("Setup · Pullback", 1, dec!(6)), // 3 (tie → by name)
            ("Mistake · FOMO", 1, dec!(-4)),  // 2 again
            ("No tag", 1, dec!(-2)),          // 4, always last
        ]
    );
    assert_eq!(r.rows[0].key, "tag:10");
}

#[test]
fn by_tag_within_one_group() {
    let r = build(&sample(), GroupBy::Tag, UTC, &tag_setup(), Some(2));
    // Only Mistake tags; "No tag" = no mistake tagged (trades 1, 3, 4).
    assert_eq!(
        nets(&r),
        [("Mistake · FOMO", 1, dec!(-4)), ("No tag", 3, dec!(14))]
    );
}

#[test]
fn query_applies_filters_and_reads_tags() {
    let conn = open_in_memory().unwrap();
    for t in sample() {
        db::trades::insert(&conn, &t.item).unwrap();
    }
    let g = db::tags::create_group(&conn, "Setup", "#396cd8").unwrap();
    let tag = db::tags::create_tag(&conn, g, "Breakout", "").unwrap();
    db::tags::set_for_trade(&conn, 1, &[tag]).unwrap();

    let all = query(&conn, &TradeQuery::default(), GroupBy::Tag, None).unwrap();
    assert_eq!(
        nets(&all),
        [("Setup · Breakout", 1, dec!(10)), ("No tag", 3, dec!(0))]
    );

    // Opened from Wednesday on: trades 3, 4 (and the open one).
    let q = TradeQuery {
        from: Some(utc("2026-03-04 00:00")),
        ..TradeQuery::default()
    };
    let part = query(&conn, &q, GroupBy::Symbol, None).unwrap();
    assert_eq!(nets(&part), [("ETHUSDT", 2, dec!(4))]);
    assert_eq!(part.open_trades, 0); // SOL opened on the 3rd
}

#[test]
fn by_playbook_with_none_last() {
    let mut trades = sample();
    trades[0].item.playbook_id = Some(7); // +10
    trades[1].item.playbook_id = Some(7); // −4
    trades[2].item.playbook_id = Some(9); // deleted playbook → "No playbook"
    let lookup = Lookup {
        playbooks: BTreeMap::from([(7, "Breakout retest".to_string())]),
        ..Lookup::default()
    };
    let r = build(&trades, GroupBy::Playbook, UTC, &lookup, None);
    assert_eq!(
        nets(&r),
        [("Breakout retest", 2, dec!(6)), ("No playbook", 2, dec!(4))]
    );
    assert_eq!(r.rows[0].key, "playbook:7");
}
