use crm_core::CoreResult;
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
