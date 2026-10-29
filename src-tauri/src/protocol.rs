//! The `crmfile://` protocol serving attachment images to the webview:
//! `/thumb/<id>` and `/full/<id>`. Encrypted images are decrypted in memory
//! and never cached.

use crm_core::CoreError;
use crm_core::domain::Id;
use crm_core::files::attachments::MAX_PREVIEW_SIZE;
use tauri::http::{Response, StatusCode, header};

use crate::AppState;
use crate::commands::with_files;

pub const SCHEME: &str = "crmfile";

pub fn respond(state: &AppState, path: &str) -> Response<Vec<u8>> {
    // `convertFileSrc()` percent-encodes the whole path, slashes included.
    let path = path.trim_start_matches('/').replace("%2F", "/");
    let Some((kind, id)) = path.split_once('/') else {
        return status(StatusCode::NOT_FOUND);
    };
    let Ok(id) = id.parse::<Id>() else {
        return status(StatusCode::NOT_FOUND);
    };
    let result = with_files(state, |files| {
        let attachment = files.get(id)?;
        let body = match kind {
            "thumb" => files.thumbnail(id)?,
            "full"
                if attachment.mime.starts_with("image/") && attachment.size <= MAX_PREVIEW_SIZE =>
            {
                let mut body = Vec::new();
                files.write_to(id, &mut body)?;
                body
            }
            _ => return Err(CoreError::new("attachment.no_preview")),
        };
        let mime = if kind == "thumb" {
            "image/png".to_owned()
        } else {
            attachment.mime
        };
        Ok((body, mime, attachment.encrypted))
    });
    match result {
        Ok((body, mime, encrypted)) => Response::builder()
            .header(header::CONTENT_TYPE, mime)
            .header(
                header::CACHE_CONTROL,
                if encrypted {
                    "no-store"
                } else {
                    "max-age=31536000, immutable"
                },
            )
            .body(body)
            .expect("valid response"),
        Err(e) if e.code == "vault.locked" => status(StatusCode::FORBIDDEN),
        Err(e) if e.code.ends_with("not_found") || e.code.starts_with("attachment.no_") => {
            status(StatusCode::NOT_FOUND)
        }
        Err(_) => status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(code)
        .body(Vec::new())
        .expect("valid response")
}
