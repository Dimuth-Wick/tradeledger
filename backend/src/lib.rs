//! TradeLedger's Rust backend: the part of the app that does the work.
//!
//! ```text
//! frontend (React)  ──invoke("…")──►  connection/   functions the UI may call
//!                                         │
//!                                         ▼
//!                                     features/     the logic: trades, stats,
//!                                         │          calendar, reports, …
//!                                         ▼
//!                                     database/     SQLite tables and queries
//! ```
//!
//! `importers/` reads exchange CSV files and `models.rs` holds the shared
//! data types (Trade, Fill, …). See docs/ARCHITECTURE.md for the full map.

/// Functions the frontend can call (Tauri commands). Thin: they check
/// input, call a feature and turn errors into plain text for the UI.
mod connection;
/// SQLite: opening the file, schema migrations and one module per table.
pub mod database;
/// The app's logic, one module per feature.
pub mod features;
/// Readers for exchange CSV exports (Binance spot and USDT-M futures).
pub mod importers;
/// Shared data types: Trade, Fill, FundingEvent, …
pub mod models;

use features::{screenshots, tags};

use std::sync::Mutex;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // data_dir() = %APPDATA% on Windows. We add our own "TradeLedger"
            // folder instead of app_data_dir(), which would use the bundle id.
            // TRADELEDGER_DB_PATH lets developers run the app against a
            // throwaway database (e.g. to test imports with fake data)
            // without touching their real journal.
            let path = match std::env::var_os("TRADELEDGER_DB_PATH") {
                Some(custom) => std::path::PathBuf::from(custom),
                None => database::db_path(&app.path().data_dir()?),
            };
            // Failing here stops startup with a clear error rather than
            // letting every later command fail on a missing DB.
            let conn = database::open(&path)?;
            tags::seed_defaults(&conn)?;
            // Deleted trades leave their image files behind; tidy up. Never
            // block startup over it.
            if let Err(e) = screenshots::prune_orphans(&conn, &screenshots::dir_for(&path)) {
                eprintln!("screenshot cleanup skipped: {e}");
            }
            app.manage(database::Db {
                conn: Mutex::new(conn),
                path,
            });
            Ok(())
        })
        // Every command the UI may call must be listed here; unlisted
        // functions are unreachable from the webview.
        .invoke_handler(tauri::generate_handler![
            connection::ping,
            connection::db_info,
            connection::trades::preview_trade,
            connection::trades::preview_plan,
            connection::trades::symbol_choices,
            connection::trades::create_trade,
            connection::trades::update_trade,
            connection::trades::delete_trade,
            connection::trades::get_trade,
            connection::trades::list_trades,
            connection::trades::query_trades,
            connection::trades::get_stats,
            connection::trades::preview_journal,
            connection::trades::update_journal,
            connection::trades::list_symbols,
            connection::import::preview_import,
            connection::import::confirm_import,
            connection::grouping::list_trade_fills,
            connection::grouping::merge_trade_with_previous,
            connection::grouping::split_trade_at_fill,
            connection::grouping::reset_grouping,
            connection::tags::list_tag_groups,
            connection::tags::create_tag_group,
            connection::tags::update_tag_group,
            connection::tags::delete_tag_group,
            connection::tags::create_tag,
            connection::tags::update_tag,
            connection::tags::delete_tag,
            connection::tags::get_trade_tags,
            connection::tags::set_trade_tags,
            connection::screenshots::add_screenshot,
            connection::screenshots::list_screenshots,
            connection::screenshots::screenshot_image,
            connection::screenshots::set_screenshot_caption,
            connection::screenshots::delete_screenshot,
            connection::custom_fields::list_custom_fields,
            connection::custom_fields::create_custom_field,
            connection::custom_fields::update_custom_field,
            connection::custom_fields::delete_custom_field,
            connection::custom_fields::get_custom_values,
            connection::custom_fields::set_custom_values,
            connection::dashboard::get_dashboard_layout,
            connection::dashboard::save_dashboard_layout,
            connection::dashboard::reset_dashboard_layout,
            connection::calendar::get_calendar_month,
            connection::calendar::get_day_trades,
            connection::calendar::get_time_zone,
            connection::calendar::set_time_zone,
            connection::calendar::list_time_zones,
            connection::reports::get_report,
            connection::playbooks::list_playbooks,
            connection::playbooks::create_playbook,
            connection::playbooks::update_playbook,
            connection::playbooks::delete_playbook,
            connection::playbooks::get_trade_playbook,
            connection::playbooks::set_trade_playbook,
            connection::playbooks::playbook_stats,
            connection::theme::get_theme,
            connection::theme::set_theme,
            connection::theme::list_accent_presets,
            connection::data::backup_now,
            connection::data::inspect_backup,
            connection::data::restore_backup,
            connection::data::export_trades_csv,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
