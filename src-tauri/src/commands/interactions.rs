use crm_core::CoreResult;
use crm_core::calendar::events::EventInput;
use crm_core::domain::Id;
use crm_core::interactions::{
    FeedCursor, FeedPage, FollowUp, Interaction, InteractionInput, InteractionKind, InteractionRepo,
};
use tauri::State;

use super::with_conn;
use crate::AppState;

/// A client's interactions, newest first; pass `next` back for more.
#[tauri::command]
#[specta::specta]
pub fn interaction_feed(
    state: State<'_, AppState>,
    client_id: Id,
    kind: Option<InteractionKind>,
    after: Option<FeedCursor>,
    limit: u32,
) -> CoreResult<FeedPage> {
    with_conn(&state, |conn, clock| {
        InteractionRepo::new(conn, clock).feed(client_id, kind, after, limit)
    })
}

#[tauri::command]
#[specta::specta]
pub fn create_interaction(
    state: State<'_, AppState>,
    input: InteractionInput,
) -> CoreResult<Interaction> {
    with_conn(&state, |conn, clock| {
        InteractionRepo::new(conn, clock).create(input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_interaction(
    state: State<'_, AppState>,
    id: Id,
    input: InteractionInput,
) -> CoreResult<Interaction> {
    with_conn(&state, |conn, clock| {
        InteractionRepo::new(conn, clock).update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_interaction(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| {
        InteractionRepo::new(conn, clock).delete(id)
    })
}

/// A task for the interaction's client and project, quoting it.
#[tauri::command]
#[specta::specta]
pub fn follow_up_task(
    state: State<'_, AppState>,
    id: Id,
    title: String,
    due_date: Option<String>,
) -> CoreResult<FollowUp> {
    with_conn(&state, |conn, clock| {
        InteractionRepo::new(conn, clock).follow_up_task(id, &title, due_date)
    })
}

/// An event for the interaction's client, project and contact.
#[tauri::command]
#[specta::specta]
pub fn follow_up_event(
    state: State<'_, AppState>,
    id: Id,
    input: EventInput,
) -> CoreResult<FollowUp> {
    with_conn(&state, |conn, clock| {
        InteractionRepo::new(conn, clock).follow_up_event(id, input)
    })
}
