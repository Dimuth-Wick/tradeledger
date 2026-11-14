# TradeLedger

> A free, open-source, offline **crypto trading journal** for your desktop. Import your Binance trades, plan your risk, tag your setups and mistakes, and see what actually makes you money.

![status](https://img.shields.io/badge/status-in%20development-orange) ![license](https://img.shields.io/badge/license-MIT-blue) ![stack](https://img.shields.io/badge/Tauri%202-Rust%20%2B%20React-informational)

## Features (v1 roadmap)

- 📥 **Import** Binance spot and USDT-M futures trade history (CSV), or add trades manually
- 🧮 **Automatic trade grouping**: partial fills become one trade; fees and funding are included in net PnL
- 🎯 **Risk plan per trade**: entry, stop loss, take profit → planned R:R and actual R-multiple
- 📊 **Dashboard**: win rate, profit factor, expectancy, avg win/loss, equity curve, drawdown, with drag-and-drop widgets
- 📅 **PnL calendar** heatmap
- 🏷️ **Tags, notes, screenshots, custom fields** on every trade
- 📈 **Reports** by tag, coin, weekday, hour, long/short
- 📘 **Playbooks**: setup checklists and per-setup stats
- 🎨 **Themes**: dark / light / accent colors
- 🔒 **Private by design**: all data stays in a local SQLite file on your computer, with one-click backup

## Tech stack

| Part | Tech |
|---|---|
| Desktop app | [Tauri 2](https://tauri.app) |
| Core logic | Rust (SQLite via rusqlite, `rust_decimal` for money math) |
| UI | React + TypeScript + Vite |

## Privacy

TradeLedger has no server and no account. Your trades are stored in your OS app-data folder (`%APPDATA%\TradeLedger` on Windows), never sent anywhere. API keys are not needed in v1.

## Getting started (development)

Prerequisites: Rust (rustup), Node.js LTS, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS (on Windows: WebView2 and the MSVC Build Tools).

```bash
npm install
npm run tauri dev   # first run compiles the Rust core and takes a few minutes
```

## Project docs

- [plan.md](plan.md): decisions, architecture, data model, roadmap
- [progress.md](progress.md): task-by-task progress log
- [CLAUDE.md](CLAUDE.md): how the AI coding assistant works on this repo

## Disclaimer

TradeLedger is a journaling and analysis tool, not financial advice.

## License

[MIT](LICENSE) © 2026 Dimuth-Wick
