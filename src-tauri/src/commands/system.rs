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

/// A newer version, if there is one.
#[tauri::command]
#[specta::specta]
pub async fn check_for_update(
    state: State<'_, AppState>,
) -> CoreResult<Option<crate::updates::AvailableUpdate>> {
    state
        .updates
        .check()
        .await
        .map_err(|_| CoreError::new("update.check_failed"))
}

/// Installs the update found by the last check and restarts the app.
#[tauri::command]
#[specta::specta]
pub async fn install_update(state: State<'_, AppState>) -> CoreResult<()> {
    state
        .updates
        .install()
        .await
        .map_err(|_| CoreError::new("update.install_failed"))
}
