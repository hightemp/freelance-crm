use crm_core::CoreResult;
use crm_core::domain::{EntityRef, Id};
use crm_core::tags::{Tag, TagInput, TagService};
use tauri::State;

use super::with_conn;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_tags(state: State<'_, AppState>) -> CoreResult<Vec<Tag>> {
    with_conn(&state, |conn, clock| TagService::new(conn, clock).list())
}

#[tauri::command]
#[specta::specta]
pub fn create_tag(state: State<'_, AppState>, input: TagInput) -> CoreResult<Tag> {
    with_conn(&state, |conn, clock| {
        TagService::new(conn, clock).create(input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_tag(state: State<'_, AppState>, id: Id, input: TagInput) -> CoreResult<Tag> {
    with_conn(&state, |conn, clock| {
        TagService::new(conn, clock).update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_tag(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| {
        TagService::new(conn, clock).delete(id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn tags_of(state: State<'_, AppState>, target: EntityRef) -> CoreResult<Vec<Tag>> {
    with_conn(&state, |conn, clock| {
        TagService::new(conn, clock).tags_of(target)
    })
}

#[tauri::command]
#[specta::specta]
pub fn set_tags(
    state: State<'_, AppState>,
    target: EntityRef,
    tag_ids: Vec<Id>,
) -> CoreResult<Vec<Tag>> {
    with_conn(&state, |conn, clock| {
        TagService::new(conn, clock).set_tags(target, &tag_ids)
    })
}
