use crm_core::CoreResult;
use crm_core::finance::dashboard::{self, Dashboard, Period};
use jiff::tz::TimeZone;
use tauri::State;

use super::with_conn;
use crate::AppState;

/// Income, expenses, open invoices and taxes for a period of dates.
#[tauri::command]
#[specta::specta]
pub fn finance_dashboard(state: State<'_, AppState>, period: Period) -> CoreResult<Dashboard> {
    with_conn(&state, |conn, clock| {
        dashboard::dashboard(conn, clock, TimeZone::system(), &period)
    })
}
