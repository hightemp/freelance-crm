use crm_core::backup::auto::{self, AutoBackupStatus};
use crm_core::backup::restore::{self, Restored};
use crm_core::backup::{self, BackupKey, KeyKind, Manifest, Unlock};
use crm_core::{CoreError, CoreResult};
use serde::Serialize;
use tauri::State;

use super::with_conn;
use crate::AppState;

/// Writes an encrypted backup to `path`. Only the database snapshot holds
/// the connection; packing and encrypting the files run without it.
#[tauri::command(async)]
#[specta::specta]
pub fn export_backup(
    state: State<'_, AppState>,
    path: std::path::PathBuf,
    passphrase: String,
) -> CoreResult<Manifest> {
    backup::check_passphrase(&passphrase)?;
    let snapshot = {
        let db = state.db.lock().expect("db mutex poisoned");
        backup::snapshot(db.conn(), &path)?
    };
    snapshot.seal(
        &state.files_root,
        &path,
        &BackupKey::passphrase(&passphrase),
        state.clock.as_ref(),
    )
}

/// Whether a backup file opens with a passphrase or the master password.
#[tauri::command]
#[specta::specta]
pub fn inspect_backup(path: std::path::PathBuf) -> CoreResult<KeyKind> {
    backup::inspect(&path)
}

/// Replaces the current data with a backup, after backing it up. `secret`
/// is the backup passphrase or the master password, as the file asks. The
/// vault locks: its keys now come from the backup.
#[tauri::command(async)]
#[specta::specta]
pub fn restore_backup(
    state: State<'_, AppState>,
    path: std::path::PathBuf,
    secret: String,
) -> CoreResult<Restored> {
    let unlock = match backup::inspect(&path)? {
        KeyKind::Passphrase => Unlock::Passphrase(&secret),
        KeyKind::Vault => Unlock::MasterPassword(&secret),
    };
    let restored = {
        let mut db = state.db.lock().expect("db mutex poisoned");
        restore::restore(
            &mut db,
            &state.files_root,
            &path,
            unlock,
            state.clock.as_ref(),
        )?
    };
    state.vault.lock();
    Ok(restored)
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AutoBackupInfo {
    #[serde(flatten)]
    pub status: AutoBackupStatus,
    /// A passphrase is stored in the OS keychain.
    pub keychain_ready: bool,
}

#[tauri::command]
#[specta::specta]
pub fn auto_backup_status(state: State<'_, AppState>) -> CoreResult<AutoBackupInfo> {
    let status = with_conn(&state, |conn, _| auto::status(conn))?;
    Ok(AutoBackupInfo {
        status,
        keychain_ready: state.keychain.get().ok().flatten().is_some(),
    })
}

/// Stores the passphrase of automatic backups in the OS keychain, or
/// removes it (`None`).
#[tauri::command]
#[specta::specta]
pub fn set_backup_passphrase(
    state: State<'_, AppState>,
    passphrase: Option<String>,
) -> CoreResult<()> {
    let keychain_failed = |_| CoreError::new("backup.keychain_unavailable");
    match passphrase {
        Some(p) => {
            backup::check_passphrase(&p)?;
            state.keychain.set(&p).map_err(keychain_failed)
        }
        None => state.keychain.delete().map_err(keychain_failed),
    }
}

/// Prepares vault-locked automatic backups; the vault must be unlocked.
#[tauri::command]
#[specta::specta]
pub fn set_up_vault_backup_key(state: State<'_, AppState>) -> CoreResult<()> {
    with_conn(&state, |conn, _| auto::set_up_vault_key(conn, &state.vault))
}

/// Writes the non-secret data as JSON and CSV into `dir`; returns the file
/// names.
#[tauri::command(async)]
#[specta::specta]
pub fn export_data(state: State<'_, AppState>, dir: std::path::PathBuf) -> CoreResult<Vec<String>> {
    with_conn(&state, |conn, clock| {
        crm_core::export::write_all(conn, clock, &dir)
    })
}
