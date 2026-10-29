use crm_core::CoreResult;
use crm_core::domain::Id;
use crm_core::statuses::{BoardColumn, StatusInput, StatusService, TaskStatus};
use tauri::State;

use super::with_conn;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_board_columns(state: State<'_, AppState>) -> CoreResult<Vec<BoardColumn>> {
    with_conn(&state, |conn, clock| {
        StatusService::new(conn, clock).columns()
    })
}

#[tauri::command]
#[specta::specta]
pub fn create_status(state: State<'_, AppState>, input: StatusInput) -> CoreResult<TaskStatus> {
    with_conn(&state, |conn, clock| {
        StatusService::new(conn, clock).create(input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_status(
    state: State<'_, AppState>,
    id: Id,
    input: StatusInput,
) -> CoreResult<TaskStatus> {
    with_conn(&state, |conn, clock| {
        StatusService::new(conn, clock).update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn reorder_statuses(state: State<'_, AppState>, ids: Vec<Id>) -> CoreResult<Vec<TaskStatus>> {
    with_conn(&state, |conn, clock| {
        StatusService::new(conn, clock).reorder(&ids)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_status(state: State<'_, AppState>, id: Id, move_to: Option<Id>) -> CoreResult<()> {
    with_conn(&state, |conn, clock| {
        StatusService::new(conn, clock).delete(id, move_to)
    })
}
