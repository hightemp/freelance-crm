//! Launch at login (FR-ST-6) behind a trait, so that commands can be
//! tested without touching the OS.

use std::sync::Mutex;

pub trait Autostart: Send + Sync {
    fn is_enabled(&self) -> Result<bool, String>;
    fn set(&self, enabled: bool) -> Result<(), String>;
}

/// Login items on macOS, the registry on Windows, an autostart entry on
/// Linux (tauri-plugin-autostart).
pub struct SystemAutostart(pub tauri::AppHandle);

impl Autostart for SystemAutostart {
    fn is_enabled(&self) -> Result<bool, String> {
        use tauri_plugin_autostart::ManagerExt;
        self.0.autolaunch().is_enabled().map_err(|e| e.to_string())
    }

    fn set(&self, enabled: bool) -> Result<(), String> {
        use tauri_plugin_autostart::ManagerExt;
        let launcher = self.0.autolaunch();
        if enabled {
            launcher.enable()
        } else {
            launcher.disable()
        }
        .map_err(|e| e.to_string())
    }
}

/// For tests and e2e runs.
#[derive(Default)]
pub struct MemoryAutostart(Mutex<bool>);

impl Autostart for MemoryAutostart {
    fn is_enabled(&self) -> Result<bool, String> {
        Ok(*self.0.lock().unwrap())
    }

    fn set(&self, enabled: bool) -> Result<(), String> {
        *self.0.lock().unwrap() = enabled;
        Ok(())
    }
}
