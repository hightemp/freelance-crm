use crm_core::CoreResult;
use crm_core::clients::{Client, ClientFilter, ClientInput, ClientRepo};
use crm_core::domain::Id;
use tauri::State;

use super::with_conn;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_clients(state: State<'_, AppState>, filter: ClientFilter) -> CoreResult<Vec<Client>> {
    with_conn(&state, |conn, clock| {
        ClientRepo::new(conn, clock).list(&filter)
    })
}

#[tauri::command]
#[specta::specta]
pub fn get_client(state: State<'_, AppState>, id: Id) -> CoreResult<Client> {
    with_conn(&state, |conn, clock| ClientRepo::new(conn, clock).get(id))
}

#[tauri::command]
#[specta::specta]
pub fn create_client(state: State<'_, AppState>, input: ClientInput) -> CoreResult<Client> {
    with_conn(&state, |conn, clock| {
        ClientRepo::new(conn, clock).create(input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_client(state: State<'_, AppState>, id: Id, input: ClientInput) -> CoreResult<Client> {
    with_conn(&state, |conn, clock| {
        ClientRepo::new(conn, clock).update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn archive_client(state: State<'_, AppState>, id: Id) -> CoreResult<Client> {
    with_conn(&state, |conn, clock| {
        ClientRepo::new(conn, clock).archive(id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn restore_client(state: State<'_, AppState>, id: Id) -> CoreResult<Client> {
    with_conn(&state, |conn, clock| {
        ClientRepo::new(conn, clock).restore(id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_client(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| {
        ClientRepo::new(conn, clock).delete(id)
    })
}
