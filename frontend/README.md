# frontend/ — what you see

The React + TypeScript user interface. It shows data and sends requests; it never calculates money or touches files.

- `pages/` — one file per tab
- `components/` — the building blocks, grouped by area (`dashboard/`, `trades/`, `settings/`, `shared/`)
- `logic/` — small tested helpers (formatting, form checks)
- `connection/backend.ts` — **the only file that talks to the Rust backend**
- `styles/app.css` — all styles and the light/dark color themes

See [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) for how a click travels through the app.
