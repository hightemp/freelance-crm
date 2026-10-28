mod bindings;
mod commands;
#[cfg(any(debug_assertions, test))]
mod dev;
mod paths;

use std::sync::Mutex;

use crm_core::db::Db;
use tauri::Manager;

/// Shared application state. Commands lock the database for the duration
/// of a single use case.
pub struct AppState {
    pub db: Mutex<Db>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    dev::bypass_proxy_for_dev_server();

    let specta = bindings::builder();
    #[cfg(debug_assertions)]
    specta
        .export(bindings::exporter(), bindings::BINDINGS_PATH)
        .expect("failed to export typescript bindings");

    let builder = tauri::Builder::default();
    // Embedded WebDriver for e2e tests only; see the `e2e` feature.
    #[cfg(feature = "e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio_webdriver::init())
        .plugin(tauri_plugin_wdio::init());

    builder
        // Must be registered first: a second launch focuses the running window.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(specta.invoke_handler())
        .setup(|app| {
            let dir = paths::data_dir(
                std::env::var_os(paths::DATA_DIR_ENV),
                app.path().app_data_dir()?,
            );
            let db_path = dir.join("crm.db");
            let db = Db::open(&db_path)?;
            app.manage(AppState { db: Mutex::new(db) });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
