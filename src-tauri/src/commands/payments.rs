use crm_core::CoreResult;
use crm_core::domain::{Id, Money};
use crm_core::finance::payments::{Payment, PaymentInput, PaymentRepo, SavedPayment};
use tauri::State;

use super::with_conn;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn create_payment(state: State<'_, AppState>, input: PaymentInput) -> CoreResult<SavedPayment> {
    with_conn(&state, |conn, clock| {
        PaymentRepo::new(conn, clock).create(input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_payment(
    state: State<'_, AppState>,
    id: Id,
    input: PaymentInput,
) -> CoreResult<SavedPayment> {
    with_conn(&state, |conn, clock| {
        PaymentRepo::new(conn, clock).update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_payment(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| {
        PaymentRepo::new(conn, clock).delete(id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn invoice_payments(state: State<'_, AppState>, invoice_id: Id) -> CoreResult<Vec<Payment>> {
    with_conn(&state, |conn, clock| {
        PaymentRepo::new(conn, clock).for_invoice(invoice_id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn project_income(state: State<'_, AppState>, project_id: Id) -> CoreResult<Vec<Money>> {
    with_conn(&state, |conn, clock| {
        PaymentRepo::new(conn, clock).project_income(project_id)
    })
}
