//! Features: everything the app knows how to do, one module per feature.
//!
//! These modules hold the real logic and calculations (PnL, stats, the
//! calendar, reports…). They know nothing about the window or the UI: the
//! UI reaches them only through `crate::connection`, and they store data
//! through `crate::database`.

/// Back up everything to one .zip and restore it.
pub mod backup;
/// PnL calendar (days in the chosen time zone) and the time-zone setting.
pub mod calendar;
/// User-defined fields on trades (text, number, choice, checkbox).
pub mod custom_fields;
/// Where each dashboard widget sits on the grid.
pub mod dashboard_layout;
/// Trades as a CSV file.
pub mod export;
/// Fills → trades (position-to-zero), plus manual merge/split.
pub mod grouping;
/// Exchange CSV import: preview, then save and regroup.
pub mod import_flow;
/// Plan, leverage and notes on a trade.
pub mod journal;
/// Setups with a rule checklist, and their stats.
pub mod playbooks;
/// Stats split by tag, coin, weekday, hour, market, side or playbook.
pub mod reports;
/// Planned R:R and R-multiple math.
pub mod risk;
/// Trade screenshots on disk.
pub mod screenshots;
/// Win rate, profit factor, drawdown, equity curve and the rest.
pub mod stats;
/// The symbol menu: popular Binance pairs per market.
pub mod symbols;
/// Tag groups and tags.
pub mod tags;
/// Light / dark / accent color.
pub mod theme;
/// Manual trades: create, edit, delete (PnL computed here).
pub mod trade_entry;
/// The trade log's filters, sorting and pages.
pub mod trade_query;
/// The safety backup made before an app update installs.
pub mod updates;
