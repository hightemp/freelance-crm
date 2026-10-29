//! The system clipboard behind a trait, so that commands copying secrets
//! can be tested without touching the real clipboard.

use tauri::{AppHandle, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;

pub trait Clipboard: Send + Sync {
    fn write_text(&self, text: &str) -> Result<(), String>;
}

pub struct SystemClipboard<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> Clipboard for SystemClipboard<R> {
    fn write_text(&self, text: &str) -> Result<(), String> {
        self.0
            .clipboard()
            .write_text(text.to_owned())
            .map_err(|e| e.to_string())
    }
}
