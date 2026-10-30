use crm_core::CoreResult;
use crm_core::countries::{self, Labels};
use crm_core::domain::Id;
use crm_core::finance::taxes::{self, TaxLine, TaxRegime, TaxRegimeInput, TaxRepo};
use crm_core::requisites::{RequisitesOwner, RequisitesService};
use serde::Serialize;
use tauri::State;

use super::with_conn;
use crate::AppState;

/// A preset offered for the freelancer's country.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaxPresetOption {
    pub key: String,
    pub labels: Labels,
}

/// Presets of the profile's country that can be computed (NPD, USN
/// "income"); "custom" is always available and not listed.
#[tauri::command]
#[specta::specta]
pub fn tax_presets(state: State<'_, AppState>) -> CoreResult<Vec<TaxPresetOption>> {
    with_conn(&state, |conn, clock| {
        let profile = RequisitesService::new(conn, clock).get(RequisitesOwner::Profile)?;
        let schema = profile.and_then(|p| countries::get(&p.country));
        Ok(schema
            .map(|s| {
                s.tax_presets
                    .iter()
                    .filter(|p| taxes::preset(&p.key, "2000-01-01").is_some())
                    .map(|p| TaxPresetOption {
                        key: p.key.clone(),
                        labels: p.labels.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default())
    })
}

/// The rules of a preset, to start a regime from.
#[tauri::command]
#[specta::specta]
pub fn tax_preset(key: String, valid_from: String) -> Option<TaxRegimeInput> {
    taxes::preset(&key, &valid_from)
}

#[tauri::command]
#[specta::specta]
pub fn list_tax_regimes(state: State<'_, AppState>) -> CoreResult<Vec<TaxRegime>> {
    with_conn(&state, |conn, clock| TaxRepo::new(conn, clock).regimes())
}

#[tauri::command]
#[specta::specta]
pub fn save_tax_regime(
    state: State<'_, AppState>,
    id: Option<Id>,
    input: TaxRegimeInput,
) -> CoreResult<TaxRegime> {
    with_conn(&state, |conn, clock| {
        TaxRepo::new(conn, clock).save(id, input)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_tax_regime(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    with_conn(&state, |conn, clock| TaxRepo::new(conn, clock).delete(id))
}

/// Estimated tax on payments in `[from, to]`; `None` when switched off.
#[tauri::command]
#[specta::specta]
pub fn tax_estimate(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> CoreResult<Option<Vec<TaxLine>>> {
    with_conn(&state, |conn, clock| {
        TaxRepo::new(conn, clock).estimate(&from, &to)
    })
}
