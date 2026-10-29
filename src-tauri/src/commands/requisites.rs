use crm_core::CoreResult;
use crm_core::countries::{self, CountrySchema};
use crm_core::requisites::{
    Requisites, RequisitesInput, RequisitesOwner, RequisitesService, SaveRequisites,
};
use tauri::State;

use super::with_conn;
use crate::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_country_schemas() -> Vec<CountrySchema> {
    countries::builtin().to_vec()
}

#[tauri::command]
#[specta::specta]
pub fn get_requisites(
    state: State<'_, AppState>,
    owner: RequisitesOwner,
) -> CoreResult<Option<Requisites>> {
    with_conn(&state, |conn, clock| {
        RequisitesService::new(conn, clock).get(owner)
    })
}

#[tauri::command]
#[specta::specta]
pub fn save_requisites(
    state: State<'_, AppState>,
    owner: RequisitesOwner,
    input: RequisitesInput,
) -> CoreResult<SaveRequisites> {
    with_conn(&state, |conn, clock| {
        RequisitesService::new(conn, clock).save(owner, input)
    })
}
