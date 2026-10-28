//! Tauri commands. Each command only maps DTOs and delegates to `crm_core`.

use crm_core::CoreError;
use crm_core::app::AppInfo;
use tauri::State;

use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn app_info(state: State<'_, AppState>) -> Result<AppInfo, CoreError> {
    let db = state.db.lock().expect("db mutex poisoned");
    crm_core::app::app_info(&db)
}
