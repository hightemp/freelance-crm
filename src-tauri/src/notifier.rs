//! System notifications behind a trait, so that the reminder scheduler can
//! be tested without showing anything.

use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

pub trait Notifier: Send + Sync {
    fn notify(&self, title: &str, body: &str) -> Result<(), String>;
}

pub struct SystemNotifier<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> Notifier for SystemNotifier<R> {
    fn notify(&self, title: &str, body: &str) -> Result<(), String> {
        self.0
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|e| e.to_string())
    }
}
