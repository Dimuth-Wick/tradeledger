mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Every command the UI may call must be listed here; unlisted
        // functions are unreachable from the webview.
        .invoke_handler(tauri::generate_handler![commands::ping])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
