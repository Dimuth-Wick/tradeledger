//! Milestone badges for journaling: 50, 100, 300 … 5,000 trades logged.
//!
//! ```text
//! trades logged ──► reached badges ──► any above "last celebrated"?
//!                                         │ yes
//!                                         ▼
//!                                  pop-up in the app ──► mark_seen
//! ```
//!
//! Every stored trade counts (open or closed, typed in or imported). The
//! highest celebrated level is kept in `settings`, so each badge is
//! celebrated once, even if trades are deleted and added again.

use rusqlite::Connection;
use serde::Serialize;
use serde_json::json;

use crate::database::{self as db, DbError, settings};

const SEEN_KEY: &str = "milestones_seen";

/// Trades needed, badge name, one line shown in the pop-up.
pub const LEVELS: [(u32, &str, &str); 7] = [
    (
        50,
        "Getting started",
        "50 trades in your journal. The habit is forming.",
    ),
    (
        100,
        "Committed",
        "100 trades logged. You have real data to learn from now.",
    ),
    (
        300,
        "Disciplined",
        "300 trades. Patterns in your trading start to show.",
    ),
    (
        500,
        "Seasoned",
        "500 trades. Your stats mean something now.",
    ),
    (
        1000,
        "Veteran",
        "1,000 trades journaled. Few traders get this far.",
    ),
    (
        2000,
        "Elite",
        "2,000 trades. A serious record of your trading.",
    ),
    (5000, "Legend", "5,000 trades. The full journal. Respect."),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Milestone {
    pub trades: u32,
    pub name: String,
    pub message: String,
    pub reached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// Trades logged now.
    pub trades: u32,
    pub milestones: Vec<Milestone>,
    /// The next level to reach; `None` once every badge is earned.
    pub next: Option<u32>,
    /// Percent 0–100 of the way from the previous level to `next`.
    pub percent: u32,
    /// Levels reached but not celebrated yet (lowest first).
    pub unseen: Vec<u32>,
}

/// Pure: the progress for `trades` logged and `seen` = highest level
/// already celebrated.
pub fn progress(trades: u32, seen: u32) -> Progress {
    let milestones: Vec<Milestone> = LEVELS
        .iter()
        .map(|&(level, name, message)| Milestone {
            trades: level,
            name: name.to_string(),
            message: message.to_string(),
            reached: trades >= level,
        })
        .collect();
    let next = LEVELS.iter().map(|l| l.0).find(|&l| trades < l);
    let prev = LEVELS
        .iter()
        .map(|l| l.0)
        .filter(|&l| trades >= l)
        .max()
        .unwrap_or(0);
    let percent = next.map_or(100, |n| (trades - prev) * 100 / (n - prev));
    let unseen = LEVELS
        .iter()
        .map(|l| l.0)
        .filter(|&l| trades >= l && l > seen)
        .collect();
    Progress {
        trades,
        milestones,
        next,
        percent,
        unseen,
    }
}

fn seen(conn: &Connection) -> Result<u32, DbError> {
    Ok(settings::get(conn, SEEN_KEY)?
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0))
}

pub fn load(conn: &Connection) -> Result<Progress, DbError> {
    Ok(progress(db::trades::count(conn)?, seen(conn)?))
}

/// Marks every badge reached so far as celebrated.
pub fn mark_seen(conn: &Connection) -> Result<Progress, DbError> {
    let now = load(conn)?;
    let highest = now.unseen.iter().copied().max().unwrap_or(0);
    if highest > seen(conn)? {
        settings::set(conn, SEEN_KEY, &json!(highest))?;
    }
    load(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::open_in_memory;

    #[test]
    fn progress_between_levels() {
        let p = progress(73, 0);
        assert_eq!(p.next, Some(100));
        // 73 is 23 of the 50 steps from 50 to 100.
        assert_eq!(p.percent, 46);
        assert_eq!(p.unseen, [50]);
        let reached: Vec<u32> = p
            .milestones
            .iter()
            .filter(|m| m.reached)
            .map(|m| m.trades)
            .collect();
        assert_eq!(reached, [50]);
    }

    #[test]
    fn start_and_end() {
        let p = progress(0, 0);
        assert_eq!((p.next, p.percent), (Some(50), 0));
        assert!(p.unseen.is_empty());

        let p = progress(5200, 2000);
        assert_eq!((p.next, p.percent), (None, 100));
        assert_eq!(p.unseen, [5000]);
        assert!(p.milestones.iter().all(|m| m.reached));
    }

    #[test]
    fn a_big_import_unlocks_several_at_once() {
        assert_eq!(progress(320, 50).unseen, [100, 300]);
        assert_eq!(progress(100, 100).unseen, Vec::<u32>::new());
        // Exactly on a level counts as reached.
        assert_eq!(progress(500, 300).unseen, [500]);
    }

    #[test]
    fn seen_is_remembered() {
        let conn = open_in_memory().unwrap();
        assert_eq!(load(&conn).unwrap().trades, 0);
        settings::set(&conn, SEEN_KEY, &json!(100)).unwrap();
        assert_eq!(seen(&conn).unwrap(), 100);
        // Nothing reached: marking keeps the saved level.
        mark_seen(&conn).unwrap();
        assert_eq!(seen(&conn).unwrap(), 100);
        // A broken value counts as nothing seen.
        settings::set(&conn, SEEN_KEY, &json!("x")).unwrap();
        assert_eq!(seen(&conn).unwrap(), 0);
    }
}
