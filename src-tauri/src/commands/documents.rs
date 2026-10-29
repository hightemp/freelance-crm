use base64::Engine;
use crm_core::domain::Id;
use crm_core::finance::document;
use crm_core::profile_images::{ImageKind, ProfileImages};
use crm_core::{CoreError, CoreResult};
use tauri::State;

use super::with_conn;
use crate::AppState;

/// Renders an invoice's PDF from its document data and profile images.
fn render(state: &AppState, id: Id) -> CoreResult<(String, Vec<u8>)> {
    let (data, number, images) = with_conn(state, |conn, clock| {
        let show_signature = crm_core::settings::load_from(conn)?.invoice_show_signature;
        let doc = document::document(conn, clock, id, show_signature)?;
        let profile = ProfileImages::new(conn, clock);
        let mut images = Vec::new();
        for (name, kind) in [
            ("logo", ImageKind::Logo),
            ("signature", ImageKind::Signature),
            ("stamp", ImageKind::Stamp),
        ] {
            if let Some(image) = profile.get(kind)? {
                images.push((name, image.bytes));
            }
        }
        let data = serde_json::to_value(&doc).expect("documents serialize");
        Ok((data, doc.number, images))
    })?;
    let refs: Vec<(&str, &[u8])> = images.iter().map(|(n, b)| (*n, b.as_slice())).collect();
    let pdf = crm_pdf::render_invoice(&data, &refs)
        .map_err(|e| CoreError::new("pdf.render_failed").with("reason", e.to_string()))?;
    Ok((number, pdf))
}

/// Saves the invoice as PDF at a path chosen in the save dialog.
#[tauri::command(async)]
#[specta::specta]
pub fn save_invoice_pdf(
    state: State<'_, AppState>,
    id: Id,
    path: std::path::PathBuf,
) -> CoreResult<()> {
    let (_, pdf) = render(&state, id)?;
    std::fs::write(&path, pdf).map_err(|_| CoreError::new("file.write_failed"))
}

/// Opens the invoice's PDF in the system viewer (a preview copy).
#[tauri::command(async)]
#[specta::specta]
pub fn open_invoice_pdf(state: State<'_, AppState>, id: Id) -> CoreResult<()> {
    let (number, pdf) = render(&state, id)?;
    let dir = state.files_root.join("open").join("plain").join("invoices");
    std::fs::create_dir_all(&dir).map_err(|_| CoreError::new("file.write_failed"))?;
    let safe: String = number
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || "-_.".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = dir.join(format!("{safe}.pdf"));
    std::fs::write(&path, pdf).map_err(|_| CoreError::new("file.write_failed"))?;
    state
        .desktop
        .open_path(&path)
        .map_err(|_| CoreError::new("file.open_failed"))
}

/// Sets a profile image (logo, signature or stamp) from a PNG or JPEG file.
#[tauri::command(async)]
#[specta::specta]
pub fn set_profile_image(
    state: State<'_, AppState>,
    kind: ImageKind,
    path: std::path::PathBuf,
) -> CoreResult<()> {
    let metadata = std::fs::metadata(&path).map_err(|_| CoreError::new("file.read_failed"))?;
    if metadata.len() > crm_core::profile_images::MAX_BYTES as u64 {
        return Err(
            CoreError::new("file.too_large").with("max", crm_core::profile_images::MAX_BYTES)
        );
    }
    let bytes = std::fs::read(&path).map_err(|_| CoreError::new("file.read_failed"))?;
    with_conn(&state, |conn, clock| {
        ProfileImages::new(conn, clock).set(kind, bytes)
    })
}

#[tauri::command]
#[specta::specta]
pub fn delete_profile_image(state: State<'_, AppState>, kind: ImageKind) -> CoreResult<()> {
    with_conn(&state, |conn, clock| {
        ProfileImages::new(conn, clock).delete(kind)
    })
}

/// The image as a data URL for previews, if set.
#[tauri::command]
#[specta::specta]
pub fn profile_image(state: State<'_, AppState>, kind: ImageKind) -> CoreResult<Option<String>> {
    with_conn(&state, |conn, clock| {
        Ok(ProfileImages::new(conn, clock).get(kind)?.map(|image| {
            format!(
                "data:{};base64,{}",
                image.mime,
                base64::engine::general_purpose::STANDARD.encode(image.bytes)
            )
        }))
    })
}
