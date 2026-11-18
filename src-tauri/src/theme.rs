//! Look of the app: light / dark / follow Windows, plus an accent color.
//! Saved in `settings.theme`; the UI only applies what Rust returns.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::db::{DbError, settings};

const KEY: &str = "theme";
pub const DEFAULT_ACCENT: &str = "#f76707";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Follow the Windows light/dark setting.
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub mode: Mode,
    /// "#rrggbb", lowercase.
    pub accent: String,
    /// Text color on accent buttons: black or white, whichever reads better.
    pub on_accent: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccentPreset {
    pub name: &'static str,
    pub hex: &'static str,
}

/// Ready-made accents shown as swatches.
pub const PRESETS: [AccentPreset; 8] = [
    AccentPreset {
        name: "Orange",
        hex: "#f76707",
    },
    AccentPreset {
        name: "Blue",
        hex: "#396cd8",
    },
    AccentPreset {
        name: "Teal",
        hex: "#0f8b8d",
    },
    AccentPreset {
        name: "Green",
        hex: "#2e8540",
    },
    AccentPreset {
        name: "Amber",
        hex: "#d99a00",
    },
    AccentPreset {
        name: "Rose",
        hex: "#d6336c",
    },
    AccentPreset {
        name: "Violet",
        hex: "#7048e8",
    },
    AccentPreset {
        name: "Slate",
        hex: "#5c6b7a",
    },
];

#[derive(Debug, Error)]
pub enum ThemeError {
    #[error("accent must be a color like #396cd8, got \"{0}\"")]
    BadAccent(String),
    #[error(transparent)]
    Db(#[from] DbError),
}

/// "#RRGGBB" → lowercase, or None if it isn't exactly that form.
fn clean_hex(s: &str) -> Option<String> {
    let s = s.trim();
    let hex = s.strip_prefix('#')?;
    (hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit())).then(|| s.to_lowercase())
}

/// WCAG relative luminance of "#rrggbb" (0 = black, 1 = white). Colors are
/// not money, so f64 is fine here.
fn luminance(hex: &str) -> f64 {
    let channel = |i: usize| {
        let v = f64::from(u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0)) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
}

/// Black or white text, whichever has the higher contrast on `hex`.
pub fn on_accent(hex: &str) -> &'static str {
    let l = luminance(hex);
    let with_white = 1.05 / (l + 0.05);
    let with_black = (l + 0.05) / 0.05;
    if with_white >= with_black {
        "#ffffff"
    } else {
        "#000000"
    }
}

fn theme(mode: Mode, accent: String) -> Theme {
    let on = on_accent(&accent).to_string();
    Theme {
        mode,
        accent,
        on_accent: on,
    }
}

pub fn default_theme() -> Theme {
    theme(Mode::System, DEFAULT_ACCENT.to_string())
}

/// Saved theme; the default if none is saved or it's broken.
pub fn load(conn: &Connection) -> Result<Theme, DbError> {
    let saved = settings::get(conn, KEY)?.and_then(|v| {
        let mode: Mode = serde_json::from_value(v.get("mode")?.clone()).ok()?;
        let accent = clean_hex(v.get("accent")?.as_str()?)?;
        Some(theme(mode, accent))
    });
    Ok(saved.unwrap_or_else(default_theme))
}

pub fn save(conn: &Connection, mode: Mode, accent: &str) -> Result<Theme, ThemeError> {
    let accent = clean_hex(accent).ok_or_else(|| ThemeError::BadAccent(accent.to_string()))?;
    settings::set(conn, KEY, &json!({ "mode": mode, "accent": accent }))?;
    Ok(theme(mode, accent))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    #[test]
    fn default_until_saved() {
        let conn = open_in_memory().unwrap();
        let t = load(&conn).unwrap();
        assert_eq!((t.mode, t.accent.as_str()), (Mode::System, "#f76707"));
        assert_eq!(t.on_accent, "#000000"); // orange: dark text reads best
    }

    #[test]
    fn save_round_trip_and_normalize() {
        let conn = open_in_memory().unwrap();
        let t = save(&conn, Mode::Dark, " #D99A00 ").unwrap();
        assert_eq!(t.accent, "#d99a00");
        assert_eq!(load(&conn).unwrap(), t);
    }

    #[test]
    fn rejects_bad_accents_and_keeps_the_old_one() {
        let conn = open_in_memory().unwrap();
        save(&conn, Mode::Light, "#2e8540").unwrap();
        for bad in ["", "396cd8", "#396cd", "#396cd8ff", "#zzzzzz", "blue"] {
            assert!(save(&conn, Mode::Dark, bad).is_err(), "{bad}");
        }
        let t = load(&conn).unwrap();
        assert_eq!((t.mode, t.accent.as_str()), (Mode::Light, "#2e8540"));
    }

    #[test]
    fn broken_saved_value_falls_back() {
        let conn = open_in_memory().unwrap();
        settings::set(&conn, KEY, &json!({ "mode": "neon", "accent": "#396cd8" })).unwrap();
        assert_eq!(load(&conn).unwrap(), default_theme());
        settings::set(&conn, KEY, &json!("dark")).unwrap();
        assert_eq!(load(&conn).unwrap(), default_theme());
    }

    #[test]
    fn text_color_on_accent() {
        assert_eq!(on_accent("#000000"), "#ffffff");
        assert_eq!(on_accent("#ffffff"), "#000000");
        assert_eq!(on_accent("#396cd8"), "#ffffff"); // blue: white text
        assert_eq!(on_accent("#d99a00"), "#000000"); // amber: black text
        assert_eq!(on_accent("#ffd43b"), "#000000"); // yellow
    }

    #[test]
    fn presets_are_valid() {
        for p in PRESETS {
            assert_eq!(clean_hex(p.hex).as_deref(), Some(p.hex), "{}", p.name);
        }
    }
}
