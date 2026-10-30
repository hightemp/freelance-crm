use crm_core::{CoreError, CoreResult};
use tauri::State;

use crate::AppState;

/// Whether the app starts with the OS (FR-ST-6).
#[tauri::command]
#[specta::specta]
pub fn autostart_enabled(state: State<'_, AppState>) -> CoreResult<bool> {
    state
        .autostart
        .is_enabled()
        .map_err(|_| CoreError::new("autostart.unavailable"))
}

#[tauri::command]
#[specta::specta]
pub fn set_autostart(state: State<'_, AppState>, enabled: bool) -> CoreResult<()> {
    state
        .autostart
        .set(enabled)
        .map_err(|_| CoreError::new("autostart.unavailable"))
}
