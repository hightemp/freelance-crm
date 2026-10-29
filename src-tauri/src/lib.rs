mod bindings;
pub mod clipboard;
mod commands;
#[cfg(any(debug_assertions, test))]
mod dev;
mod paths;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crm_core::db::Db;
use crm_core::domain::{Clock, SystemClock};
use crm_core::vault::Vault;
use crm_core::vault::crypto::KdfParams;
use tauri::{Emitter, Manager};

use crate::clipboard::{Clipboard, SystemClipboard};

/// How often the vault checks for inactivity and sleep.
const VAULT_TICK: Duration = Duration::from_secs(15);

/// Emitted when the vault locks by itself (inactivity or sleep).
pub const VAULT_LOCKED_EVENT: &str = "vault://locked";

/// Shared application state. Commands lock the database for the duration
/// of a single use case.
pub struct AppState {
    pub db: Mutex<Db>,
    /// Injected so that tests can use a fake clock.
    pub clock: Arc<dyn Clock>,
    pub vault: Vault,
    pub clipboard: Box<dyn Clipboard>,
}

impl AppState {
    pub fn new(
        db: Db,
        clock: Arc<dyn Clock>,
        kdf: KdfParams,
        clipboard: Box<dyn Clipboard>,
    ) -> Self {
        let autolock = crm_core::settings::load(&db)
            .map(|s| s.vault_autolock_minutes)
            .unwrap_or(crm_core::vault::state::DEFAULT_AUTOLOCK_MINUTES);
        let vault = Vault::new(kdf, clock.clone());
        vault.set_autolock_minutes(autolock);
        Self {
            db: Mutex::new(db),
            clock,
            vault,
            clipboard,
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    dev::bypass_proxy_for_dev_server();

    let specta = bindings::builder::<tauri::Wry>();
    #[cfg(debug_assertions)]
    specta
        .export(bindings::exporter(), bindings::BINDINGS_PATH)
        .expect("failed to export typescript bindings");

    let builder = tauri::Builder::default();
    // Embedded WebDriver for e2e tests only; see the `e2e` feature.
    #[cfg(feature = "e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio_webdriver::init())
        .plugin(tauri_plugin_wdio::init());

    builder
        // Must be registered first: a second launch focuses the running window.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(specta.invoke_handler())
        .setup(|app| {
            let dir = paths::data_dir(
                std::env::var_os(paths::DATA_DIR_ENV),
                app.path().app_data_dir()?,
            );
            let db_path = dir.join("crm.db");
            let db = Db::open(&db_path)?;
            app.manage(AppState::new(
                db,
                Arc::new(SystemClock),
                KdfParams::default(),
                Box::new(SystemClipboard(app.handle().clone())),
            ));

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(VAULT_TICK);
                    if handle.state::<AppState>().vault.tick() {
                        let _ = handle.emit(VAULT_LOCKED_EVENT, ());
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
