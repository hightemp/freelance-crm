use crm_core::CoreResult;
use crm_core::domain::Id;
use crm_core::time_entries::{SavedEntry, TimeEntry, TimeEntryInput, TimeRepo, TimeTarget};
use crm_core::time_report::{self, GroupBy, TimeFilter, TimeReport};
use crm_core::{CoreError, settings};
use jiff::tz::TimeZone;
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

/// Finished entries for the journal, newest first.
#[tauri::command]
#[specta::specta]
pub fn list_time_entries(
    state: State<'_, AppState>,
    filter: TimeFilter,
) -> CoreResult<Vec<TimeEntry>> {
    with_conn(&state, |conn, _| time_report::entries(conn, filter))
}

/// Totals by day (in the system zone), project or task, rounded per the
/// settings.
#[tauri::command]
#[specta::specta]
pub fn time_report(
    state: State<'_, AppState>,
    filter: TimeFilter,
    group: GroupBy,
) -> CoreResult<TimeReport> {
    with_conn(&state, |conn, _| {
        let rounding = settings::load_from(conn)?.time_rounding_minutes;
        time_report::report(conn, filter, group, &TimeZone::system(), rounding)
    })
}

/// Writes the entries as CSV to `path` with the UI's column headers.
#[tauri::command(async)]
#[specta::specta]
pub fn export_time_csv(
    state: State<'_, AppState>,
    path: std::path::PathBuf,
    filter: TimeFilter,
    headers: Vec<String>,
) -> CoreResult<()> {
    let text = with_conn(&state, |conn, _| {
        let rounding = settings::load_from(conn)?.time_rounding_minutes;
        time_report::csv(conn, filter, &TimeZone::system(), rounding, &headers)
    })?;
    std::fs::write(&path, text).map_err(|_| CoreError::new("file.write_failed"))
}
