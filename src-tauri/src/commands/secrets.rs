use crm_core::domain::Id;
use crm_core::vault::secrets::{
    FieldRef, Secret, SecretFilter, SecretInput, SecretPayload, SecretRepo,
};
use crm_core::vault::templates::{self, SecretTemplate};
use crm_core::{CoreError, CoreResult};
use tauri::State;

use crate::AppState;

/// Runs a use case with the secret repository (locked database, clock, vault).
fn with_secrets<T>(
    state: &AppState,
    f: impl FnOnce(&SecretRepo<'_>) -> CoreResult<T>,
) -> CoreResult<T> {
    let db = state.db.lock().expect("db mutex poisoned");
    f(&SecretRepo::new(
        db.conn(),
        state.clock.as_ref(),
        &state.vault,
    ))
}

#[tauri::command]
#[specta::specta]
pub fn list_secret_templates() -> Vec<SecretTemplate> {
    templates::templates()
}

#[tauri::command]
#[specta::specta]
pub fn list_secrets(state: State<'_, AppState>, filter: SecretFilter) -> CoreResult<Vec<Secret>> {
    with_secrets(&state, |repo| repo.list(&filter))
}

#[tauri::command]
#[specta::specta]
pub fn get_secret(state: State<'_, AppState>, id: Id) -> CoreResult<Secret> {
    with_secrets(&state, |repo| repo.get(id))
}

#[tauri::command]
#[specta::specta]
pub fn create_secret(state: State<'_, AppState>, input: SecretInput) -> CoreResult<Secret> {
    with_secrets(&state, |repo| repo.create(input))
}

#[tauri::command]
#[specta::specta]
pub fn update_secret(state: State<'_, AppState>, id: Id, input: SecretInput) -> CoreResult<Secret> {
    with_secrets(&state, |repo| repo.update(id, input))
}

#[tauri::command]
#[specta::specta]
pub fn delete_secret(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_secrets(&state, |repo| repo.delete(id))
}

/// All values, for the edit form of an unlocked vault.
#[tauri::command]
#[specta::specta]
pub fn secret_payload(state: State<'_, AppState>, id: Id) -> CoreResult<SecretPayload> {
    with_secrets(&state, |repo| repo.payload(id))
}

#[tauri::command]
#[specta::specta]
pub fn reveal_secret_field(
    state: State<'_, AppState>,
    id: Id,
    field: FieldRef,
) -> CoreResult<String> {
    with_secrets(&state, |repo| {
        repo.reveal(id, &field).map(|v| v.to_string())
    })
}

/// Copies a value on the backend; it never passes through the UI.
#[tauri::command]
#[specta::specta]
pub fn copy_secret_field(state: State<'_, AppState>, id: Id, field: FieldRef) -> CoreResult<()> {
    let value = with_secrets(&state, |repo| repo.reveal(id, &field))?;
    state
        .clipboard
        .write_text(&value)
        .map_err(|_| CoreError::new("clipboard.failed"))
}
