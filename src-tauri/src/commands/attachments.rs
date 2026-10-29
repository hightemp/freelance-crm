use std::path::{Path, PathBuf};

use crm_core::domain::{EntityRef, Id};
use crm_core::files::attachments::{Attachment, Attachments};
use crm_core::files::{open, thumbnails};
use crm_core::{CoreError, CoreResult};
use tauri::State;

use crate::AppState;

/// Runs a use case with the attachments of the app's data folder.
pub(crate) fn with_files<T>(
    state: &AppState,
    f: impl FnOnce(&Attachments) -> CoreResult<T>,
) -> CoreResult<T> {
    let db = state.db.lock().expect("db mutex poisoned");
    let files = Attachments::new(
        &state.files_root,
        db.conn(),
        state.clock.as_ref(),
        &state.vault,
    );
    f(&files)
}

fn desktop_error(_: String) -> CoreError {
    CoreError::new("file.open_failed")
}

#[tauri::command]
#[specta::specta]
pub fn list_attachments(
    state: State<'_, AppState>,
    owner: EntityRef,
) -> CoreResult<Vec<Attachment>> {
    with_files(&state, |files| files.list(owner))
}

/// Copies a file into the app; runs off the main thread (files can be big).
#[tauri::command(async)]
#[specta::specta]
pub fn attach_file(
    state: State<'_, AppState>,
    owner: EntityRef,
    path: PathBuf,
    encrypt: bool,
) -> CoreResult<Attachment> {
    with_files(&state, |files| files.attach_path(owner, &path, encrypt))
}

#[tauri::command(async)]
#[specta::specta]
pub fn delete_attachment(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_files(&state, |files| files.delete(id))
}

#[tauri::command(async)]
#[specta::specta]
pub fn open_attachment(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    let path = with_files(&state, |files| state.open_files.copy_for_opening(files, id))?;
    state.desktop.open_path(&path).map_err(desktop_error)
}

/// Shows a copy in the file manager; decrypted copies are never revealed.
#[tauri::command(async)]
#[specta::specta]
pub fn reveal_attachment(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    let path = with_files(&state, |files| {
        if files.get(id)?.encrypted {
            return Err(CoreError::new("attachment.encrypted"));
        }
        state.open_files.copy_for_opening(files, id)
    })?;
    state.desktop.reveal(&path).map_err(desktop_error)
}

/// "Save as" to a path chosen in the system dialog.
#[tauri::command(async)]
#[specta::specta]
pub fn save_attachment_as(state: State<'_, AppState>, id: Id, path: PathBuf) -> CoreResult<()> {
    with_files(&state, |files| open::export(files, id, Path::new(&path)))
}

/// Attaches the image on the clipboard as a PNG named `file_name`.
#[tauri::command(async)]
#[specta::specta]
pub fn attach_clipboard_image(
    state: State<'_, AppState>,
    owner: EntityRef,
    file_name: String,
    encrypt: bool,
) -> CoreResult<Attachment> {
    let image = state
        .clipboard
        .read_image()
        .ok()
        .flatten()
        .ok_or_else(|| CoreError::new("clipboard.no_image"))?;
    let png = thumbnails::encode_png(image.width, image.height, image.rgba)
        .ok_or_else(|| CoreError::new("clipboard.no_image"))?;
    with_files(&state, |files| {
        files.attach(owner, &file_name, png.as_slice(), encrypt)
    })
}
