mod bindings;
mod commands;

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
    let specta = bindings::builder();
    #[cfg(debug_assertions)]
    specta
        .export(bindings::exporter(), bindings::BINDINGS_PATH)
        .expect("failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(specta.invoke_handler())
        .setup(|app| {
            let db_path = app.path().app_data_dir()?.join("crm.db");
            let db = Db::open(&db_path)?;
            app.manage(AppState { db: Mutex::new(db) });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
