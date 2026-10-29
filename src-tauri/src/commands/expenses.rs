use crm_core::CoreResult;
use crm_core::domain::Id;
use crm_core::finance::expenses::{Expense, ExpenseFilter, ExpenseInput, ExpenseRepo, Profit};
use tauri::State;

use super::with_conn;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_expenses(
    state: State<'_, AppState>,
    filter: ExpenseFilter,
) -> CoreResult<Vec<Expense>> {
    with_conn(&state, |conn, clock| {
        ExpenseRepo::new(conn, clock).list(&filter)
    })
}

#[tauri::command]
#[specta::specta]
pub fn expense_categories(state: State<'_, AppState>) -> CoreResult<Vec<String>> {
    with_conn(&state, |conn, clock| {
        ExpenseRepo::new(conn, clock).categories()
    })
}

#[tauri::command]
#[specta::specta]
pub fn create_expense(state: State<'_, AppState>, input: ExpenseInput) -> CoreResult<Expense> {
    with_conn(&state, |conn, clock| {
        ExpenseRepo::new(conn, clock).create(input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_expense(
    state: State<'_, AppState>,
    id: Id,
    input: ExpenseInput,
) -> CoreResult<Expense> {
    with_conn(&state, |conn, clock| {
        ExpenseRepo::new(conn, clock).update(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_expense(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| {
        ExpenseRepo::new(conn, clock).delete(id)
    })
}

#[tauri::command]
#[specta::specta]
pub fn project_profit(state: State<'_, AppState>, project_id: Id) -> CoreResult<Vec<Profit>> {
    with_conn(&state, |conn, clock| {
        ExpenseRepo::new(conn, clock).project_profit(project_id)
    })
}
