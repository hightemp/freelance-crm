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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let db_path = app.path().app_data_dir()?.join("crm.db");
            let db = Db::open(&db_path)?;
            app.manage(AppState { db: Mutex::new(db) });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
