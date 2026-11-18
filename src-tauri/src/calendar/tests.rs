//! Calendar tests with fake trades. March 2026 starts on a Sunday and ends on
//! a Tuesday, so its grid runs Mon 23 Feb → Sun 5 Apr (6 weeks).

use chrono::NaiveDateTime;
use rust_decimal_macros::dec;

use super::*;
use crate::db::{open_in_memory, test_data};
use crate::models::TradeStatus;

/// UTC instant "YYYY-MM-DD HH:MM" in ms.
fn utc(s: &str) -> i64 {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M")
        .unwrap()
        .and_utc()
        .timestamp_millis()
}

fn closed_at(close: &str, net: Decimal) -> Trade {
    let c = utc(close);
    Trade {
        status: TradeStatus::Closed,
        open_time: c - 3_600_000,
        close_time: Some(c),
        net_pnl: net,
        ..test_data::trade()
    }
}

const IST: Zone = Zone::Fixed(330); // UTC+05:30

/// Returns the db and the ids of A..G.
fn setup() -> (Connection, Vec<i64>) {
    let conn = open_in_memory().unwrap();
    let trades = [
        closed_at("2026-03-02 10:00", dec!(10)),  // A
        closed_at("2026-03-02 15:00", dec!(-4)),  // B
        closed_at("2026-03-02 22:30", dec!(5)),   // C: 3 Mar 04:00 in UTC+05:30
        closed_at("2026-02-28 23:00", dec!(100)), // D: 1 Mar 04:30 in UTC+05:30
        closed_at("2026-03-31 20:00", dec!(-7)),  // E: 1 Apr 01:30 in UTC+05:30
        Trade {
            status: TradeStatus::Open, // F: never on the calendar
            close_time: None,
            ..closed_at("2026-03-05 10:00", dec!(50))
        },
        closed_at("2026-03-10 12:00", dec!(0)), // G: breakeven
    ];
    let ids = trades
        .iter()
        .map(|t| db::trades::insert(&conn, t).unwrap())
        .collect();
    (conn, ids)
}

fn day<'a>(m: &'a CalendarMonth, date: &str) -> &'a Day {
    m.weeks
        .iter()
        .flat_map(|w| &w.days)
        .find(|d| d.date == date)
        .unwrap()
}

const NOW: &str = "2026-03-15 20:00";

#[test]
fn month_in_utc() {
    let (conn, _) = setup();
    let m = month(&conn, 2026, 3, Zone::Fixed(0), utc(NOW)).unwrap();

    // Grid shape.
    assert_eq!(m.weeks.len(), 6);
    assert_eq!(m.weeks[0].days[0].date, "2026-02-23");
    assert_eq!(m.weeks[5].days[6].date, "2026-04-05");
    assert!(m.weeks.iter().all(|w| w.days.len() == 7));

    // A + B + C = 10 − 4 + 5.
    let d2 = day(&m, "2026-03-02");
    assert_eq!(
        (d2.trades, d2.wins, d2.losses, d2.net_pnl),
        (3, 2, 1, dec!(11))
    );
    assert_eq!(day(&m, "2026-03-10").trades, 1); // breakeven still counts
    assert_eq!(day(&m, "2026-03-31").net_pnl, dec!(-7));
    assert_eq!(day(&m, "2026-03-05").trades, 0); // open trade F ignored

    // D closed in February: outside the month, and grey days stay empty.
    let feb28 = day(&m, "2026-02-28");
    assert_eq!((feb28.in_month, feb28.trades), (false, 0));

    assert_eq!((m.trades, m.net_pnl), (5, dec!(4))); // 11 + 0 − 7
    assert_eq!((m.trading_days, m.green_days, m.red_days), (3, 1, 1));
    assert_eq!(m.best_day.as_ref().unwrap().date, "2026-03-02");
    assert_eq!(m.worst_day.as_ref().unwrap().net_pnl, dec!(-7));
    assert_eq!(m.max_abs_day, dec!(11));

    // Week totals: Mon 2 – Sun 8 Mar holds A, B, C; the last week holds E.
    let weeks: Vec<(u32, Decimal)> = m.weeks.iter().map(|w| (w.trades, w.net_pnl)).collect();
    assert_eq!(
        weeks,
        [
            (0, dec!(0)),
            (3, dec!(11)),
            (1, dec!(0)),
            (0, dec!(0)),
            (0, dec!(0)),
            (1, dec!(-7))
        ]
    );
    assert_eq!(m.today, "2026-03-15");
    assert_eq!(m.zone.value, "utc");
}

#[test]
fn time_zone_moves_trades_between_days_and_months() {
    let (conn, _) = setup();
    let m = month(&conn, 2026, 3, IST, utc(NOW)).unwrap();
    assert_eq!(day(&m, "2026-03-01").net_pnl, dec!(100)); // D moves in from Feb
    assert_eq!(day(&m, "2026-03-02").net_pnl, dec!(6)); // A + B
    assert_eq!(day(&m, "2026-03-03").net_pnl, dec!(5)); // C moves to the 3rd
    assert_eq!(day(&m, "2026-03-31").trades, 0); // E moves out to April
    assert_eq!((m.trades, m.net_pnl), (5, dec!(111))); // D, A, B, C, G
    assert_eq!(m.weeks[0].net_pnl, dec!(100));
    assert_eq!(m.today, "2026-03-16"); // 20:00 UTC = 01:30 next day
}

#[test]
fn day_trades_use_the_zone() {
    let (conn, ids) = setup();
    let in_utc: Vec<i64> = day_trades(&conn, "2026-03-02", Zone::Fixed(0))
        .unwrap()
        .iter()
        .map(|t| t.id)
        .collect();
    assert_eq!(in_utc, [ids[0], ids[1], ids[2]]);
    let in_ist: Vec<i64> = day_trades(&conn, "2026-03-02", IST)
        .unwrap()
        .iter()
        .map(|t| t.id)
        .collect();
    assert_eq!(in_ist, [ids[0], ids[1]]);
    assert!(day_trades(&conn, "2026-3-2", IST).is_err());
}

#[test]
fn empty_month_and_bad_input() {
    let conn = open_in_memory().unwrap();
    let m = month(&conn, 2026, 2, Zone::Fixed(0), utc(NOW)).unwrap();
    // Feb 2026: Sun 1 → Sat 28, grid Mon 26 Jan → Sun 1 Mar.
    assert_eq!(m.weeks.len(), 5);
    assert_eq!((m.trades, m.trading_days), (0, 0));
    assert_eq!(m.best_day, None);
    assert_eq!(m.max_abs_day, dec!(0));
    assert!(month(&conn, 2026, 13, Zone::Local, 0).is_err());
    assert!(month(&conn, 1969, 1, Zone::Local, 0).is_err());
}

#[test]
fn zone_text_round_trip() {
    for (text, zone) in [
        ("local", Zone::Local),
        ("utc", Zone::Fixed(0)),
        ("+05:30", Zone::Fixed(330)),
        ("-03:00", Zone::Fixed(-180)),
        ("+05:45", Zone::Fixed(345)),
        ("+14:00", Zone::Fixed(840)),
    ] {
        assert_eq!(Zone::parse(text), Some(zone), "{text}");
        assert_eq!(zone.value(), text);
    }
    assert_eq!(Zone::parse("+00:00"), Some(Zone::Fixed(0)));
    for bad in [
        "",
        "Asia/Colombo",
        "+5:30",
        "+05:10",
        "+15:00",
        "-12:30",
        "05:30",
    ] {
        assert_eq!(Zone::parse(bad), None, "{bad}");
    }
    assert_eq!(Zone::Fixed(330).label(0), "UTC+05:30");
}

#[test]
fn zone_setting_saved_and_validated() {
    let conn = open_in_memory().unwrap();
    assert_eq!(zone(&conn).unwrap(), Zone::Local); // default
    let saved = set_zone(&conn, "+05:30", 0).unwrap();
    assert_eq!(saved.value, "+05:30");
    assert_eq!(zone(&conn).unwrap(), IST);
    assert!(set_zone(&conn, "Mars/Olympus", 0).is_err());
    assert_eq!(zone(&conn).unwrap(), IST); // unchanged
}

#[test]
fn zone_options_cover_real_offsets() {
    let opts = zone_options(0);
    let values: Vec<&str> = opts.iter().map(|o| o.value.as_str()).collect();
    assert_eq!(&values[..2], ["local", "utc"]);
    for v in ["-12:00", "+05:30", "+05:45", "+14:00"] {
        assert!(values.contains(&v), "{v}");
    }
    assert_eq!(values.len(), 57); // 2 + 52 half-hour steps + 3 quarter-hour zones
    assert!(opts.iter().all(|o| Zone::parse(&o.value).is_some()));
}
