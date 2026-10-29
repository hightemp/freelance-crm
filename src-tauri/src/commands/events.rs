use crm_core::CoreResult;
use crm_core::calendar::events::{EditScope, Event, EventInput, EventRepo, Occurrence};
use crm_core::calendar::items::{self, CalendarItem, CalendarLayers};
use crm_core::domain::{Id, Timestamp};
use jiff::tz::TimeZone;
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

/// Occurrences in `[from, to)` with recurring events expanded.
#[tauri::command]
#[specta::specta]
pub fn list_occurrences(
    state: State<'_, AppState>,
    from: Timestamp,
    to: Timestamp,
) -> CoreResult<Vec<Occurrence>> {
    with_conn(&state, |conn, clock| {
        EventRepo::new(conn, clock).occurrences(from, to)
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

/// Edits one occurrence of a series, it and the following ones, or all.
#[tauri::command]
#[specta::specta]
pub fn update_occurrence(
    state: State<'_, AppState>,
    id: Id,
    recurrence_id: String,
    scope: EditScope,
    input: EventInput,
) -> CoreResult<Event> {
    with_conn(&state, |conn, clock| {
        EventRepo::new(conn, clock).update_occurrence(id, &recurrence_id, scope, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_occurrence(
    state: State<'_, AppState>,
    id: Id,
    recurrence_id: String,
    scope: EditScope,
) -> CoreResult<()> {
    with_conn(&state, |conn, clock| {
        EventRepo::new(conn, clock).delete_occurrence(id, &recurrence_id, scope)
    })
}

/// What the calendar shows in `[from, to)`; deadlines use the system zone.
#[tauri::command]
#[specta::specta]
pub fn calendar_items(
    state: State<'_, AppState>,
    from: Timestamp,
    to: Timestamp,
    layers: CalendarLayers,
) -> CoreResult<Vec<CalendarItem>> {
    with_conn(&state, |conn, clock| {
        items::calendar_items(conn, clock, from, to, layers, &TimeZone::system())
    })
}
