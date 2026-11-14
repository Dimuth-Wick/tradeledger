//! Schema migrations, oldest first.
//!
//! Rules:
//! - Never edit or reorder a migration that has shipped; users' DBs already
//!   ran it. Add a new one at the end instead.
//! - SQLite's `PRAGMA user_version` stores how many have been applied, so on
//!   startup only the new ones run.

use rusqlite_migration::{M, Migrations};

/// Bump together with adding a migration below; tests check they agree.
/// Test-only: the app itself just runs `to_latest()` and needn't know the number.
#[cfg(test)]
pub const LATEST_VERSION: u32 = 1;

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        // v1: key → JSON value store for preferences (theme, dashboard layout…).
        // The trading tables arrive in v2 (task T2.2).
        M::up(
            "CREATE TABLE settings (
                key   TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            ) STRICT;",
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Catches SQL syntax errors and broken ordering in every migration.
    #[test]
    fn migrations_are_valid() {
        migrations().validate().unwrap();
    }

    #[test]
    fn latest_version_matches_migration_count() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        migrations().to_latest(&mut conn).unwrap();
        let v: u32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, LATEST_VERSION);
    }
}
