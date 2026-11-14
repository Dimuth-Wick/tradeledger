mod commands;
// Public so the core can be used (and tested) as a library, not only through
// the Tauri commands; also keeps not-yet-used repository functions from being
// flagged as dead code before the commands that call them exist.
pub mod db;
pub mod importers;
pub mod models;
pub mod risk;
pub mod trade_entry;

use std::sync::Mutex;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // data_dir() = %APPDATA% on Windows. We add our own "TradeLedger"
            // folder instead of app_data_dir(), which would use the bundle id.
            let path = db::db_path(&app.path().data_dir()?);
            // Failing here stops startup with a clear error rather than
            // letting every later command fail on a missing DB.
            let conn = db::open(&path)?;
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
