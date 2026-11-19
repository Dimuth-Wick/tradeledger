//! Key → JSON value preferences (theme, dashboard layout, time zone…).

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use super::DbError;

/// `None` if the key was never set.
pub fn get(conn: &Connection, key: &str) -> Result<Option<Value>, DbError> {
    let text: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(text.map(|t| serde_json::from_str(&t)).transpose()?)
}

/// Insert or overwrite.
pub fn set(conn: &Connection, key: &str, value: &Value) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value.to_string()],
    )?;
    Ok(())
}

/// Removes the key (no error if it wasn't set).
pub fn delete(conn: &Connection, key: &str) -> Result<(), DbError> {
    conn.execute("DELETE FROM settings WHERE key = ?1", [key])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::open_in_memory;
    use serde_json::json;

    #[test]
    fn missing_key_is_none() {
        let conn = open_in_memory().unwrap();
        assert_eq!(get(&conn, "theme").unwrap(), None);
    }

    #[test]
    fn delete_removes_the_key() {
        let conn = open_in_memory().unwrap();
        set(&conn, "theme", &json!("dark")).unwrap();
        delete(&conn, "theme").unwrap();
        delete(&conn, "theme").unwrap(); // second time is a no-op
        assert_eq!(get(&conn, "theme").unwrap(), None);
    }

    #[test]
    fn set_then_overwrite() {
        let conn = open_in_memory().unwrap();
        set(&conn, "theme", &json!("dark")).unwrap();
        set(
            &conn,
            "theme",
            &json!({ "mode": "light", "accent": "#396cd8" }),
        )
        .unwrap();
        assert_eq!(
            get(&conn, "theme").unwrap(),
            Some(json!({ "mode": "light", "accent": "#396cd8" }))
        );
    }
}
