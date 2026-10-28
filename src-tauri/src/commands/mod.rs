//! Tauri commands. Each command only maps DTOs and delegates to `crm_core`.

use crm_core::CoreError;
use crm_core::app::AppInfo;
use crm_core::settings::Settings;
use tauri::State;

use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn app_info(state: State<'_, AppState>) -> Result<AppInfo, CoreError> {
    let db = state.db.lock().expect("db mutex poisoned");
    crm_core::app::app_info(&db)
}

#[tauri::command]
#[specta::specta]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, CoreError> {
    let db = state.db.lock().expect("db mutex poisoned");
    crm_core::settings::load(&db)
}

#[tauri::command]
#[specta::specta]
pub fn save_settings(
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Settings, CoreError> {
    let mut db = state.db.lock().expect("db mutex poisoned");
    crm_core::settings::save(&mut db, &settings)
}
