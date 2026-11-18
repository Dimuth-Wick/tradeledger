pub mod calendar;
mod commands;
pub mod custom_fields;
pub mod dashboard_layout;
// Public so the core can be used (and tested) as a library, not only through
// the Tauri commands; also keeps not-yet-used repository functions from being
// flagged as dead code before the commands that call them exist.
pub mod db;
pub mod grouping;
pub mod import_flow;
pub mod importers;
pub mod journal;
pub mod models;
pub mod playbooks;
pub mod reports;
pub mod risk;
pub mod screenshots;
pub mod stats;
pub mod tags;
pub mod theme;
pub mod trade_entry;
pub mod trade_query;

use std::sync::Mutex;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // data_dir() = %APPDATA% on Windows. We add our own "TradeLedger"
            // folder instead of app_data_dir(), which would use the bundle id.
            // TRADELEDGER_DB_PATH lets developers run the app against a
            // throwaway database (e.g. to test imports with fake data)
            // without touching their real journal.
            let path = match std::env::var_os("TRADELEDGER_DB_PATH") {
                Some(custom) => std::path::PathBuf::from(custom),
                None => db::db_path(&app.path().data_dir()?),
            };
            // Failing here stops startup with a clear error rather than
            // letting every later command fail on a missing DB.
            let conn = db::open(&path)?;
            tags::seed_defaults(&conn)?;
            // Deleted trades leave their image files behind; tidy up. Never
            // block startup over it.
            if let Err(e) = screenshots::prune_orphans(&conn, &screenshots::dir_for(&path)) {
                eprintln!("screenshot cleanup skipped: {e}");
            }
            app.manage(db::Db {
                conn: Mutex::new(conn),
                path,
            });
            Ok(())
        })
        // Every command the UI may call must be listed here; unlisted
        // functions are unreachable from the webview.
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::db_info,
            commands::trades::preview_trade,
            commands::trades::create_trade,
            commands::trades::update_trade,
            commands::trades::delete_trade,
            commands::trades::get_trade,
            commands::trades::list_trades,
            commands::trades::query_trades,
            commands::trades::get_stats,
            commands::trades::preview_journal,
            commands::trades::update_journal,
            commands::trades::list_symbols,
            commands::import::preview_import,
            commands::import::confirm_import,
            commands::grouping::list_trade_fills,
            commands::grouping::merge_trade_with_previous,
            commands::grouping::split_trade_at_fill,
            commands::grouping::reset_grouping,
            commands::tags::list_tag_groups,
            commands::tags::create_tag_group,
            commands::tags::update_tag_group,
            commands::tags::delete_tag_group,
            commands::tags::create_tag,
            commands::tags::update_tag,
            commands::tags::delete_tag,
            commands::tags::get_trade_tags,
            commands::tags::set_trade_tags,
            commands::screenshots::add_screenshot,
            commands::screenshots::list_screenshots,
            commands::screenshots::screenshot_image,
            commands::screenshots::set_screenshot_caption,
            commands::screenshots::delete_screenshot,
            commands::custom_fields::list_custom_fields,
            commands::custom_fields::create_custom_field,
            commands::custom_fields::update_custom_field,
            commands::custom_fields::delete_custom_field,
            commands::custom_fields::get_custom_values,
            commands::custom_fields::set_custom_values,
            commands::dashboard::get_dashboard_layout,
            commands::dashboard::save_dashboard_layout,
            commands::dashboard::reset_dashboard_layout,
            commands::calendar::get_calendar_month,
            commands::calendar::get_day_trades,
            commands::calendar::get_time_zone,
            commands::calendar::set_time_zone,
            commands::calendar::list_time_zones,
            commands::reports::get_report,
            commands::playbooks::list_playbooks,
            commands::playbooks::create_playbook,
            commands::playbooks::update_playbook,
            commands::playbooks::delete_playbook,
            commands::playbooks::get_trade_playbook,
            commands::playbooks::set_trade_playbook,
            commands::playbooks::playbook_stats,
            commands::theme::get_theme,
            commands::theme::set_theme,
            commands::theme::list_accent_presets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
