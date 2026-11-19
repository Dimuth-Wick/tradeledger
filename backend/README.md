# backend/ — what works

The Rust core (a Tauri app). It does every calculation and owns the database.

- `src/connection/` — **the functions the frontend can call**, one file per area
- `src/features/` — the logic, one module per feature (tests next to each)
- `src/database/` — SQLite tables, saving and loading
- `src/importers/` — Binance CSV readers
- `src/models.rs` — shared data types
- `tauri.conf.json` — app name, version, window and installer settings

Run the checks from here: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
See [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) for the full map.
