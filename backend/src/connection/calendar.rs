//! PnL calendar and time-zone commands.

use std::fmt::Display;

use rusqlite::Connection;
use tauri::State;

use crate::database::Db;
use crate::features::calendar::{self, CalendarMonth, ZoneInfo};
use crate::models::{Trade, WithId};

fn with_conn<T, E: Display>(
    db: &Db,
    f: impl FnOnce(&Connection) -> Result<T, E>,
) -> Result<T, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    f(&conn).map_err(|e| e.to_string())
}

/// Month grid in the saved time zone. `month` is 1–12.
#[tauri::command]
pub fn get_calendar_month(
    db: State<'_, Db>,
    year: i32,
    month: u32,
) -> Result<CalendarMonth, String> {
    with_conn(&db, |c| {
        let zone = calendar::zone(c)?;
        calendar::month(c, year, month, zone, calendar::now_ms())
    })
}

/// Trades that closed on `date` ("YYYY-MM-DD") in the saved time zone.
#[tauri::command]
pub fn get_day_trades(db: State<'_, Db>, date: String) -> Result<Vec<WithId<Trade>>, String> {
    with_conn(&db, |c| {
        let zone = calendar::zone(c)?;
        calendar::day_trades(c, &date, zone)
    })
}

#[tauri::command]
pub fn get_time_zone(db: State<'_, Db>) -> Result<ZoneInfo, String> {
    with_conn(&db, |c| calendar::zone_info(c, calendar::now_ms()))
}

#[tauri::command]
pub fn set_time_zone(db: State<'_, Db>, value: String) -> Result<ZoneInfo, String> {
    with_conn(&db, |c| calendar::set_zone(c, &value, calendar::now_ms()))
}

#[tauri::command]
pub fn list_time_zones() -> Result<Vec<ZoneInfo>, String> {
    Ok(calendar::zone_options(calendar::now_ms()))
}
