use crm_core::CoreResult;
use crm_core::vault::VaultStatus;
use tauri::State;

use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn vault_status(state: State<'_, AppState>) -> CoreResult<VaultStatus> {
    let db = state.db.lock().expect("db mutex poisoned");
    state.vault.status(db.conn())
}

#[tauri::command]
#[specta::specta]
pub fn vault_init(state: State<'_, AppState>, password: String) -> CoreResult<()> {
    let db = state.db.lock().expect("db mutex poisoned");
    state.vault.init(db.conn(), &password)
}

#[tauri::command]
#[specta::specta]
pub fn vault_unlock(state: State<'_, AppState>, password: String) -> CoreResult<()> {
    let db = state.db.lock().expect("db mutex poisoned");
    state.vault.unlock(db.conn(), &password)
}

#[tauri::command]
#[specta::specta]
pub fn vault_lock(state: State<'_, AppState>) {
    state.vault.lock();
}

#[tauri::command]
#[specta::specta]
pub fn vault_change_password(
    state: State<'_, AppState>,
    old_password: String,
    new_password: String,
) -> CoreResult<()> {
    let db = state.db.lock().expect("db mutex poisoned");
    state
        .vault
        .change_password(db.conn(), &old_password, &new_password)
}
