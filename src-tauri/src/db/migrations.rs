//! Schema migrations, oldest first.
//!
//! Rules:
//! - Never edit or reorder a migration that has shipped; users' DBs already
//!   ran it. Add a new one at the end instead.
//! - SQLite's `PRAGMA user_version` stores how many have been applied, so on
//!   startup only the new ones run.
//!
//! Storage conventions (all tables are STRICT, so SQLite enforces types):
//! - Money / quantities / prices: TEXT holding a `rust_decimal` string
//!   ("0.00012345"). Exact, unlike REAL (floating point).
//! - Times: INTEGER milliseconds since 1970-01-01 UTC. Time zones are a
//!   display concern, handled in the UI layer.
//! - Enums: TEXT with a CHECK list, so a typo can never be stored.
//! - Booleans: INTEGER 0/1 with a CHECK.

use rusqlite_migration::{M, Migrations};

/// Bump together with adding a migration below; tests check they agree.
/// Test-only: the app itself just runs `to_latest()` and needn't know the number.
#[cfg(test)]
pub const LATEST_VERSION: u32 = 2;

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        // v1: key → JSON value store for preferences (theme, dashboard layout…).
        M::up(
            "CREATE TABLE settings (
                key   TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            ) STRICT;",
        ),
        // v2: trading data (fills, trades, funding, tags, journal tables).
        M::up(V2_TRADING_SCHEMA),
    ])
}

const V2_TRADING_SCHEMA: &str = "
-- One row per imported file, so the user can see what came from where.
CREATE TABLE import_batches (
    id             INTEGER PRIMARY KEY,
    exchange       TEXT    NOT NULL,
    market         TEXT    NOT NULL CHECK (market IN ('spot', 'usdm')),
    file_name      TEXT    NOT NULL,
    imported_at    INTEGER NOT NULL,
    rows_total     INTEGER NOT NULL DEFAULT 0,
    rows_inserted  INTEGER NOT NULL DEFAULT 0,
    rows_duplicate INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE playbooks (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT ''
) STRICT;

CREATE TABLE playbook_rules (
    id          INTEGER PRIMARY KEY,
    playbook_id INTEGER NOT NULL REFERENCES playbooks(id) ON DELETE CASCADE,
    text        TEXT    NOT NULL,
    position    INTEGER NOT NULL DEFAULT 0
) STRICT;

-- A trade = one position from open to flat (built from fills, or manual).
CREATE TABLE trades (
    id            INTEGER PRIMARY KEY,
    exchange      TEXT    NOT NULL,
    market        TEXT    NOT NULL CHECK (market IN ('spot', 'usdm')),
    symbol        TEXT    NOT NULL,
    direction     TEXT    NOT NULL CHECK (direction IN ('long', 'short')),
    status        TEXT    NOT NULL CHECK (status IN ('open', 'closed')),
    open_time     INTEGER NOT NULL,
    close_time    INTEGER,
    qty           TEXT    NOT NULL,
    avg_entry     TEXT    NOT NULL,
    avg_exit      TEXT,
    leverage      INTEGER,
    gross_pnl     TEXT    NOT NULL DEFAULT '0',
    fees          TEXT    NOT NULL DEFAULT '0',
    funding       TEXT    NOT NULL DEFAULT '0',
    net_pnl       TEXT    NOT NULL DEFAULT '0',
    planned_entry TEXT,
    stop_loss     TEXT,
    take_profit   TEXT,
    planned_rr    TEXT,
    r_multiple    TEXT,
    notes         TEXT    NOT NULL DEFAULT '',
    -- Deleting a playbook keeps its trades, just unlinked.
    playbook_id   INTEGER REFERENCES playbooks(id) ON DELETE SET NULL,
    is_manual     INTEGER NOT NULL DEFAULT 0 CHECK (is_manual IN (0, 1))
) STRICT;
CREATE INDEX idx_trades_open_time ON trades(open_time);
CREATE INDEX idx_trades_symbol    ON trades(exchange, market, symbol);

-- Raw executions from the exchange. dedupe_hash makes re-importing the same
-- CSV harmless: a second copy of a row is rejected by the UNIQUE index.
CREATE TABLE fills (
    id              INTEGER PRIMARY KEY,
    exchange        TEXT    NOT NULL,
    market          TEXT    NOT NULL CHECK (market IN ('spot', 'usdm')),
    symbol          TEXT    NOT NULL,
    side            TEXT    NOT NULL CHECK (side IN ('buy', 'sell')),
    qty             TEXT    NOT NULL,
    price           TEXT    NOT NULL,
    fee             TEXT    NOT NULL DEFAULT '0',
    fee_asset       TEXT    NOT NULL DEFAULT '',
    realized_pnl    TEXT,
    time            INTEGER NOT NULL,
    dedupe_hash     TEXT    NOT NULL UNIQUE,
    -- Deleting a trade un-groups its fills (they stay, ready to regroup).
    trade_id        INTEGER REFERENCES trades(id) ON DELETE SET NULL,
    import_batch_id INTEGER REFERENCES import_batches(id) ON DELETE SET NULL
) STRICT;
CREATE INDEX idx_fills_grouping ON fills(exchange, market, symbol, time);
CREATE INDEX idx_fills_trade    ON fills(trade_id);

-- Futures funding payments (negative = paid, positive = received).
CREATE TABLE funding_events (
    id              INTEGER PRIMARY KEY,
    exchange        TEXT    NOT NULL,
    symbol          TEXT    NOT NULL,
    amount          TEXT    NOT NULL,
    time            INTEGER NOT NULL,
    dedupe_hash     TEXT    NOT NULL UNIQUE,
    trade_id        INTEGER REFERENCES trades(id) ON DELETE SET NULL,
    import_batch_id INTEGER REFERENCES import_batches(id) ON DELETE SET NULL
) STRICT;
CREATE INDEX idx_funding_trade ON funding_events(trade_id);

-- User-defined tags, e.g. group 'Mistake' → tags 'FOMO', 'Moved stop'.
CREATE TABLE tag_groups (
    id       INTEGER PRIMARY KEY,
    name     TEXT    NOT NULL UNIQUE,
    color    TEXT    NOT NULL DEFAULT '',
    position INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE tags (
    id       INTEGER PRIMARY KEY,
    group_id INTEGER NOT NULL REFERENCES tag_groups(id) ON DELETE CASCADE,
    name     TEXT    NOT NULL,
    color    TEXT    NOT NULL DEFAULT '',
    UNIQUE (group_id, name)
) STRICT;

-- Link tables: rows vanish automatically when either side is deleted.
CREATE TABLE trade_tags (
    trade_id INTEGER NOT NULL REFERENCES trades(id) ON DELETE CASCADE,
    tag_id   INTEGER NOT NULL REFERENCES tags(id)   ON DELETE CASCADE,
    PRIMARY KEY (trade_id, tag_id)
) STRICT, WITHOUT ROWID;

-- Only the file name is stored; the image lives in the app-data folder.
CREATE TABLE screenshots (
    id         INTEGER PRIMARY KEY,
    trade_id   INTEGER NOT NULL REFERENCES trades(id) ON DELETE CASCADE,
    file_name  TEXT    NOT NULL,
    caption    TEXT    NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE custom_fields (
    id         INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL UNIQUE,
    field_type TEXT    NOT NULL CHECK (field_type IN ('text', 'number', 'select', 'checkbox')),
    -- JSON array of choices for 'select' fields.
    options    TEXT    NOT NULL DEFAULT '[]',
    position   INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE custom_values (
    trade_id INTEGER NOT NULL REFERENCES trades(id)        ON DELETE CASCADE,
    field_id INTEGER NOT NULL REFERENCES custom_fields(id) ON DELETE CASCADE,
    value    TEXT    NOT NULL,
    PRIMARY KEY (trade_id, field_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE playbook_checks (
    trade_id INTEGER NOT NULL REFERENCES trades(id)         ON DELETE CASCADE,
    rule_id  INTEGER NOT NULL REFERENCES playbook_rules(id) ON DELETE CASCADE,
    checked  INTEGER NOT NULL DEFAULT 0 CHECK (checked IN (0, 1)),
    PRIMARY KEY (trade_id, rule_id)
) STRICT, WITHOUT ROWID;
";

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

    #[test]
    fn all_planned_tables_exist() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        migrations().to_latest(&mut conn).unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for expected in [
            "custom_fields",
            "custom_values",
            "fills",
            "funding_events",
            "import_batches",
            "playbook_checks",
            "playbook_rules",
            "playbooks",
            "screenshots",
            "settings",
            "tag_groups",
            "tags",
            "trade_tags",
            "trades",
        ] {
            assert!(tables.iter().any(|t| t == expected), "missing {expected}");
        }
    }

    /// A v1 database (only `settings`) must upgrade to v2 and keep its data.
    #[test]
    fn upgrades_from_v1_keeping_settings() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        migrations().to_version(&mut conn, 1).unwrap();
        conn.execute("INSERT INTO settings VALUES ('theme', '\"dark\"')", [])
            .unwrap();

        migrations().to_latest(&mut conn).unwrap();

        let theme: String = conn
            .query_row("SELECT value FROM settings WHERE key = 'theme'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(theme, "\"dark\"");
    }
}
