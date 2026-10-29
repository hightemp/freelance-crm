use crm_core::CoreResult;
use crm_core::custom_fields::{CustomField, CustomFieldInput, CustomFieldRepo};
use crm_core::domain::{EntityRef, Id};
use tauri::State;

use super::with_conn;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_custom_fields(
    state: State<'_, AppState>,
    owner: EntityRef,
) -> CoreResult<Vec<CustomField>> {
    with_conn(&state, |conn, clock| {
        CustomFieldRepo::new(conn, clock).list(owner)
    })
}

#[tauri::command]
#[specta::specta]
pub fn create_custom_field(
    state: State<'_, AppState>,
    owner: EntityRef,
    input: CustomFieldInput,
) -> CoreResult<CustomField> {
    with_conn(&state, |conn, clock| {
        CustomFieldRepo::new(conn, clock).create(owner, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_custom_field(
    state: State<'_, AppState>,
    id: Id,
    input: CustomFieldInput,
) -> CoreResult<CustomField> {
    with_conn(&state, |conn, clock| {
        CustomFieldRepo::new(conn, clock).update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_custom_field(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| {
        CustomFieldRepo::new(conn, clock).delete(id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn reorder_custom_fields(
    state: State<'_, AppState>,
    owner: EntityRef,
    ids: Vec<Id>,
) -> CoreResult<Vec<CustomField>> {
    with_conn(&state, |conn, clock| {
        CustomFieldRepo::new(conn, clock).reorder(owner, &ids)
    })
}
