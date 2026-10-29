use crm_core::CoreResult;
use crm_core::domain::Id;
use crm_core::time_entries::{SavedEntry, TimeEntry, TimeEntryInput, TimeRepo, TimeTarget};
use tauri::State;

use super::with_conn;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn timer_running(state: State<'_, AppState>) -> CoreResult<Option<TimeEntry>> {
    with_conn(&state, |conn, clock| TimeRepo::new(conn, clock).running())
}

/// Starts the timer; a running one is stopped or the call refused, per
/// the `timerStopPrevious` setting.
#[tauri::command]
#[specta::specta]
pub fn timer_start(
    state: State<'_, AppState>,
    target: TimeTarget,
    description: String,
    billable: bool,
) -> CoreResult<TimeEntry> {
    with_conn(&state, |conn, clock| {
        let stop_previous = crm_core::settings::load_from(conn)?.timer_stop_previous;
        TimeRepo::new(conn, clock).start(target, &description, billable, stop_previous)
    })
}

#[tauri::command]
#[specta::specta]
pub fn timer_stop(state: State<'_, AppState>) -> CoreResult<TimeEntry> {
    with_conn(&state, |conn, clock| TimeRepo::new(conn, clock).stop())
}

#[tauri::command]
#[specta::specta]
pub fn get_time_entry(state: State<'_, AppState>, id: Id) -> CoreResult<TimeEntry> {
    with_conn(&state, |conn, clock| TimeRepo::new(conn, clock).get(id))
}

#[tauri::command]
#[specta::specta]
pub fn create_time_entry(
    state: State<'_, AppState>,
    input: TimeEntryInput,
) -> CoreResult<SavedEntry> {
    with_conn(&state, |conn, clock| {
        TimeRepo::new(conn, clock).create(input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_time_entry(
    state: State<'_, AppState>,
    id: Id,
    input: TimeEntryInput,
) -> CoreResult<SavedEntry> {
    with_conn(&state, |conn, clock| {
        TimeRepo::new(conn, clock).update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_time_entry(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| TimeRepo::new(conn, clock).delete(id))
}
