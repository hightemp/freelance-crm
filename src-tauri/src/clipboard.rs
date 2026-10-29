//! The system clipboard behind a trait, so that commands copying secrets
//! can be tested without touching the real clipboard.

use tauri::{AppHandle, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;

pub trait Clipboard: Send + Sync {
    fn write_text(&self, text: &str) -> Result<(), String>;
    /// The image on the clipboard, if any.
    fn read_image(&self) -> Result<Option<RgbaImage>, String>;
}

pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct SystemClipboard<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> Clipboard for SystemClipboard<R> {
    fn write_text(&self, text: &str) -> Result<(), String> {
        self.0
            .clipboard()
            .write_text(text.to_owned())
            .map_err(|e| e.to_string())
    }

    fn read_image(&self) -> Result<Option<RgbaImage>, String> {
        // The plugin reports "no image" as an error, like any other failure.
        Ok(self.0.clipboard().read_image().ok().map(|image| RgbaImage {
            width: image.width(),
            height: image.height(),
            rgba: image.rgba().to_vec(),
        }))
    }
}
