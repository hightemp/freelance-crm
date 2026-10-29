use crm_core::CoreResult;
use crm_core::domain::Id;
use crm_core::finance::invoices::{Invoice, InvoiceInput, InvoiceRepo};
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
