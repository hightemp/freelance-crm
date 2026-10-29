use crm_core::CoreResult;
use crm_core::checklists::{ChecklistItem, ChecklistRepo};
use crm_core::domain::Id;
use crm_core::tasks::{Task, TaskFilter, TaskInput, TaskRepo};
use jiff::tz::TimeZone;
use tauri::State;

use super::with_conn;
use crate::AppState;

/// Deadline presets use the OS time zone, read on every call so that a
/// change while the app runs is picked up.
#[tauri::command]
#[specta::specta]
pub fn list_tasks(state: State<'_, AppState>, filter: TaskFilter) -> CoreResult<Vec<Task>> {
    with_conn(&state, |conn, clock| {
        TaskRepo::new(conn, clock).list(&filter, &TimeZone::system())
    })
}

#[tauri::command]
#[specta::specta]
pub fn get_task(state: State<'_, AppState>, id: Id) -> CoreResult<Task> {
    with_conn(&state, |conn, clock| TaskRepo::new(conn, clock).get(id))
}

#[tauri::command]
#[specta::specta]
pub fn create_task(state: State<'_, AppState>, input: TaskInput) -> CoreResult<Task> {
    with_conn(&state, |conn, clock| {
        TaskRepo::new(conn, clock).create(input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_task(state: State<'_, AppState>, id: Id, input: TaskInput) -> CoreResult<Task> {
    with_conn(&state, |conn, clock| {
        TaskRepo::new(conn, clock).update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn set_task_status(state: State<'_, AppState>, id: Id, status_id: Id) -> CoreResult<Task> {
    with_conn(&state, |conn, clock| {
        TaskRepo::new(conn, clock).set_status(id, status_id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn move_task(
    state: State<'_, AppState>,
    id: Id,
    status_id: Id,
    after: Option<Id>,
    before: Option<Id>,
) -> CoreResult<Task> {
    with_conn(&state, |conn, clock| {
        TaskRepo::new(conn, clock).move_task(id, status_id, after, before)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_task(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| TaskRepo::new(conn, clock).delete(id))
}

#[tauri::command]
#[specta::specta]
pub fn list_checklist(state: State<'_, AppState>, task_id: Id) -> CoreResult<Vec<ChecklistItem>> {
    with_conn(&state, |conn, clock| {
        ChecklistRepo::new(conn, clock).list(task_id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn add_checklist_item(
    state: State<'_, AppState>,
    task_id: Id,
    text: String,
) -> CoreResult<ChecklistItem> {
    with_conn(&state, |conn, clock| {
        ChecklistRepo::new(conn, clock).add(task_id, &text)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_checklist_item(
    state: State<'_, AppState>,
    id: Id,
    text: String,
    done: bool,
) -> CoreResult<ChecklistItem> {
    with_conn(&state, |conn, clock| {
        ChecklistRepo::new(conn, clock).update(id, &text, done)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_checklist_item(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| {
        ChecklistRepo::new(conn, clock).delete(id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn reorder_checklist(
    state: State<'_, AppState>,
    task_id: Id,
    ids: Vec<Id>,
) -> CoreResult<Vec<ChecklistItem>> {
    with_conn(&state, |conn, clock| {
        ChecklistRepo::new(conn, clock).reorder(task_id, &ids)
    })
}
