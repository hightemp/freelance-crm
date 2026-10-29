//! Opening files with system apps behind a trait, so that commands can be
//! tested without launching anything.

use std::path::Path;

use tauri::{AppHandle, Runtime};
use tauri_plugin_opener::OpenerExt;

pub trait Desktop: Send + Sync {
    /// Opens the file with its default app.
    fn open_path(&self, path: &Path) -> Result<(), String>;
    /// Shows the file in the system file manager.
    fn reveal(&self, path: &Path) -> Result<(), String>;
}

pub struct SystemDesktop<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> Desktop for SystemDesktop<R> {
    fn open_path(&self, path: &Path) -> Result<(), String> {
        self.0
            .opener()
            .open_path(path.to_string_lossy(), None::<&str>)
            .map_err(|e| e.to_string())
    }

    fn reveal(&self, path: &Path) -> Result<(), String> {
        self.0
            .opener()
            .reveal_item_in_dir(path)
            .map_err(|e| e.to_string())
    }
}
