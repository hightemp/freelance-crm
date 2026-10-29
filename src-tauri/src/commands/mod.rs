//! Tauri commands. Each command only maps DTOs and delegates to `crm_core`.

mod clients;
mod contacts;
mod fields;
mod notes;
mod requisites;
mod tags;

pub use clients::*;
pub use contacts::*;
pub use fields::*;
pub use notes::*;
pub use requisites::*;
pub use tags::*;

use crm_core::app::AppInfo;
use crm_core::db::Connection;
use crm_core::domain::Clock;
use crm_core::settings::Settings;
use crm_core::{CoreError, CoreResult};
use tauri::State;

use crate::AppState;

/// Runs a use case with the locked connection and the app clock.
fn with_conn<T>(
    state: &AppState,
    f: impl FnOnce(&Connection, &dyn Clock) -> CoreResult<T>,
) -> CoreResult<T> {
    let db = state.db.lock().expect("db mutex poisoned");
    f(db.conn(), state.clock.as_ref())
}

#[cfg(test)]
mod tests;

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
