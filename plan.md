# TradeLedger — Plan

A customizable, offline crypto trading journal for Windows (TradeZella-style), built with **Tauri 2 (Rust) + React + TypeScript + SQLite**.

---

## 1. Decisions (from the requirements grilling)

| # | Topic | Decision |
|---|-------|----------|
| 1 | Trade input | Manual entry + CSV import. Exchange API sync is **v2**. |
| 2 | Exchange | **Binance** first. Importer designed so other exchanges plug in later. |
| 3 | Markets | **Spot and USDT-M futures** equally (leverage, funding, realized PnL). |
| 4 | Stack | **Tauri 2**: Rust core + React/TypeScript UI. |
| 5 | v1 features | Trade log + dashboard, PnL calendar, tags/notes/screenshots, reports + playbooks, **risk plan per trade (entry / stop loss / take profit → planned R:R and R-multiple)**. |
| 6 | Customization | Custom fields, custom tags/tag groups, drag-and-drop dashboard layout, themes (dark/light/colors). |
| 7 | Fills → trades | Auto grouping (position-to-zero rule) + manual merge/split. Net PnL = gross − fees − funding. |
| 8 | Data | Local SQLite + screenshots in the OS app-data folder, **never in the repo**. One-click backup/restore zip. |
| 9 | Platforms | Windows first. GitHub Actions release builds (Win/macOS/Linux) near the end. |
| 10 | Name / license | **TradeLedger**, MIT, `github.com/Dimuth-Wick/tradeledger`. |
| 11 | Learning loop | Claude writes all the code. Every task: explain → quiz → code → diff walkthrough → tests → commit/push → update progress → ask. |
| 12 | Git identity | Author `Dimuth-Wick` with the GitHub **noreply** email. No co-author trailers. |

---

## 2. Architecture

```
            Binance CSV
                 │
┌────────────────▼────────────────────────────────┐
│  TradeLedger (Tauri desktop app)                │
│                                                 │
│  ┌───────────────────────────────────────────┐  │
│  │ React + TypeScript UI (webview)           │  │
│  │ Dashboard · Calendar · Trade log · Reports│  │
│  └───────────────▲───────────┬───────────────┘  │
│        results   │           │ invoke("cmd")    │
│  ┌───────────────┴───────────▼───────────────┐  │
│  │ Rust core                                 │  │
│  │ importers · grouper · stats · backup      │  │
│  └───────────────────────┬───────────────────┘  │
└──────────────────────────┼──────────────────────┘
                           ▼
        SQLite DB + screenshots  (%APPDATA%/TradeLedger)
```

**Rule:** all business logic (parsing, grouping, money math, stats, DB) lives in Rust and is unit-tested there. React only renders and calls commands.

### Key libraries (proposed — confirm in each task)
| Layer | Library | Why |
|---|---|---|
| Desktop shell | Tauri 2 | Small native app, Rust backend |
| DB | `rusqlite` (bundled) + `rusqlite_migration` | Simple, synchronous, embedded SQLite |
| Money math | `rust_decimal` | No floating-point rounding errors in PnL |
| CSV | `csv` + `serde` | Typed parsing |
| Time | `chrono` | Timestamps, time zones |
| UI build | Vite + React + TypeScript | Fast dev server |
| Tables | TanStack Table | Sorting/filtering trade log |
| Charts | Recharts (or ECharts) | Equity curve, bars, heatmap |
| Dashboard grid | `react-grid-layout` | Drag/resize widgets |
| Tests | `cargo test`, Vitest | Rust logic + UI logic |

---

## 3. Data model (draft — finalized in Phase 2)

```
fills ──many-to-one──► trades ◄── funding_events
                         │
     ┌──────────┬────────┼──────────┬──────────────┐
 trade_tags  screenshots  custom_values  playbook_checks
     │                       │              │
   tags ── tag_groups   custom_fields   playbooks
settings (theme, dashboard layout, preferences)
import_batches (which file, when, how many rows)
```

| Table | Main columns |
|---|---|
| `fills` | id, exchange, market (spot/usdm), symbol, side, qty, price, fee, fee_asset, realized_pnl, time, **dedupe_hash (unique)**, trade_id, import_batch_id |
| `trades` | id, market, symbol, direction (long/short), open_time, close_time, qty, avg_entry, avg_exit, leverage?, gross_pnl, fees, funding, net_pnl, **planned_entry, stop_loss, take_profit, planned_rr, r_multiple**, notes, playbook_id, status (open/closed), is_manual |
| `funding_events` | id, symbol, amount, time, trade_id |
| `tag_groups` / `tags` / `trade_tags` | e.g. groups *Setup*, *Mistake*, *Emotion* — user-defined |
| `screenshots` | id, trade_id, file_name, caption |
| `custom_fields` / `custom_values` | field type: text / number / select / checkbox |
| `playbooks` / `playbook_rules` / `playbook_checks` | setup rules checklist per trade |
| `settings` | key → JSON value |

---

## 4. Core formulas

| Metric | Formula |
|---|---|
| Net PnL | gross PnL − fees − funding |
| Risk per unit | long: entry − stop · short: stop − entry |
| Planned R:R | \|take_profit − entry\| ÷ risk per unit |
| R-multiple | long: (exit − entry) ÷ risk · short: (entry − exit) ÷ risk |
| Win rate | winning trades ÷ closed trades |
| Profit factor | Σ winning net PnL ÷ \|Σ losing net PnL\| |
| Expectancy | average R-multiple (also shown in $) |
| Avg win / avg loss | mean net PnL of winners / of losers |
| Max drawdown | largest peak-to-trough drop of the equity curve |

### Fill grouping algorithm (per exchange + market + symbol)
```
position = 0
for fill in fills sorted by time:
    if position == 0: open new trade
    attach fill to current trade
    position += (+qty if BUY else −qty)
    if position == 0: close trade (compute averages, PnL)
    if sign of position flipped: split fill → close trade, open new one with remainder
```
Manual **merge** (combine two trades) and **split** (break at a fill) let the user fix edge cases.

---

## 5. Roadmap (tasks)

Each task = one learning loop + one commit. Task IDs are used in `progress.md`.

### Phase 0 — Setup
- **T0.1** Install & verify toolchain: Rust (rustup), Node LTS, Tauri prerequisites (WebView2, MSVC build tools), VS Code extensions (rust-analyzer, Tauri).
- **T0.2** Configure git identity (Dimuth-Wick + noreply email), create GitHub repo `Dimuth-Wick/tradeledger`, push docs, `.gitignore`, LICENSE.

### Phase 1 — Skeleton
- **T1.1** Scaffold Tauri 2 + React + TS (Vite). App opens a window on Windows.
- **T1.2** First Rust command (`ping`) called from React via `invoke()`. Project folder structure + lint/format (rustfmt, clippy, eslint, prettier).

### Phase 2 — Database
- **T2.1** SQLite connection in app-data folder + migration system.
- **T2.2** Schema v1 (tables above) + Rust structs + repository functions + tests.

### Phase 3 — Manual trades & risk plan
- **T3.1** Rust: create/update/delete trade commands; R:R and R-multiple calculator with unit tests (long + short).
- **T3.2** UI: trade entry form (entry, stop, take profit, exit, qty, fees, direction, market) with validation and live R:R preview.

### Phase 4 — Binance import
- **T4.1** Study a real (anonymized) Binance spot + futures export; write the column mapping in `docs/binance-csv.md`; sample fixture CSVs (fake data) in `tests/fixtures/`.
- **T4.2** Importer trait (`ExchangeImporter`) + Binance spot parser + duplicate protection (dedupe_hash).
- **T4.3** Binance USDT-M futures parser + funding fee import.
- **T4.4** Fill grouping engine (position-to-zero + flip handling) with thorough tests.
- **T4.5** Import UI: pick file → preview → confirm → result summary.
- **T4.6** Manual merge/split trades.

### Phase 5 — Trade log & journaling
- **T5.1** Trade log table: sort, filter (date, symbol, market, tag, win/loss), pagination.
- **T5.2** Trade detail page: notes (markdown), planned SL/TP editing.
- **T5.3** Tags & tag groups manager (custom).
- **T5.4** Screenshots: paste/drag image, stored in app-data, viewer.
- **T5.5** Custom fields manager + display on trade detail.

### Phase 6 — Stats & dashboard
- **T6.1** Rust stats engine (all formulas in §4) with tests and date-range filter.
- **T6.2** Dashboard widgets: KPI tiles, equity curve, win/loss bars, R distribution.
- **T6.3** Drag-and-drop/resizable dashboard layout, saved in `settings`.

### Phase 7 — Calendar
- **T7.1** PnL calendar heatmap (month view, day click → that day's trades), time-zone setting.

### Phase 8 — Reports & playbooks
- **T8.1** Reports: stats grouped by tag, symbol, weekday, hour, market, direction.
- **T8.2** Playbooks: define setup rules; checklist on each trade; per-playbook stats.

### Phase 9 — Polish
- **T9.1** Themes (dark/light + accent color) via CSS variables.
- **T9.2** Backup/restore zip (DB + screenshots) and CSV export of trades.

### Phase 10 — Release
- **T10.1** GitHub Actions: build Windows/macOS/Linux installers on version tag; GitHub Releases.
- **T10.2** README screenshots (fake demo data), v1.0.0 tag.

### v2 ideas (not in v1)
Binance read-only API sync · more exchanges (Bybit, OKX) · cloud backup · daily journal · trade replay on chart · link to crypto-chart-skills analysis.

---

## 6. Definition of done (every task)
1. Concept explained + quiz answered.
2. Code builds with no warnings (`cargo clippy`, `npm run lint`).
3. Tests added and passing (`cargo test`, `npm test`).
4. Diff walked through with the user.
5. `progress.md` updated.
6. Committed and pushed as Dimuth-Wick.
7. User says "next" before the next task starts.
