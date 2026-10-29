//! Tauri commands. Each command only maps DTOs and delegates to `crm_core`.

mod attachments;
mod clients;
mod contacts;
mod events;
mod fields;
mod interactions;
mod invoices;
mod notes;
mod payments;
mod projects;
mod requisites;
mod secrets;
mod statuses;
mod tags;
mod tasks;
mod time;
mod vault;

pub(crate) use attachments::with_files;
pub use attachments::*;
pub use clients::*;
pub use contacts::*;
pub use events::*;
pub use fields::*;
pub use interactions::*;
pub use invoices::*;
pub use notes::*;
pub use payments::*;
pub use projects::*;
pub use requisites::*;
pub use secrets::*;
pub use statuses::*;
pub use tags::*;
pub use tasks::*;
pub use time::*;
pub use vault::*;

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
    let saved = crm_core::settings::save(&mut db, &settings)?;
    state
        .vault
        .set_autolock_minutes(saved.vault_autolock_minutes);
    Ok(saved)
}

/// User activity from the UI, postponing the vault auto-lock.
#[tauri::command]
#[specta::specta]
pub fn vault_touch(state: State<'_, AppState>) {
    state.vault.touch();
}
