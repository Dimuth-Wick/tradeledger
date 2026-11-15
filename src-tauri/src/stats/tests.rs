//! Stats tests. Every expected number is worked out by hand in the comments.

use rust_decimal_macros::dec;

use super::*;
use crate::db::{self, open_in_memory, test_data};

/// Closed trade (fake): opens at `t`, closes 1000 ms later.
fn closed(t: i64, net: Decimal, r: Option<Decimal>) -> Trade {
    Trade {
        status: TradeStatus::Closed,
        open_time: t,
        close_time: Some(t + 1000),
        gross_pnl: net + dec!(1),
        fees: dec!(0.8),
        funding: dec!(0.2),
        net_pnl: net,
        r_multiple: r,
        ..test_data::trade()
    }
}

fn open(t: i64, net: Decimal) -> Trade {
    Trade {
        status: TradeStatus::Open,
        open_time: t,
        close_time: None,
        net_pnl: net,
        r_multiple: None,
        ..test_data::trade()
    }
}

fn run(trades: &[Trade]) -> Stats {
    let refs: Vec<(i64, &Trade)> = trades
        .iter()
        .enumerate()
        .map(|(i, t)| (i as i64 + 1, t))
        .collect();
    compute(&refs)
}

/// Closed nets in close order: +9 (1.5R), −4 (−1R), +10 (no R), 0 (0R),
/// −2.25 (−0.5R); plus one open trade at −0.5.
fn sample() -> Vec<Trade> {
    vec![
        closed(1_000, dec!(9), Some(dec!(1.5))),
        closed(2_000, dec!(-4), Some(dec!(-1))),
        closed(3_000, dec!(10), None),
        closed(4_000, dec!(0), Some(dec!(0))),
        closed(5_000, dec!(-2.25), Some(dec!(-0.5))),
        open(6_000, dec!(-0.5)),
    ]
}

#[test]
fn counts_and_win_rate() {
    let s = run(&sample());
    assert_eq!((s.trades, s.open, s.closed), (6, 1, 5));
    assert_eq!((s.wins, s.losses, s.breakeven), (2, 2, 1));
    assert_eq!(s.win_rate, Some(dec!(40))); // 2 / 5
}

#[test]
fn pnl_totals_use_closed_trades_only() {
    let s = run(&sample());
    assert_eq!(s.net_pnl, dec!(12.75)); // 9 − 4 + 10 + 0 − 2.25 (open −0.5 excluded)
    assert_eq!(s.gross_pnl, dec!(17.75)); // each closed gross = net + 1
    assert_eq!(s.fees, dec!(4)); // 5 × 0.8
    assert_eq!(s.funding, dec!(1)); // 5 × 0.2
}

#[test]
fn averages_profit_factor_and_expectancy() {
    let s = run(&sample());
    assert_eq!(s.avg_win, Some(dec!(9.5))); // (9 + 10) / 2
    assert_eq!(s.avg_loss, Some(dec!(-3.125))); // (−4 − 2.25) / 2
    assert_eq!(s.largest_win, Some(dec!(10)));
    assert_eq!(s.largest_loss, Some(dec!(-4)));
    assert_eq!(s.payoff_ratio, Some(dec!(3.04))); // 9.5 / 3.125
    assert_eq!(s.profit_factor, Some(dec!(3.04))); // 19 / 6.25
    assert_eq!(s.expectancy, Some(dec!(2.55))); // 12.75 / 5
    assert_eq!(s.avg_r, Some(dec!(0))); // (1.5 − 1 + 0 − 0.5) / 4
    assert_eq!(s.trades_with_r, 4);
    assert_eq!(s.avg_hold_ms, Some(1000));
}

#[test]
fn equity_curve_and_max_drawdown() {
    let s = run(&sample());
    let equity: Vec<Decimal> = s.equity_curve.iter().map(|p| p.equity).collect();
    assert_eq!(equity, [dec!(9), dec!(5), dec!(15), dec!(15), dec!(12.75)]);
    assert_eq!(s.equity_curve[0].time, 2_000); // close time of the first trade
    // Peaks 9 → 9 → 15 → 15 → 15; drops 0, 4, 0, 0, 2.25 → max 4.
    assert_eq!(s.max_drawdown, dec!(4));
    assert_eq!(
        (s.max_drawdown_start, s.max_drawdown_end),
        (Some(2_000), Some(3_000))
    );
}

#[test]
fn drawdown_from_the_start_when_the_first_trade_loses() {
    // −5 then +3: the curve starts at 0, so the drop is 5.
    let s = run(&[closed(1_000, dec!(-5), None), closed(2_000, dec!(3), None)]);
    assert_eq!(s.max_drawdown, dec!(5));
    assert_eq!(
        (s.max_drawdown_start, s.max_drawdown_end),
        (Some(1_000), Some(2_000))
    );
}

#[test]
fn streaks_reset_on_the_other_result_and_on_breakeven() {
    // W W W L L BE L W  → longest win run 3, longest loss run 2.
    let nets = [1, 2, 3, -1, -1, 0, -1, 4];
    let trades: Vec<Trade> = nets
        .iter()
        .enumerate()
        .map(|(i, n)| closed(i as i64 * 10_000, Decimal::from(*n), None))
        .collect();
    let s = run(&trades);
    assert_eq!((s.max_win_streak, s.max_loss_streak), (3, 2));
}

#[test]
fn input_order_does_not_matter() {
    let mut reversed = sample();
    reversed.reverse();
    let (a, b) = (run(&sample()), run(&reversed));
    assert_eq!(a.net_pnl, b.net_pnl);
    assert_eq!(a.max_drawdown, b.max_drawdown);
    let curve = |s: &Stats| s.equity_curve.iter().map(|p| p.equity).collect::<Vec<_>>();
    assert_eq!(curve(&a), curve(&b));
}

#[test]
fn r_distribution_buckets() {
    let rs = [
        dec!(-3),
        dec!(-2),
        dec!(-0.5),
        dec!(0),
        dec!(0.99),
        dec!(1),
        dec!(2.5),
        dec!(3),
        dec!(7),
    ];
    let trades: Vec<Trade> = rs
        .iter()
        .enumerate()
        .map(|(i, r)| closed(i as i64 * 10_000, dec!(1), Some(*r)))
        .collect();
    let s = run(&trades);
    let counts: Vec<(String, u32)> = s
        .r_buckets
        .iter()
        .map(|b| (b.label.clone(), b.count))
        .collect();
    assert_eq!(
        counts,
        [
            ("< -2R".to_string(), 1),      // −3
            ("-2R to -1R".to_string(), 1), // −2 (lower edge inclusive)
            ("-1R to 0R".to_string(), 1),  // −0.5
            ("0R to 1R".to_string(), 2),   // 0, 0.99
            ("1R to 2R".to_string(), 1),   // 1
            ("2R to 3R".to_string(), 1),   // 2.5
            ("≥ 3R".to_string(), 2),       // 3, 7
        ]
    );
}

#[test]
fn no_losses_means_no_profit_factor() {
    let s = run(&[closed(1_000, dec!(5), None), closed(2_000, dec!(3), None)]);
    assert_eq!((s.wins, s.losses), (2, 0));
    assert_eq!(s.profit_factor, None); // shown as ∞ because wins > 0
    assert_eq!(s.payoff_ratio, None);
    assert_eq!(s.max_drawdown, Decimal::ZERO);
}

#[test]
fn empty_and_open_only_sets() {
    let empty = run(&[]);
    assert_eq!((empty.trades, empty.closed), (0, 0));
    assert_eq!(empty.win_rate, None);
    assert_eq!(empty.expectancy, None);
    assert_eq!(empty.net_pnl, Decimal::ZERO);
    assert!(empty.equity_curve.is_empty());
    assert_eq!(empty.r_buckets.len(), 7);

    let only_open = run(&[open(1, dec!(-1))]);
    assert_eq!(
        (only_open.trades, only_open.open, only_open.closed),
        (1, 1, 0)
    );
    assert_eq!(only_open.avg_hold_ms, None);
}

#[test]
fn query_uses_the_trade_log_filters() {
    let conn = open_in_memory().unwrap();
    for t in sample() {
        db::trades::insert(&conn, &t).unwrap();
    }
    let all = query(&conn, &TradeQuery::default()).unwrap();
    assert_eq!(all.net_pnl, dec!(12.75));

    // Opened at 2000..4000 (end exclusive): −4 and +10.
    let q = TradeQuery {
        from: Some(2_000),
        to: Some(4_000),
        ..TradeQuery::default()
    };
    let part = query(&conn, &q).unwrap();
    assert_eq!((part.closed, part.net_pnl), (2, dec!(6)));
    assert_eq!(part.equity_curve.len(), 2);
}
