use crm_core::vault::{VaultStatus, recovery};
use crm_core::{CoreError, CoreResult};
use tauri::State;

use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn vault_status(state: State<'_, AppState>) -> CoreResult<VaultStatus> {
    let db = state.db.lock().expect("db mutex poisoned");
    state.vault.status(db.conn())
}

/// Creates the vault and returns the recovery key, shown to the user once.
#[tauri::command]
#[specta::specta]
pub fn vault_init(state: State<'_, AppState>, password: String) -> CoreResult<String> {
    let db = state.db.lock().expect("db mutex poisoned");
    state
        .vault
        .init(db.conn(), &password)
        .map(|key| key.to_string())
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

/// Unlocks with the recovery key and sets a new master password.
#[tauri::command]
#[specta::specta]
pub fn vault_recover(
    state: State<'_, AppState>,
    recovery_key: String,
    new_password: String,
) -> CoreResult<()> {
    let db = state.db.lock().expect("db mutex poisoned");
    state.vault.recover(db.conn(), &recovery_key, &new_password)
}

/// Issues a new recovery key; the previous one stops working.
#[tauri::command]
#[specta::specta]
pub fn vault_rotate_recovery_key(state: State<'_, AppState>) -> CoreResult<String> {
    let db = state.db.lock().expect("db mutex poisoned");
    state
        .vault
        .rotate_recovery_key(db.conn())
        .map(|key| key.to_string())
}

/// Saves a recovery key to a file the user chose. Only valid recovery keys
/// are written, so this cannot be used to write arbitrary files.
#[tauri::command]
#[specta::specta]
pub fn save_recovery_key_file(path: String, recovery_key: String) -> CoreResult<()> {
    recovery::decode(&recovery_key)?;
    let text = format!("{}\n", recovery_key.trim());
    std::fs::write(&path, text).map_err(|_| CoreError::new("recovery.save_failed"))
}
