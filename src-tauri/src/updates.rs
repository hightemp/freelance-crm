//! New versions from GitHub releases (tauri-plugin-updater) behind a trait,
//! so that commands can be tested without the network.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use serde::Serialize;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A newer version than the running one.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AvailableUpdate {
    pub version: String,
    /// Release notes (Markdown).
    pub notes: Option<String>,
    /// RFC 3339.
    pub date: Option<String>,
}

pub trait Updates: Send + Sync {
    fn check(&self) -> BoxFuture<'_, Result<Option<AvailableUpdate>, String>>;
    /// Downloads, verifies and installs the update found by the last check,
    /// then restarts the app.
    fn install(&self) -> BoxFuture<'_, Result<(), String>>;
}

/// Checks the endpoint in `tauri.conf.json`; the update's signature is
/// verified with the public key there.
pub struct SystemUpdates {
    app: tauri::AppHandle,
    found: Mutex<Option<tauri_plugin_updater::Update>>,
}

impl SystemUpdates {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            found: Mutex::new(None),
        }
    }
}

impl Updates for SystemUpdates {
    fn check(&self) -> BoxFuture<'_, Result<Option<AvailableUpdate>, String>> {
        Box::pin(async move {
            use tauri_plugin_updater::UpdaterExt;
            let update = self
                .app
                .updater()
                .map_err(|e| e.to_string())?
                .check()
                .await
                .map_err(|e| e.to_string())?;
            let available = update.as_ref().map(|u| AvailableUpdate {
                version: u.version.clone(),
                notes: u.body.clone(),
                date: u.date.map(|d| d.to_string()),
            });
            *self.found.lock().expect("updates mutex poisoned") = update;
            Ok(available)
        })
    }

    fn install(&self) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async move {
            let update = self
                .found
                .lock()
                .expect("updates mutex poisoned")
                .take()
                .ok_or("no update was found")?;
            update
                .download_and_install(|_, _| {}, || {})
                .await
                .map_err(|e| e.to_string())?;
            self.app.restart()
        })
    }
}

/// Offers what a test sets; offers nothing by default (e2e runs).
#[derive(Clone, Default)]
pub struct ScriptedUpdates(Arc<Mutex<Script>>);

#[derive(Default)]
struct Script {
    offer: Option<AvailableUpdate>,
    failing: bool,
    installed: bool,
}

impl ScriptedUpdates {
    pub fn offer(&self, update: AvailableUpdate) {
        self.0.lock().unwrap().offer = Some(update);
    }

    pub fn fail_checks(&self) {
        self.0.lock().unwrap().failing = true;
    }

    pub fn installed(&self) -> bool {
        self.0.lock().unwrap().installed
    }
}

impl Updates for ScriptedUpdates {
    fn check(&self) -> BoxFuture<'_, Result<Option<AvailableUpdate>, String>> {
        let script = self.0.lock().unwrap();
        let result = if script.failing {
            Err("offline".into())
        } else {
            Ok(script.offer.clone())
        };
        Box::pin(async move { result })
    }

    fn install(&self) -> BoxFuture<'_, Result<(), String>> {
        let mut script = self.0.lock().unwrap();
        let result = if script.offer.is_some() {
            script.installed = true;
            Ok(())
        } else {
            Err("no update was found".into())
        };
        Box::pin(async move { result })
    }
}
