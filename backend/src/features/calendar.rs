//! PnL calendar: closed trades grouped by the day they closed.
//!
//! A trade's result belongs to the day it **closed**, in the chosen time
//! zone. Rust builds the whole month grid (Monday-first weeks, days of the
//! neighbour months included but empty), so the UI only draws it.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{
    DateTime, Datelike, Duration, FixedOffset, Local, LocalResult, NaiveDate, NaiveDateTime,
    Offset, TimeZone,
};
use rusqlite::Connection;
use rusqlite::types::Value;
use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;

use crate::database::{self as db, DbError, settings};
use crate::models::{TimestampMs, Trade, WithId};

const SETTING: &str = "time_zone";
const MS_PER_MIN: i64 = 60_000;

#[derive(Debug, Error)]
pub enum CalendarError {
    #[error("invalid month {year}-{month}")]
    BadMonth { year: i32, month: u32 },
    #[error("invalid date \"{0}\" (use YYYY-MM-DD)")]
    BadDate(String),
    #[error("unknown time zone \"{0}\"")]
    BadZone(String),
    #[error(transparent)]
    Db(#[from] DbError),
}

/// Which clock decides the day a trade belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// The computer's time zone, with its daylight-saving rules.
    Local,
    /// A fixed offset from UTC in minutes (0 = UTC, Binance's day).
    Fixed(i32),
}

impl Zone {
    /// Setting text: "local", "utc" or "+05:30" / "-03:00".
    pub fn parse(s: &str) -> Option<Zone> {
        match s {
            "local" => return Some(Zone::Local),
            "utc" => return Some(Zone::Fixed(0)),
            _ => {}
        }
        let (sign, rest) = match s.as_bytes().first()? {
            b'+' => (1, &s[1..]),
            b'-' => (-1, &s[1..]),
            _ => return None,
        };
        let (h, m) = rest.split_once(':')?;
        if h.len() != 2 || m.len() != 2 {
            return None;
        }
        let (h, m): (i32, i32) = (h.parse().ok()?, m.parse().ok()?);
        let minutes = sign * (h * 60 + m);
        let valid = m % 15 == 0 && m < 60 && (-12 * 60..=14 * 60).contains(&minutes);
        valid.then_some(Zone::Fixed(minutes))
    }

    pub fn value(self) -> String {
        match self {
            Zone::Local => "local".into(),
            Zone::Fixed(0) => "utc".into(),
            Zone::Fixed(m) => offset_text(m),
        }
    }

    pub fn label(self, now: TimestampMs) -> String {
        match self {
            Zone::Local => format!("Computer time (UTC{})", offset_text(local_offset_min(now))),
            Zone::Fixed(0) => "UTC (Binance day)".into(),
            Zone::Fixed(m) => format!("UTC{}", offset_text(m)),
        }
    }

    /// Wall-clock date and time of an instant in this zone.
    pub fn local_time(self, ms: TimestampMs) -> NaiveDateTime {
        let utc = DateTime::from_timestamp_millis(ms).unwrap_or_default();
        match self {
            Zone::Local => utc.with_timezone(&Local).naive_local(),
            Zone::Fixed(m) => utc.with_timezone(&fixed(m)).naive_local(),
        }
    }

    /// The calendar day of an instant.
    pub fn date_of(self, ms: TimestampMs) -> NaiveDate {
        self.local_time(ms).date()
    }

    /// The instant a day starts (local midnight).
    pub fn start_of(self, date: NaiveDate) -> TimestampMs {
        let midnight = date.and_hms_opt(0, 0, 0).unwrap_or_default();
        match self {
            Zone::Fixed(m) => midnight.and_utc().timestamp_millis() - i64::from(m) * MS_PER_MIN,
            Zone::Local => match Local.from_local_datetime(&midnight) {
                LocalResult::Single(t) | LocalResult::Ambiguous(t, _) => t.timestamp_millis(),
                // Clocks jumped over midnight (rare): the day starts an hour later.
                LocalResult::None => Local
                    .from_local_datetime(&(midnight + Duration::hours(1)))
                    .earliest()
                    .map_or(midnight.and_utc().timestamp_millis(), |t| {
                        t.timestamp_millis()
                    }),
            },
        }
    }
}

fn fixed(minutes: i32) -> FixedOffset {
    FixedOffset::east_opt(minutes * 60).unwrap_or(FixedOffset::east_opt(0).unwrap())
}

fn offset_text(minutes: i32) -> String {
    let sign = if minutes < 0 { '-' } else { '+' };
    let m = minutes.abs();
    format!("{sign}{:02}:{:02}", m / 60, m % 60)
}

fn local_offset_min(now: TimestampMs) -> i32 {
    let utc = DateTime::from_timestamp_millis(now).unwrap_or_default();
    utc.with_timezone(&Local).offset().fix().local_minus_utc() / 60
}

pub fn now_ms() -> TimestampMs {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneInfo {
    pub value: String,
    pub label: String,
}

fn info(zone: Zone, now: TimestampMs) -> ZoneInfo {
    ZoneInfo {
        value: zone.value(),
        label: zone.label(now),
    }
}

/// Saved zone; computer time if none is saved or it's no longer valid.
pub fn zone(conn: &Connection) -> Result<Zone, DbError> {
    Ok(settings::get(conn, SETTING)?
        .and_then(|v| v.as_str().and_then(Zone::parse))
        .unwrap_or(Zone::Local))
}

pub fn zone_info(conn: &Connection, now: TimestampMs) -> Result<ZoneInfo, DbError> {
    Ok(info(zone(conn)?, now))
}

pub fn set_zone(
    conn: &Connection,
    value: &str,
    now: TimestampMs,
) -> Result<ZoneInfo, CalendarError> {
    let z = Zone::parse(value).ok_or_else(|| CalendarError::BadZone(value.to_string()))?;
    settings::set(conn, SETTING, &serde_json::Value::String(z.value()))?;
    Ok(info(z, now))
}

/// Choices for the time-zone picker: computer time, UTC, then every
/// real-world offset from UTC−12:00 to UTC+14:00.
pub fn zone_options(now: TimestampMs) -> Vec<ZoneInfo> {
    // Whole and half hours, plus the few :45 zones (Nepal, Chatham...).
    let mut offsets: Vec<i32> = (-24..=28).map(|half| half * 30).collect();
    offsets.extend([345, 525, 765]);
    offsets.sort_unstable();
    let mut out = vec![info(Zone::Local, now), info(Zone::Fixed(0), now)];
    out.extend(
        offsets
            .into_iter()
            .filter(|&m| m != 0)
            .map(|m| info(Zone::Fixed(m), now)),
    );
    out
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    /// "YYYY-MM-DD".
    pub date: String,
    pub day: u32,
    /// False for the neighbour-month days that fill the first/last week.
    pub in_month: bool,
    pub trades: u32,
    pub wins: u32,
    pub losses: u32,
    pub net_pnl: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Week {
    /// Monday to Sunday.
    pub days: Vec<Day>,
    pub trades: u32,
    pub net_pnl: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayRef {
    pub date: String,
    pub net_pnl: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarMonth {
    pub year: i32,
    pub month: u32,
    pub zone: ZoneInfo,
    /// Today's date in that zone, to highlight it.
    pub today: String,
    pub weeks: Vec<Week>,
    pub trades: u32,
    pub net_pnl: Decimal,
    pub trading_days: u32,
    pub green_days: u32,
    pub red_days: u32,
    pub best_day: Option<DayRef>,
    pub worst_day: Option<DayRef>,
    /// Largest |day net| this month, for scaling cell colors.
    pub max_abs_day: Decimal,
    /// Sum of the winning trades' net PnL this month (≥ 0).
    pub profit: Decimal,
    /// Sum of the losing trades' net PnL this month (≤ 0).
    pub loss: Decimal,
    pub wins: u32,
    pub losses: u32,
}

#[derive(Default)]
struct Agg {
    trades: u32,
    wins: u32,
    losses: u32,
    net: Decimal,
}

fn month_start(year: i32, month: u32) -> Result<NaiveDate, CalendarError> {
    if !(1970..=9999).contains(&year) {
        return Err(CalendarError::BadMonth { year, month });
    }
    NaiveDate::from_ymd_opt(year, month, 1).ok_or(CalendarError::BadMonth { year, month })
}

fn next_month(first: NaiveDate) -> NaiveDate {
    let (y, m) = if first.month() == 12 {
        (first.year() + 1, 1)
    } else {
        (first.year(), first.month() + 1)
    };
    NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(first)
}

/// Closed trades with close time in `[from, to)`, oldest close first.
fn closed_between(
    conn: &Connection,
    from: TimestampMs,
    to: TimestampMs,
) -> Result<Vec<WithId<Trade>>, DbError> {
    db::trades::list_where(
        conn,
        "status = ? AND close_time >= ? AND close_time < ? ORDER BY close_time, id",
        &[
            Value::Text("closed".into()),
            Value::Integer(from),
            Value::Integer(to),
        ],
    )
}

/// The month grid for `year`-`month` (1–12).
pub fn month(
    conn: &Connection,
    year: i32,
    month: u32,
    zone: Zone,
    now: TimestampMs,
) -> Result<CalendarMonth, CalendarError> {
    let first = month_start(year, month)?;
    let next = next_month(first);
    let trades = closed_between(conn, zone.start_of(first), zone.start_of(next))?;

    let mut by_day: BTreeMap<NaiveDate, Agg> = BTreeMap::new();
    // Month totals over trades: what the winners made, what the losers lost.
    let (mut profit, mut loss) = (Decimal::ZERO, Decimal::ZERO);
    for t in &trades {
        let Some(close) = t.item.close_time else {
            continue;
        };
        let a = by_day.entry(zone.date_of(close)).or_default();
        a.trades += 1;
        a.net += t.item.net_pnl;
        match t.item.net_pnl.cmp(&Decimal::ZERO) {
            std::cmp::Ordering::Greater => {
                a.wins += 1;
                profit += t.item.net_pnl;
            }
            std::cmp::Ordering::Less => {
                a.losses += 1;
                loss += t.item.net_pnl;
            }
            std::cmp::Ordering::Equal => {}
        }
    }

    // Monday on or before the 1st, through the Sunday on or after the last day.
    let last = next - Duration::days(1);
    let grid_start = first - Duration::days(i64::from(first.weekday().num_days_from_monday()));
    let grid_end = last + Duration::days(i64::from(6 - last.weekday().num_days_from_monday()));

    let mut weeks = Vec::new();
    let mut day = grid_start;
    while day <= grid_end {
        let mut week = Week {
            days: Vec::with_capacity(7),
            trades: 0,
            net_pnl: Decimal::ZERO,
        };
        for _ in 0..7 {
            let in_month = day.month() == month && day.year() == year;
            let a = by_day.get(&day).filter(|_| in_month);
            let (trades, wins, losses, net) = a.map_or((0, 0, 0, Decimal::ZERO), |a| {
                (a.trades, a.wins, a.losses, a.net)
            });
            week.trades += trades;
            week.net_pnl += net;
            week.days.push(Day {
                date: day.to_string(),
                day: day.day(),
                in_month,
                trades,
                wins,
                losses,
                net_pnl: net.normalize(),
            });
            day += Duration::days(1);
        }
        week.net_pnl = week.net_pnl.normalize();
        weeks.push(week);
    }

    let day_ref = |(d, a): (&NaiveDate, &Agg)| DayRef {
        date: d.to_string(),
        net_pnl: a.net.normalize(),
    };
    let best_day = by_day.iter().max_by_key(|(_, a)| a.net).map(day_ref);
    let worst_day = by_day.iter().min_by_key(|(_, a)| a.net).map(day_ref);

    Ok(CalendarMonth {
        year,
        month,
        zone: info(zone, now),
        today: zone.date_of(now).to_string(),
        weeks,
        trades: by_day.values().map(|a| a.trades).sum(),
        net_pnl: by_day.values().map(|a| a.net).sum::<Decimal>().normalize(),
        trading_days: by_day.len() as u32,
        green_days: by_day.values().filter(|a| a.net > Decimal::ZERO).count() as u32,
        red_days: by_day.values().filter(|a| a.net < Decimal::ZERO).count() as u32,
        best_day,
        worst_day,
        max_abs_day: by_day
            .values()
            .map(|a| a.net.abs())
            .max()
            .unwrap_or_default()
            .normalize(),
        profit: profit.normalize(),
        loss: loss.normalize(),
        wins: by_day.values().map(|a| a.wins).sum(),
        losses: by_day.values().map(|a| a.losses).sum(),
    })
}

/// Trades that closed on `date` ("YYYY-MM-DD") in `zone`, oldest first.
pub fn day_trades(
    conn: &Connection,
    date: &str,
    zone: Zone,
) -> Result<Vec<WithId<Trade>>, CalendarError> {
    // chrono also accepts "2026-3-2"; only take the exact YYYY-MM-DD form.
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .ok()
        .filter(|_| date.len() == 10)
        .ok_or_else(|| CalendarError::BadDate(date.to_string()))?;
    let from = zone.start_of(d);
    let to = zone.start_of(d + Duration::days(1));
    Ok(closed_between(conn, from, to)?)
}

#[cfg(test)]
mod tests;
