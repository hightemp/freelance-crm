use crm_core::domain::Id;
use crm_core::vault::generator::{self, Generated, PassphraseOptions, PasswordOptions};
use crm_core::vault::secrets::{
    FieldRef, Secret, SecretFilter, SecretInput, SecretOutline, SecretPayload, SecretRepo,
};
use crm_core::vault::ssh;
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

/// Which values a secret has, without the values.
#[tauri::command]
#[specta::specta]
pub fn secret_outline(state: State<'_, AppState>, id: Id) -> CoreResult<SecretOutline> {
    with_secrets(&state, |repo| repo.outline(id))
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

#[tauri::command]
#[specta::specta]
pub fn generate_password(options: PasswordOptions) -> CoreResult<Generated> {
    generator::generate_password(&options)
}

#[tauri::command]
#[specta::specta]
pub fn generate_passphrase(options: PassphraseOptions) -> CoreResult<Generated> {
    generator::generate_passphrase(&options)
}

/// A key pair to fill the fields of an `ssh_key` secret.
#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SshKeyDto {
    pub private_key: String,
    pub public_key: String,
    pub algorithm: String,
    pub comment: String,
    pub fingerprint: String,
    pub encrypted: bool,
}

impl From<ssh::SshKeyPair> for SshKeyDto {
    fn from(pair: ssh::SshKeyPair) -> Self {
        Self {
            private_key: pair.private_key.to_string(),
            public_key: pair.public_key,
            algorithm: pair.algorithm,
            comment: pair.comment,
            fingerprint: pair.fingerprint,
            encrypted: pair.encrypted,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn generate_ssh_key(
    key_type: ssh::SshKeyType,
    comment: String,
    passphrase: Option<String>,
) -> CoreResult<SshKeyDto> {
    ssh::generate(key_type, &comment, passphrase.as_deref()).map(Into::into)
}

#[tauri::command]
#[specta::specta]
pub fn import_ssh_key(private_key: String, passphrase: Option<String>) -> CoreResult<SshKeyDto> {
    ssh::import(&private_key, passphrase.as_deref()).map(Into::into)
}

/// Writes the private key of an `ssh_key` secret to a file (0600 on Unix).
#[tauri::command]
#[specta::specta]
pub fn export_ssh_key(state: State<'_, AppState>, id: Id, path: String) -> CoreResult<()> {
    let key = with_secrets(&state, |repo| {
        repo.reveal(
            id,
            &FieldRef::Field {
                key: "private_key".into(),
            },
        )
    })?;
    ssh::export_private_key(&key, std::path::Path::new(&path))
}
