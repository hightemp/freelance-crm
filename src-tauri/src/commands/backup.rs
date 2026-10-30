use crm_core::CoreResult;
use crm_core::backup::restore::{self, Restored};
use crm_core::backup::{self, Manifest};
use tauri::State;

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
    snapshot.seal(&state.files_root, &path, &passphrase, state.clock.as_ref())
}

/// Replaces the current data with a backup, after backing it up. The vault
/// locks: its keys now come from the backup.
#[tauri::command(async)]
#[specta::specta]
pub fn restore_backup(
    state: State<'_, AppState>,
    path: std::path::PathBuf,
    passphrase: String,
) -> CoreResult<Restored> {
    let restored = {
        let mut db = state.db.lock().expect("db mutex poisoned");
        restore::restore(
            &mut db,
            &state.files_root,
            &path,
            &passphrase,
            state.clock.as_ref(),
        )?
    };
    state.vault.lock();
    Ok(restored)
}
