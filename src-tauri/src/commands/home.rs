use crm_core::CoreResult;
use crm_core::home::{self, HomeSummary};
use jiff::tz::TimeZone;
use tauri::State;

use super::with_conn;
use crate::AppState;

/// What the home screen shows: tasks, events, invoices and recent clients.
#[tauri::command]
#[specta::specta]
pub fn home_summary(state: State<'_, AppState>) -> CoreResult<HomeSummary> {
    with_conn(&state, |conn, clock| {
        home::summary(conn, clock, &TimeZone::system())
    })
}
