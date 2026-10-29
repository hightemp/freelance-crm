use crm_core::CoreResult;
use crm_core::domain::Id;
use crm_core::finance::from_time::FromTimeInput;
use crm_core::finance::invoices::{Invoice, InvoiceInput, InvoiceRepo};
use crm_core::time_entries::TimeEntry;
use crm_core::time_report::{self, TimeFilter};
use jiff::tz::TimeZone;
use tauri::State;

use super::with_conn;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_invoices(
    state: State<'_, AppState>,
    client_id: Option<Id>,
    project_id: Option<Id>,
) -> CoreResult<Vec<Invoice>> {
    with_conn(&state, |conn, clock| {
        InvoiceRepo::new(conn, clock)
            .in_zone(TimeZone::system())
            .list(client_id, project_id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn get_invoice(state: State<'_, AppState>, id: Id) -> CoreResult<Invoice> {
    with_conn(&state, |conn, clock| {
        InvoiceRepo::new(conn, clock)
            .in_zone(TimeZone::system())
            .get(id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn create_invoice(state: State<'_, AppState>, input: InvoiceInput) -> CoreResult<Invoice> {
    with_conn(&state, |conn, clock| {
        InvoiceRepo::new(conn, clock)
            .in_zone(TimeZone::system())
            .create(input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_invoice(
    state: State<'_, AppState>,
    id: Id,
    input: InvoiceInput,
) -> CoreResult<Invoice> {
    with_conn(&state, |conn, clock| {
        InvoiceRepo::new(conn, clock)
            .in_zone(TimeZone::system())
            .update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_invoice(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| {
        InvoiceRepo::new(conn, clock).delete(id)
    })
}

/// The number the next invoice issued on `issue_date` would get.
#[tauri::command]
#[specta::specta]
pub fn preview_invoice_number(
    state: State<'_, AppState>,
    issue_date: String,
) -> CoreResult<String> {
    with_conn(&state, |conn, _| {
        let template = crm_core::settings::load_from(conn)?.invoice_number_template;
        crm_core::finance::numbering::preview(conn, &template, &issue_date)
    })
}

/// Bills unbilled time as a draft invoice, rounded per the settings.
#[tauri::command]
#[specta::specta]
pub fn invoice_from_time(state: State<'_, AppState>, input: FromTimeInput) -> CoreResult<Invoice> {
    with_conn(&state, |conn, clock| {
        let rounding = crm_core::settings::load_from(conn)?.time_rounding_minutes;
        InvoiceRepo::new(conn, clock)
            .in_zone(TimeZone::system())
            .from_time(input, rounding)
    })
}

/// Billable time not on an invoice yet, for the "from time" wizard.
#[tauri::command]
#[specta::specta]
pub fn unbilled_time(state: State<'_, AppState>, filter: TimeFilter) -> CoreResult<Vec<TimeEntry>> {
    with_conn(&state, |conn, _| time_report::unbilled(conn, filter))
}
