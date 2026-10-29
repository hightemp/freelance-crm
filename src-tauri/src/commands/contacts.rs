use crm_core::CoreResult;
use crm_core::contact_methods::{ContactMethod, ContactMethodInput, ContactMethodRepo};
use crm_core::contacts::{Contact, ContactInput, ContactRepo};
use crm_core::domain::{EntityRef, Id};
use tauri::State;

use super::with_conn;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_contacts(state: State<'_, AppState>, client_id: Id) -> CoreResult<Vec<Contact>> {
    with_conn(&state, |conn, clock| {
        ContactRepo::new(conn, clock).list(client_id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn create_contact(
    state: State<'_, AppState>,
    client_id: Id,
    input: ContactInput,
) -> CoreResult<Contact> {
    with_conn(&state, |conn, clock| {
        ContactRepo::new(conn, clock).create(client_id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_contact(
    state: State<'_, AppState>,
    id: Id,
    input: ContactInput,
) -> CoreResult<Contact> {
    with_conn(&state, |conn, clock| {
        ContactRepo::new(conn, clock).update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn set_primary_contact(state: State<'_, AppState>, id: Id) -> CoreResult<Contact> {
    with_conn(&state, |conn, clock| {
        ContactRepo::new(conn, clock).set_primary(id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_contact(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| {
        ContactRepo::new(conn, clock).delete(id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn list_contact_methods(
    state: State<'_, AppState>,
    owner: EntityRef,
) -> CoreResult<Vec<ContactMethod>> {
    with_conn(&state, |conn, clock| {
        ContactMethodRepo::new(conn, clock).list(owner)
    })
}

#[tauri::command]
#[specta::specta]
pub fn replace_contact_methods(
    state: State<'_, AppState>,
    owner: EntityRef,
    methods: Vec<ContactMethodInput>,
) -> CoreResult<Vec<ContactMethod>> {
    with_conn(&state, |conn, clock| {
        ContactMethodRepo::new(conn, clock).replace(owner, methods)
    })
}
