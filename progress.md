# TradeLedger — Progress

Status: ⬜ todo · 🟨 in progress · ✅ done

**Current task:** T3.1 — Trade CRUD + R:R / R-multiple calculator

| Task  | Description                                               | Status | Commit  | Date       |
| ----- | --------------------------------------------------------- | ------ | ------- | ---------- |
| —     | Requirements grilling + plan, README, progress, CLAUDE.md | ✅     | —       | 2026-11-14 |
| T0.1  | Install & verify toolchain                                | ✅     | b236bc7 | 2026-11-14 |
| T0.2  | Git identity + GitHub repo + first push                   | ✅     | b236bc7 | 2026-11-14 |
| T1.1  | Scaffold Tauri 2 + React + TS                             | ✅     | 383b82f | 2026-11-14 |
| T1.2  | First `invoke()` command + lint/format                    | ✅     | 1e70ec4 | 2026-11-14 |
| T2.1  | SQLite in app-data + migrations                           | ✅     | 2a706ba | 2026-11-14 |
| T2.2  | Schema v1 + structs + repository + tests                  | ✅     | 8fac8a3 | 2026-11-14 |
| T3.1  | Trade CRUD + R:R / R-multiple calculator                  | ⬜     |         |            |
| T3.2  | Trade entry form + live R:R preview                       | ⬜     |         |            |
| T4.1  | Binance CSV study + fixtures                              | ⬜     |         |            |
| T4.2  | Importer trait + Binance spot parser + dedupe             | ⬜     |         |            |
| T4.3  | Binance futures parser + funding                          | ⬜     |         |            |
| T4.4  | Fill grouping engine                                      | ⬜     |         |            |
| T4.5  | Import UI                                                 | ⬜     |         |            |
| T4.6  | Merge/split trades                                        | ⬜     |         |            |
| T5.1  | Trade log table                                           | ⬜     |         |            |
| T5.2  | Trade detail + notes                                      | ⬜     |         |            |
| T5.3  | Tags & tag groups                                         | ⬜     |         |            |
| T5.4  | Screenshots                                               | ⬜     |         |            |
| T5.5  | Custom fields                                             | ⬜     |         |            |
| T6.1  | Stats engine                                              | ⬜     |         |            |
| T6.2  | Dashboard widgets                                         | ⬜     |         |            |
| T6.3  | Drag-and-drop dashboard layout                            | ⬜     |         |            |
| T7.1  | PnL calendar heatmap                                      | ⬜     |         |            |
| T8.1  | Reports                                                   | ⬜     |         |            |
| T8.2  | Playbooks                                                 | ⬜     |         |            |
| T9.1  | Themes                                                    | ⬜     |         |            |
| T9.2  | Backup/restore + CSV export                               | ⬜     |         |            |
| T10.1 | GitHub Actions release builds                             | ⬜     |         |            |
| T10.2 | README screenshots + v1.0.0                               | ⬜     |         |            |

---

## Log

<!-- Newest first. One entry per finished task: what was built, what was learned, quiz result, any follow-ups. -->

### 2026-11-14 — T2.2 Schema + structs + repositories

- New deps: `rust_decimal` (serde → decimals sent to UI as strings), `rust_decimal_macros` (tests only).
- Migration v2: all 13 trading tables from plan §3 (STRICT, CHECK-ed enums, decimals as TEXT, times as ms INTEGER, FK rules: deleting a trade unlinks fills/funding and cascades tags/screenshots/custom values/checks).
- Schema additions vs. draft: `exchange` on trades; `exchange`, `dedupe_hash`, `import_batch_id` on funding_events. plan.md §3 updated.
- `models.rs`: Fill, Trade, FundingEvent, ImportBatch, `WithId<T>`, enums Market/Side/Direction/TradeStatus.
- Repositories: settings, import_batches, fills (dedupe insert, list by instrument/trade, link), funding (dedupe insert, list by trade), trades (insert/get/update/delete/list). Tags, screenshots, custom fields, playbooks repos come with their tasks.
- Tests: Rust 31 (incl. v1→v2 upgrade, exact decimal round-trip, FK + CHECK enforcement, cascade/unlink on delete), TS 3.
- Verified in the real app: existing v1 DB upgraded to "schema v2".
- Next: T3.1.

### 2026-11-14 — T2.1 SQLite in app-data + migrations

- New deps: `rusqlite` 0.41 (`bundled`), `rusqlite_migration` 2.7, `thiserror` 2.
- `db` module: DB at `%APPDATA%\TradeLedger\tradeledger.db` (OS data dir + fixed folder name, not the bundle id); folder auto-created; `foreign_keys=ON`, WAL journal; migrations run on every startup via `PRAGMA user_version`.
- Migration v1 = `settings` table (key → JSON). Trading tables come as v2 in T2.2.
- Connection held in Tauri state behind a `Mutex`; new `db_info` command (path + schema version, camelCase JSON), shown on the home page.
- Tests: Rust 9 (path, migrations valid, version, foreign keys, WAL, reopen keeps data), TS 3.
- Verified in the real app: file created in `%APPDATA%\TradeLedger`, UI shows "schema v1"; no DB files in the repo.
- Next: T2.2.

### 2026-11-14 — T1.2 First `invoke()` command + lint/format

- Rust: `commands::ping` (returns `Result<String, String>`, the shape all commands use) + unit test; template `greet` removed.
- UI: `src/lib/api.ts` typed wrapper around `invoke()`; `src/pages/HomePage.tsx` with a "Ping Rust core" button; template logos removed.
- Tooling: ESLint (typescript-eslint, react-hooks, react-refresh), Prettier, Vitest. Scripts: `lint`, `format`, `format:check`, `test`.
- Verified: cargo fmt / clippy `-D warnings` / test (1), npm lint / format:check / test (2) / build all pass; clicking the button in the real window showed "pong from TradeLedger core v0.1.0".
- Note: `npm audit` reports 7 moderate issues in `minimatch` under typescript-eslint (dev-only lint tool, not shipped in the app).
- Next: T2.1.

### 2026-11-14 — T1.1 Scaffold Tauri 2 + React + TS

- Generated with `create-tauri-app` 4.7.5 (react-ts template, npm, identifier `com.dimuthwick.tradeledger`), copied into the repo; existing README and `.gitignore` kept.
- Branded: product name / window title "TradeLedger", 1200×800 window, Cargo metadata (author, MIT).
- Verified: `npm run build`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` (0 tests yet) all pass; `npm run tauri dev` opened the "TradeLedger" window.
- Template `greet` command left in place; T1.2 replaces it with `ping`.
- Note: build stopped once because C: was full; ~23 GB freed.
- Next: T1.2.

### 2026-11-14 — T0.2 Git identity + GitHub repo + first push

- Repo-local git identity set: Dimuth-Wick + GitHub noreply email.
- Repo, remote, `.gitignore` and LICENSE were already in place from the planning commit.
- CLAUDE.md loop replaced with the lean version (explanations and quizzes happen in a separate chat).
- Known: the planning commit `39417d5` carries the gmail address; left as is (no force-push).
- Next: T1.1.

### 2026-11-14 — T0.1 Install & verify toolchain

- Already present: Node 24.11.0 (LTS), npm 11.6.1, git 2.55.0, WebView2 157.0.4341.57, VS Code 1.145.1.
- Installed: rustup 1.29.1 with rustc/cargo 1.100.0 (`stable-x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 (C++ workload) + Windows SDK 10.0.26100.0, VS Code extensions rust-analyzer and Tauri.
- Verified: a test Rust program compiled, linked and ran.
- No repo code changed; the progress entry went in with the T0.2 commit.
- Next: T0.2.

### 2026-11-14 — Planning

- Requirements settled through a grilling session (see plan.md §1).
- Stack: Tauri 2 (Rust) + React + TypeScript + SQLite.
- Understanding check: architecture, fill grouping, R-multiple, profit factor, data safety.
- Next: T0.1.
