use crm_core::CoreResult;
use crm_core::calendar::events::{Event, EventInput, EventRepo};
use crm_core::domain::{Id, Timestamp};
use tauri::State;

use super::with_conn;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_events(
    state: State<'_, AppState>,
    from: Timestamp,
    to: Timestamp,
) -> CoreResult<Vec<Event>> {
    with_conn(&state, |conn, clock| {
        EventRepo::new(conn, clock).list(from, to)
    })
}

#[tauri::command]
#[specta::specta]
pub fn get_event(state: State<'_, AppState>, id: Id) -> CoreResult<Event> {
    with_conn(&state, |conn, clock| EventRepo::new(conn, clock).get(id))
}

#[tauri::command]
#[specta::specta]
pub fn create_event(state: State<'_, AppState>, input: EventInput) -> CoreResult<Event> {
    with_conn(&state, |conn, clock| {
        EventRepo::new(conn, clock).create(input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_event(state: State<'_, AppState>, id: Id, input: EventInput) -> CoreResult<Event> {
    with_conn(&state, |conn, clock| {
        EventRepo::new(conn, clock).update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_event(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| EventRepo::new(conn, clock).delete(id))
}
