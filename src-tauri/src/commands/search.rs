use crm_core::CoreResult;
use crm_core::search::{self, SearchHit};
use tauri::State;

use super::with_conn;
use crate::AppState;

/// Global search: the best `limit` records matching every word by prefix.
#[tauri::command]
#[specta::specta]
pub fn search(state: State<'_, AppState>, query: String, limit: u32) -> CoreResult<Vec<SearchHit>> {
    with_conn(&state, |conn, _| search::search(conn, &query, limit))
}
