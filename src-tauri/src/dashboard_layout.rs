//! Dashboard layout: where each widget sits on a 12-column grid.
//!
//! Saved as JSON in `settings` under `dashboard_layout`. Rust owns the list
//! of widgets, their default places and minimum sizes, and checks every saved
//! layout, so a broken or outdated value can never break the dashboard: it
//! falls back to the default, and widgets added in later versions are placed
//! below the saved ones.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::db::{DbError, settings};

const KEY: &str = "dashboard_layout";
/// Grid columns (the UI uses the same number).
pub const COLS: u32 = 12;
/// Tallest widget, in grid rows.
const MAX_H: u32 = 30;
/// Lowest allowed top row; keeps junk values out.
const MAX_Y: u32 = 500;

/// One widget's place and size, in grid units. Same shape as a
/// react-grid-layout item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutItem {
    /// Widget id, e.g. "equity".
    pub i: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    /// Filled in by Rust; ignored when saving.
    #[serde(default)]
    pub min_w: u32,
    #[serde(default)]
    pub min_h: u32,
}

/// A widget with its default place `[x, y, w, h]` and minimum size `[w, h]`.
struct Widget {
    id: &'static str,
    place: [u32; 4],
    min: [u32; 2],
}

const WIDGETS: [Widget; 4] = [
    Widget {
        id: "kpis",
        place: [0, 0, 12, 5],
        min: [3, 3],
    },
    Widget {
        id: "equity",
        place: [0, 5, 12, 7],
        min: [3, 4],
    },
    Widget {
        id: "pnl-bars",
        place: [0, 12, 6, 7],
        min: [3, 4],
    },
    Widget {
        id: "r-dist",
        place: [6, 12, 6, 7],
        min: [3, 4],
    },
];

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("unknown widget \"{0}\"")]
    Unknown(String),
    #[error("widget \"{0}\" appears twice")]
    Duplicate(String),
    #[error("widget \"{0}\" has an invalid size or position")]
    OutOfBounds(String),
    #[error(transparent)]
    Db(#[from] DbError),
}

fn mins(id: &str) -> Option<(u32, u32)> {
    WIDGETS
        .iter()
        .find(|w| w.id == id)
        .map(|w| (w.min[0], w.min[1]))
}

pub fn default_layout() -> Vec<LayoutItem> {
    WIDGETS
        .iter()
        .map(|w| {
            let [x, y, width, height] = w.place;
            LayoutItem {
                i: w.id.to_string(),
                x,
                y,
                w: width,
                h: height,
                min_w: w.min[0],
                min_h: w.min[1],
            }
        })
        .collect()
}

/// Checks a layout and returns it cleaned up: minimum sizes from Rust, and
/// any missing widget added at its default size below the others.
pub fn validate(items: &[LayoutItem]) -> Result<Vec<LayoutItem>, LayoutError> {
    let mut out: Vec<LayoutItem> = Vec::with_capacity(WIDGETS.len());
    for item in items {
        let (min_w, min_h) = mins(&item.i).ok_or_else(|| LayoutError::Unknown(item.i.clone()))?;
        if out.iter().any(|o| o.i == item.i) {
            return Err(LayoutError::Duplicate(item.i.clone()));
        }
        let fits = (min_w..=COLS).contains(&item.w)
            && (min_h..=MAX_H).contains(&item.h)
            && item.x + item.w <= COLS
            && item.y <= MAX_Y;
        if !fits {
            return Err(LayoutError::OutOfBounds(item.i.clone()));
        }
        out.push(LayoutItem {
            min_w,
            min_h,
            ..item.clone()
        });
    }
    let mut bottom = out.iter().map(|o| o.y + o.h).max().unwrap_or(0);
    for d in default_layout() {
        if !out.iter().any(|o| o.i == d.i) {
            let h = d.h;
            out.push(LayoutItem {
                x: 0,
                y: bottom,
                ..d
            });
            bottom += h;
        }
    }
    Ok(out)
}

/// The saved layout, or the default if none is saved or it's no longer valid.
pub fn load(conn: &Connection) -> Result<Vec<LayoutItem>, DbError> {
    let saved = settings::get(conn, KEY)?
        .and_then(|v| serde_json::from_value::<Vec<LayoutItem>>(v).ok())
        .and_then(|items| validate(&items).ok());
    Ok(saved.unwrap_or_else(default_layout))
}

/// Validates and saves; returns what was stored.
pub fn save(conn: &Connection, items: &[LayoutItem]) -> Result<Vec<LayoutItem>, LayoutError> {
    let clean = validate(items)?;
    let value = serde_json::to_value(&clean).map_err(DbError::from)?;
    settings::set(conn, KEY, &value)?;
    Ok(clean)
}

/// Forgets the saved layout; returns the default.
pub fn reset(conn: &Connection) -> Result<Vec<LayoutItem>, DbError> {
    settings::delete(conn, KEY)?;
    Ok(default_layout())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use serde_json::json;

    fn item(i: &str, x: u32, y: u32, w: u32, h: u32) -> LayoutItem {
        LayoutItem {
            i: i.to_string(),
            x,
            y,
            w,
            h,
            min_w: 0,
            min_h: 0,
        }
    }

    #[test]
    fn default_when_nothing_saved() {
        let conn = open_in_memory().unwrap();
        let layout = load(&conn).unwrap();
        assert_eq!(layout, default_layout());
        let ids: Vec<&str> = layout.iter().map(|l| l.i.as_str()).collect();
        assert_eq!(ids, ["kpis", "equity", "pnl-bars", "r-dist"]);
    }

    #[test]
    fn save_then_load_round_trip() {
        let conn = open_in_memory().unwrap();
        let moved = vec![
            item("equity", 0, 0, 8, 8),
            item("kpis", 8, 0, 4, 12),
            item("r-dist", 0, 8, 4, 6),
            item("pnl-bars", 4, 8, 4, 6),
        ];
        let saved = save(&conn, &moved).unwrap();
        assert_eq!(load(&conn).unwrap(), saved);
        // Minimum sizes always come from Rust, not from the UI.
        assert_eq!((saved[1].min_w, saved[1].min_h), (3, 3));
        assert_eq!((saved[0].x, saved[0].w), (0, 8));
    }

    #[test]
    fn rejects_bad_layouts() {
        let conn = open_in_memory().unwrap();
        let cases = [
            (vec![item("pie", 0, 0, 4, 4)], "unknown widget \"pie\""),
            (
                vec![item("kpis", 0, 0, 4, 4), item("kpis", 4, 0, 4, 4)],
                "widget \"kpis\" appears twice",
            ),
            (vec![item("kpis", 10, 0, 4, 4)], "invalid size"), // past column 12
            (vec![item("equity", 0, 0, 2, 4)], "invalid size"), // narrower than min 3
            (vec![item("equity", 0, 0, 4, 31)], "invalid size"), // taller than 30
        ];
        for (items, msg) in cases {
            let err = save(&conn, &items).unwrap_err().to_string();
            assert!(err.contains(msg), "{err}");
        }
        // Nothing bad was stored.
        assert_eq!(load(&conn).unwrap(), default_layout());
    }

    #[test]
    fn missing_widgets_are_added_below() {
        // An older saved layout without the R chart.
        let clean = validate(&[item("kpis", 0, 0, 12, 6), item("equity", 0, 6, 6, 5)]).unwrap();
        let tail: Vec<(&str, u32, u32)> = clean[2..]
            .iter()
            .map(|l| (l.i.as_str(), l.x, l.y))
            .collect();
        // Bottom of the saved ones is 6 + 5 = 11; each new one stacks below.
        assert_eq!(tail, [("pnl-bars", 0, 11), ("r-dist", 0, 18)]);
    }

    #[test]
    fn broken_saved_value_falls_back_to_default() {
        let conn = open_in_memory().unwrap();
        settings::set(&conn, KEY, &json!({ "not": "a list" })).unwrap();
        assert_eq!(load(&conn).unwrap(), default_layout());
        settings::set(
            &conn,
            KEY,
            &json!([{ "i": "gone", "x": 0, "y": 0, "w": 4, "h": 4 }]),
        )
        .unwrap();
        assert_eq!(load(&conn).unwrap(), default_layout());
    }

    #[test]
    fn reset_forgets_the_saved_layout() {
        let conn = open_in_memory().unwrap();
        save(&conn, &[item("equity", 0, 0, 4, 4)]).unwrap();
        assert_ne!(load(&conn).unwrap(), default_layout());
        assert_eq!(reset(&conn).unwrap(), default_layout());
        assert_eq!(load(&conn).unwrap(), default_layout());
    }
}
