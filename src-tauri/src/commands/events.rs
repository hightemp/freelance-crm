use crm_core::calendar::events::{EditScope, Event, EventInput, EventRepo, Occurrence};
use crm_core::calendar::ics::{self, ParsedIcs};
use crm_core::calendar::import::{self, ImportLink, ImportReport};
use crm_core::calendar::items::{self, CalendarItem, CalendarLayers};
use crm_core::calendar::reminders::{Reminder, ReminderRepo};
use crm_core::domain::{EntityRef, Id, Timestamp};
use crm_core::{CoreError, CoreResult};
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

#[tauri::command]
#[specta::specta]
pub fn list_reminders(state: State<'_, AppState>, owner: EntityRef) -> CoreResult<Vec<Reminder>> {
    with_conn(&state, |conn, clock| {
        ReminderRepo::new(conn, clock).list(owner)
    })
}

/// Replaces the reminders of an event or task with these offsets.
#[tauri::command]
#[specta::specta]
pub fn set_reminders(
    state: State<'_, AppState>,
    owner: EntityRef,
    minutes_before: Vec<u32>,
) -> CoreResult<Vec<Reminder>> {
    with_conn(&state, |conn, clock| {
        ReminderRepo::new(conn, clock).set(owner, &minutes_before)
    })
}

/// Calendar files larger than this are refused.
const MAX_ICS_BYTES: u64 = 20 * 1024 * 1024;

fn read_ics(path: &std::path::Path) -> CoreResult<ParsedIcs> {
    let size = std::fs::metadata(path)
        .map_err(|_| CoreError::new("file.read_failed"))?
        .len();
    if size > MAX_ICS_BYTES {
        return Err(CoreError::new("file.too_large").with("max", MAX_ICS_BYTES));
    }
    let bytes = std::fs::read(path).map_err(|_| CoreError::new("file.read_failed"))?;
    let tz = TimeZone::system();
    Ok(ics::parse(
        &String::from_utf8_lossy(&bytes),
        tz.iana_name().unwrap_or("UTC"),
    ))
}

/// What an .ics file contains, for a preview before importing.
#[tauri::command(async)]
#[specta::specta]
pub fn preview_ics(path: std::path::PathBuf) -> CoreResult<ParsedIcs> {
    read_ics(&path)
}

/// Imports the selected events (all if `uids` is `None`).
#[tauri::command(async)]
#[specta::specta]
pub fn import_ics(
    state: State<'_, AppState>,
    path: std::path::PathBuf,
    uids: Option<Vec<String>>,
    link: ImportLink,
) -> CoreResult<ImportReport> {
    let parsed = read_ics(&path)?;
    with_conn(&state, |conn, clock| {
        import::import(conn, clock, &parsed, uids.as_deref(), link)
    })
}
