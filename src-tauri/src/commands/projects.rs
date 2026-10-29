use crm_core::CoreResult;
use crm_core::domain::Id;
use crm_core::projects::{Project, ProjectFilter, ProjectInput, ProjectRepo};
use tauri::State;

use super::with_conn;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_projects(
    state: State<'_, AppState>,
    filter: ProjectFilter,
) -> CoreResult<Vec<Project>> {
    with_conn(&state, |conn, clock| {
        ProjectRepo::new(conn, clock).list(&filter)
    })
}

#[tauri::command]
#[specta::specta]
pub fn get_project(state: State<'_, AppState>, id: Id) -> CoreResult<Project> {
    with_conn(&state, |conn, clock| ProjectRepo::new(conn, clock).get(id))
}

#[tauri::command]
#[specta::specta]
pub fn create_project(state: State<'_, AppState>, input: ProjectInput) -> CoreResult<Project> {
    with_conn(&state, |conn, clock| {
        ProjectRepo::new(conn, clock).create(input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_project(
    state: State<'_, AppState>,
    id: Id,
    input: ProjectInput,
) -> CoreResult<Project> {
    with_conn(&state, |conn, clock| {
        ProjectRepo::new(conn, clock).update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn archive_project(state: State<'_, AppState>, id: Id) -> CoreResult<Project> {
    with_conn(&state, |conn, clock| {
        ProjectRepo::new(conn, clock).archive(id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn restore_project(state: State<'_, AppState>, id: Id) -> CoreResult<Project> {
    with_conn(&state, |conn, clock| {
        ProjectRepo::new(conn, clock).restore(id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_project(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| {
        ProjectRepo::new(conn, clock).delete(id)
    })
}
