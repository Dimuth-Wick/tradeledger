//! Trade log query tests (in-memory DB, fake trades).

use rust_decimal_macros::dec;

use super::*;
use crate::database::{open_in_memory, test_data};

/// Which instrument and side a fake trade is on.
type Instr<'a> = (&'a str, Market, Direction);

/// Inserts a fake trade; returns its id.
fn add(
    conn: &Connection,
    (symbol, market, direction): Instr<'_>,
    open_time: i64,
    net: Decimal,
    closed: bool,
    r: Option<Decimal>,
) -> i64 {
    let t = Trade {
        symbol: symbol.into(),
        market,
        direction,
        open_time,
        close_time: closed.then_some(open_time + 1000),
        status: if closed {
            TradeStatus::Closed
        } else {
            TradeStatus::Open
        },
        avg_exit: closed.then_some(dec!(1)),
        net_pnl: net,
        r_multiple: r,
        ..test_data::trade()
    };
    db::trades::insert(conn, &t).unwrap()
}

/// 6 trades over 6 days (day n opens at n × 86_400_000 ms).
fn sample(conn: &Connection) -> Vec<i64> {
    const DAY: i64 = 86_400_000;
    use Direction::{Long, Short};
    use Market::{Spot, Usdm};
    vec![
        add(
            conn,
            ("BTCUSDT", Usdm, Long),
            DAY,
            dec!(9),
            true,
            Some(dec!(1.5)),
        ),
        add(
            conn,
            ("BTCUSDT", Usdm, Short),
            2 * DAY,
            dec!(-4),
            true,
            Some(dec!(-1)),
        ),
        add(conn, ("ETHUSDT", Spot, Long), 3 * DAY, dec!(10), true, None),
        add(
            conn,
            ("ETHUSDT", Usdm, Short),
            4 * DAY,
            dec!(0),
            true,
            Some(dec!(0)),
        ),
        add(
            conn,
            ("SOLUSDT", Spot, Long),
            5 * DAY,
            dec!(-0.5),
            false,
            None,
        ),
        add(
            conn,
            ("BTCUSDT", Spot, Long),
            6 * DAY,
            dec!(-2.25),
            true,
            Some(dec!(-0.5)),
        ),
    ]
}

fn ids(page: &TradePage) -> Vec<i64> {
    page.items.iter().map(|t| t.id).collect()
}

fn run(conn: &Connection, q: TradeQuery) -> TradePage {
    query(conn, &q).unwrap()
}

#[test]
fn default_query_is_newest_first_with_totals() {
    let conn = open_in_memory().unwrap();
    let id = sample(&conn);
    let p = run(&conn, TradeQuery::default());
    assert_eq!(ids(&p), [id[5], id[4], id[3], id[2], id[1], id[0]]);
    assert_eq!((p.total, p.page, p.page_count), (6, 0, 1));
    // Closed only: 9 − 4 + 10 + 0 − 2.25 (the open −0.5 is excluded).
    assert_eq!(p.closed_net_pnl, dec!(12.75));
    assert_eq!((p.wins, p.losses), (2, 2));
}

#[test]
fn filters_by_date_range_inclusive_start_exclusive_end() {
    let conn = open_in_memory().unwrap();
    let id = sample(&conn);
    const DAY: i64 = 86_400_000;
    let p = run(
        &conn,
        TradeQuery {
            from: Some(2 * DAY),
            to: Some(4 * DAY),
            ..TradeQuery::default()
        },
    );
    assert_eq!(ids(&p), [id[2], id[1]]);
}

#[test]
fn symbol_filter_is_partial_and_case_insensitive() {
    let conn = open_in_memory().unwrap();
    sample(&conn);
    let p = run(
        &conn,
        TradeQuery {
            symbol: Some(" btc ".into()),
            ..TradeQuery::default()
        },
    );
    assert_eq!(p.total, 3);
    assert!(p.items.iter().all(|t| t.item.symbol == "BTCUSDT"));

    // Special characters are matched literally, not as wildcards.
    let none = run(
        &conn,
        TradeQuery {
            symbol: Some("%".into()),
            ..TradeQuery::default()
        },
    );
    assert_eq!(none.total, 0);
}

#[test]
fn market_side_and_status_filters_combine() {
    let conn = open_in_memory().unwrap();
    let id = sample(&conn);
    let p = run(
        &conn,
        TradeQuery {
            market: Some(Market::Spot),
            direction: Some(Direction::Long),
            status: Some(TradeStatus::Closed),
            ..TradeQuery::default()
        },
    );
    assert_eq!(ids(&p), [id[5], id[2]]);
}

#[test]
fn outcome_filter_uses_net_pnl_of_closed_trades() {
    let conn = open_in_memory().unwrap();
    let id = sample(&conn);
    let q = |o| TradeQuery {
        outcome: Some(o),
        ..TradeQuery::default()
    };
    assert_eq!(ids(&run(&conn, q(Outcome::Win))), [id[2], id[0]]);
    // The open SOL trade (−0.5 so far) is not a loss yet.
    assert_eq!(ids(&run(&conn, q(Outcome::Loss))), [id[5], id[1]]);
    assert_eq!(ids(&run(&conn, q(Outcome::Breakeven))), [id[3]]);
}

#[test]
fn sorts_by_net_pnl_exactly() {
    let conn = open_in_memory().unwrap();
    let id = sample(&conn);
    let p = run(
        &conn,
        TradeQuery {
            sort: SortField::NetPnl,
            descending: true,
            ..TradeQuery::default()
        },
    );
    // 10 > 9 > 0 > −0.5 > −2.25 > −4  (as text, "9" would beat "10").
    assert_eq!(ids(&p), [id[2], id[0], id[3], id[4], id[5], id[1]]);
}

#[test]
fn missing_values_sort_last_in_both_directions() {
    let conn = open_in_memory().unwrap();
    let id = sample(&conn);
    for descending in [true, false] {
        let p = run(
            &conn,
            TradeQuery {
                sort: SortField::RMultiple,
                descending,
                ..TradeQuery::default()
            },
        );
        let got = ids(&p);
        // ETH spot and SOL have no R → always at the end (newest first).
        assert_eq!(&got[4..], [id[4], id[2]], "descending={descending}");
    }
    let asc = run(
        &conn,
        TradeQuery {
            sort: SortField::RMultiple,
            descending: false,
            ..TradeQuery::default()
        },
    );
    assert_eq!(&ids(&asc)[..4], [id[1], id[5], id[3], id[0]]);
}

#[test]
fn symbol_sort_breaks_ties_newest_first() {
    let conn = open_in_memory().unwrap();
    let id = sample(&conn);
    let p = run(
        &conn,
        TradeQuery {
            sort: SortField::Symbol,
            descending: false,
            ..TradeQuery::default()
        },
    );
    assert_eq!(ids(&p), [id[5], id[1], id[0], id[3], id[2], id[4]]);
}

#[test]
fn paginates_and_clamps_past_the_end() {
    let conn = open_in_memory().unwrap();
    let id = sample(&conn);
    let page = |n| TradeQuery {
        page: n,
        page_size: 4,
        ..TradeQuery::default()
    };
    let p0 = run(&conn, page(0));
    assert_eq!(
        (ids(&p0), p0.page_count),
        (vec![id[5], id[4], id[3], id[2]], 2)
    );
    let p1 = run(&conn, page(1));
    assert_eq!(ids(&p1), [id[1], id[0]]);
    // Totals cover every page, not just the visible one.
    assert_eq!(p1.closed_net_pnl, dec!(12.75));

    let past = run(&conn, page(9));
    assert_eq!(past.page, 1, "clamped to the last page");
    assert_eq!(ids(&past), [id[1], id[0]]);

    let zero_size = run(
        &conn,
        TradeQuery {
            page_size: 0,
            ..TradeQuery::default()
        },
    );
    assert_eq!(zero_size.page_size, 1);
}

#[test]
fn empty_result_has_one_empty_page() {
    let conn = open_in_memory().unwrap();
    let p = run(&conn, TradeQuery::default());
    assert_eq!((p.total, p.page, p.page_count), (0, 0, 1));
    assert!(p.items.is_empty());
}

#[test]
fn tag_filter_matches_any_selected_tag() {
    let conn = open_in_memory().unwrap();
    let id = sample(&conn);
    conn.execute_batch(&format!(
        "INSERT INTO tag_groups (id, name) VALUES (1, 'Setup');
         INSERT INTO tags (id, group_id, name) VALUES (1, 1, 'Breakout'), (2, 1, 'Pullback'), (3, 1, 'Range');
         INSERT INTO trade_tags (trade_id, tag_id) VALUES ({a}, 1), ({b}, 2), ({a}, 2);",
        a = id[0],
        b = id[3]
    ))
    .unwrap();
    let p = run(
        &conn,
        TradeQuery {
            tag_ids: vec![1, 2],
            ..TradeQuery::default()
        },
    );
    assert_eq!(
        ids(&p),
        [id[3], id[0]],
        "each trade once, even with two matching tags"
    );
    let none = run(
        &conn,
        TradeQuery {
            tag_ids: vec![3],
            ..TradeQuery::default()
        },
    );
    assert_eq!(none.total, 0);
}

#[test]
fn query_deserializes_from_ui_json_with_defaults() {
    let q: TradeQuery = serde_json::from_value(serde_json::json!({
        "symbol": "eth",
        "outcome": "win",
        "sort": "netPnl"
    }))
    .unwrap();
    assert_eq!(q.outcome, Some(Outcome::Win));
    assert_eq!(q.sort, SortField::NetPnl);
    assert!(q.descending);
    assert_eq!((q.page, q.page_size), (0, DEFAULT_PAGE_SIZE));
}
