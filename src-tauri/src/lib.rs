pub mod autostart;
mod backups;
mod bindings;
pub mod clipboard;
mod commands;
pub mod desktop;
#[cfg(any(debug_assertions, test))]
mod dev;
pub mod keychain;
pub mod notifier;
mod paths;
mod protocol;
mod reminders;
mod security;
mod tray;
pub mod updates;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crm_core::db::Db;
use crm_core::domain::{Clock, SystemClock};
use crm_core::files::blobs::BlobStore;
use crm_core::files::open::OpenFiles;
use crm_core::vault::Vault;
use crm_core::vault::crypto::KdfParams;
use tauri::{Emitter, Manager};

use crate::clipboard::{Clipboard, SystemClipboard};
use crate::desktop::{Desktop, SystemDesktop};
use crate::notifier::{Notifier, SystemNotifier};

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
    /// The data folder; stored files live under it.
    pub files_root: PathBuf,
    pub open_files: OpenFiles,
    pub desktop: Box<dyn Desktop>,
    pub notifier: Box<dyn Notifier>,
    /// Keeps the passphrase of automatic backups, if the user allows it.
    pub keychain: Box<dyn keychain::Keychain>,
    pub autostart: Box<dyn autostart::Autostart>,
    pub updates: Box<dyn updates::Updates>,
}

/// What the app uses of the OS; tests pass fakes.
pub struct Services {
    pub clipboard: Box<dyn Clipboard>,
    pub desktop: Box<dyn Desktop>,
    pub notifier: Box<dyn Notifier>,
    pub keychain: Box<dyn keychain::Keychain>,
    pub autostart: Box<dyn autostart::Autostart>,
    pub updates: Box<dyn updates::Updates>,
}

impl AppState {
    pub fn new(
        db: Db,
        clock: Arc<dyn Clock>,
        kdf: KdfParams,
        files_root: PathBuf,
        services: Services,
    ) -> Self {
        let Services {
            clipboard,
            desktop,
            notifier,
            keychain,
            autostart,
            updates,
        } = services;
        let autolock = crm_core::settings::load(&db)
            .map(|s| s.vault_autolock_minutes)
            .unwrap_or(crm_core::vault::state::DEFAULT_AUTOLOCK_MINUTES);
        let vault = Vault::new(kdf, clock.clone());
        vault.set_autolock_minutes(autolock);
        let open_files = OpenFiles::new(&files_root);
        open_files.clear_on_lock(&vault);
        Self {
            db: Mutex::new(db),
            clock,
            vault,
            clipboard,
            files_root,
            open_files,
            desktop,
            notifier,
            keychain,
            autostart,
            updates,
        }
    }

    /// Cleans up after a previous run: copies opened from the app (possibly
    /// decrypted, left by a crash) and files nothing references any more.
    pub fn start_up(&self) -> crm_core::CoreResult<()> {
        self.open_files.clear_all();
        let db = self.db.lock().expect("db mutex poisoned");
        BlobStore::new(&self.files_root, db.conn(), self.clock.as_ref()).collect_garbage()?;
        Ok(())
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

    // Must be registered first: a second launch focuses the running window.
    // E2E runs have their own data and may run beside an open copy of the app.
    #[cfg(not(feature = "e2e"))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        tray::show_main(app);
    }));

    builder
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(specta.invoke_handler())
        .register_asynchronous_uri_scheme_protocol(protocol::SCHEME, |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let path = request.uri().path().to_owned();
            // Decrypting a large image must not block the webview.
            std::thread::spawn(move || {
                responder.respond(protocol::respond(&app.state::<AppState>(), &path));
            });
        })
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
                dir.clone(),
                Services {
                    clipboard: Box::new(SystemClipboard(app.handle().clone())),
                    desktop: Box::new(SystemDesktop(app.handle().clone())),
                    notifier: Box::new(SystemNotifier(app.handle().clone())),
                    // E2E runs must not touch the real keychain.
                    keychain: if cfg!(feature = "e2e") {
                        Box::new(keychain::MemoryKeychain::default())
                    } else {
                        Box::new(keychain::SystemKeychain)
                    },
                    autostart: if cfg!(feature = "e2e") {
                        Box::new(autostart::MemoryAutostart::default())
                    } else {
                        Box::new(autostart::SystemAutostart(app.handle().clone()))
                    },
                    // E2E runs must not download anything.
                    updates: if cfg!(feature = "e2e") {
                        Box::new(updates::ScriptedUpdates::default())
                    } else {
                        Box::new(updates::SystemUpdates::new(app.handle().clone()))
                    },
                },
            ));
            app.state::<AppState>().start_up()?;
            // Record reminders missed while closed before the UI asks.
            let _ = reminders::tick(&app.state::<AppState>(), &jiff::tz::TimeZone::system());

            // E2E sessions end by closing the window: it must really close.
            #[cfg(not(feature = "e2e"))]
            {
                let russian =
                    crm_core::settings::load(&app.state::<AppState>().db.lock().expect("db"))
                        .map(|s| s.locale == Some(crm_core::settings::Locale::Ru))
                        .unwrap_or(false)
                        || std::env::var("LANG").is_ok_and(|l| l.starts_with("ru"));
                // No tray (e.g. no indicator on Linux): closing quits as usual.
                if let Err(e) = tray::install(app, russian) {
                    eprintln!("tray icon unavailable: {e}");
                }
            }

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(VAULT_TICK);
                    let state = handle.state::<AppState>();
                    if state.vault.tick() {
                        let _ = handle.emit(VAULT_LOCKED_EVENT, ());
                    }
                    if let Ok(true) = reminders::tick(&state, &jiff::tz::TimeZone::system()) {
                        let _ = handle.emit(reminders::MISSED_EVENT, ());
                    }
                    if backups::tick(&state).is_some() {
                        let _ = handle.emit(backups::DONE_EVENT, ());
                    }
                    tray::refresh(&handle);
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event
                && let Some(state) = app.try_state::<AppState>()
            {
                state.open_files.clear_all();
            }
        });
}
