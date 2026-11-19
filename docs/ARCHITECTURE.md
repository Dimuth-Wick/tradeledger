# How TradeLedger is organised

TradeLedger is a desktop app built with [Tauri](https://tauri.app). It has two halves that talk to each other:

| Half         | Folder      | Language           | Job                                                          |
| ------------ | ----------- | ------------------ | ------------------------------------------------------------ |
| **Frontend** | `frontend/` | TypeScript + React | Everything you **see**: pages, buttons, charts               |
| **Backend**  | `backend/`  | Rust               | Everything that **works**: calculations, files, the database |

The frontend never does money math or touches files. It asks the backend, and the backend answers. The two meet in a folder called **`connection/`** on each side.

```text
┌──────────────────────── frontend/ (React, what you see) ───────────────────────┐
│  pages/          one file per tab (Dashboard, Calendar, Trades…)                │
│  components/     the pieces pages are built from (charts, forms, lists)         │
│  logic/          small helpers: formatting, form checks (no money math)         │
│  connection/backend.ts   ── invoke("create_trade", …) ──┐                       │
└─────────────────────────────────────────────────────────┼──────────────────────┘
                                                          │  Tauri carries the call
┌──────────────────────── backend/ (Rust, what works) ────┼──────────────────────┐
│  src/connection/   the functions the frontend may call ◄┘                       │
│        │            (check input, call a feature, return the answer)            │
│        ▼                                                                         │
│  src/features/     the real logic: trades, stats, calendar, reports, backup…    │
│        │                                                                         │
│        ▼                                                                         │
│  src/database/     SQLite: tables, saving and loading                           │
│  src/importers/    reading Binance CSV files                                    │
│  src/models.rs     shared types (Trade, Fill, …)                                │
└────────────────────────────────────────────────────────────────────────────────┘
                     your data: %APPDATA%\TradeLedger\tradeledger.db (never in the repo)
```

## Follow one click: "Save trade"

| Step | Where                                               | What happens                                                                  |
| ---- | --------------------------------------------------- | ----------------------------------------------------------------------------- |
| 1    | `frontend/components/trades/TradeForm.tsx`          | You fill the form and press **Save trade**                                    |
| 2    | `frontend/logic/tradeForm.ts`                       | Checks the boxes are numbers and turns them into a `TradeInput`               |
| 3    | `frontend/connection/backend.ts` → `createTrade()`  | Sends `invoke("create_trade", { input })` to the backend                      |
| 4    | `backend/src/connection/trades.rs` → `create_trade` | Receives the call, opens the database                                         |
| 5    | `backend/src/features/trade_entry.rs`               | Validates, works out quantity, fees, PnL, R:R and R-multiple (exact decimals) |
| 6    | `backend/src/database/trades.rs`                    | Saves the row in SQLite                                                       |
| 7    | back up the same path                               | The saved trade returns to the form, which shows "Saved trade #…"             |

Every feature follows this same path, so once you know one, you know them all.

## Folder by folder

### `frontend/` — what you see

| Folder / file           | What's inside                                                                |
| ----------------------- | ---------------------------------------------------------------------------- |
| `main.tsx`, `App.tsx`   | Start-up, the top bar and the tabs                                           |
| `pages/`                | One file per tab: `DashboardPage`, `CalendarPage`, `HomePage` (Trades), …    |
| `components/dashboard/` | Dashboard widgets: key numbers, equity and win-rate charts, R distribution   |
| `components/trades/`    | New-trade form, trade log and list, notes, tags, screenshots                 |
| `components/settings/`  | The Settings sections: appearance, tags, custom fields, backup and export    |
| `components/shared/`    | Pieces used everywhere: theme toggle, option chips, filters, tag chips       |
| `logic/`                | Helpers with tests (`*.test.ts`): number formatting, form checks, chart rows |
| `connection/backend.ts` | **The only file that talks to Rust.** One typed function per backend command |
| `styles/app.css`        | All styles; colors are CSS variables so the light/dark themes swap them      |

### `backend/` — what works

| Folder / file               | What's inside                                                                         |
| --------------------------- | ------------------------------------------------------------------------------------- |
| `src/main.rs`, `src/lib.rs` | Start-up: opens the database and registers every command                              |
| `src/connection/`           | **The functions the frontend calls**, one file per area (trades, calendar, data, …)   |
| `src/features/`             | **The logic**, one module per feature; tests sit next to each one (`…/tests.rs`)      |
| `src/database/`             | SQLite: `migrations.rs` (table layout over versions) and one file per table           |
| `src/importers/`            | Binance spot and USDT-M futures CSV readers                                           |
| `src/models.rs`             | The shared data types                                                                 |
| `tauri.conf.json`           | App name, window size, version, installer settings                                    |
| `capabilities/`             | What the window is allowed to do (e.g. open file dialogs)                             |
| `icons/`                    | App icons; `app-icon.svg` is the source (`npx tauri icon backend/icons/app-icon.svg`) |
| `examples/seed_demo.rs`     | Makes a demo database full of fake trades                                             |
| `tests/`                    | Checks that the fake CSV files are well-formed                                        |

### Everything else

| Folder               | What's inside                                                      |
| -------------------- | ------------------------------------------------------------------ |
| `test-data/`         | Fake Binance CSV files used by the tests (never real data)         |
| `docs/`              | This guide, the Binance CSV notes, README screenshots              |
| `public/`            | Files served as-is (the browser-tab icon)                          |
| `.github/workflows/` | CI (tests on every push) and Release (installers on a version tag) |

## Rules that keep it simple

- **Money is exact.** All amounts use Rust's `rust_decimal`, never floating point. The frontend receives amounts as text and only formats them.
- **Logic lives in the backend.** If something is calculated, it's in `backend/src/features/` and has a test.
- **One way in.** The frontend reaches the backend only through `frontend/connection/backend.ts` → `backend/src/connection/`.
- **Your data stays on your computer**, in the app-data folder, never in this repository.
