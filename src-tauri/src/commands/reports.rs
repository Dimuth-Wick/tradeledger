//! Reports command.

use tauri::State;

use crate::db::Db;
use crate::reports::{self, GroupBy, Report};
use crate::trade_query::TradeQuery;

/// Stats per group for the trades matching `query` (sort/page ignored).
/// `tag_group` limits a tag report to one tag group.
#[tauri::command]
pub fn get_report(
    db: State<'_, Db>,
    query: TradeQuery,
    group_by: GroupBy,
    tag_group: Option<i64>,
) -> Result<Report, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    reports::query(&conn, &query, group_by, tag_group).map_err(|e| e.to_string())
}
