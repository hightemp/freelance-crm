use crm_core::CoreResult;
use crm_core::domain::{EntityRef, Id};
use crm_core::notes::{Note, NoteInput, NoteRepo};
use tauri::State;

use super::with_conn;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_notes(state: State<'_, AppState>, owner: EntityRef) -> CoreResult<Vec<Note>> {
    with_conn(&state, |conn, clock| NoteRepo::new(conn, clock).list(owner))
}

#[tauri::command]
#[specta::specta]
pub fn create_note(
    state: State<'_, AppState>,
    owner: EntityRef,
    input: NoteInput,
) -> CoreResult<Note> {
    with_conn(&state, |conn, clock| {
        NoteRepo::new(conn, clock).create(owner, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_note(state: State<'_, AppState>, id: Id, input: NoteInput) -> CoreResult<Note> {
    with_conn(&state, |conn, clock| {
        NoteRepo::new(conn, clock).update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn set_note_pinned(state: State<'_, AppState>, id: Id, pinned: bool) -> CoreResult<Note> {
    with_conn(&state, |conn, clock| {
        NoteRepo::new(conn, clock).set_pinned(id, pinned)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_note(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| NoteRepo::new(conn, clock).delete(id))
}
