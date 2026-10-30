//! The tray icon: the app keeps running (and reminding) with the window
//! closed, unless the settings say closing quits (FR-ST-6); the tray
//! brings it back, stops the timer, locks the vault or quits. Its tooltip
//! shows the running timer.

use crm_core::time_entries::{TimeEntry, TimeRepo};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime, WindowEvent};

use crate::AppState;

/// Emitted when the timer changes outside the UI (the tray).
pub const TIMER_CHANGED_EVENT: &str = "timer://changed";

/// The tray's "Stop timer" item, enabled while a timer runs.
pub struct TimerItem<R: Runtime>(pub MenuItem<R>);

/// Installs the tray and makes closing the main window hide it. Without a
/// tray (e.g. no indicator support on Linux) closing quits as usual.
#[cfg_attr(feature = "e2e", allow(dead_code))]
pub fn install<R: Runtime>(app: &tauri::App<R>, russian: bool) -> tauri::Result<()> {
    let [open, stop, lock, quit] = menu_labels(russian);
    let open = MenuItem::with_id(app, open.0, open.1, true, None::<&str>)?;
    let stop = MenuItem::with_id(app, stop.0, stop.1, false, None::<&str>)?;
    let lock = MenuItem::with_id(app, lock.0, lock.1, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, quit.0, quit.1, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &stop, &lock, &quit])?;
    app.manage(TimerItem(stop));
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("freelance-crm")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "stop_timer" => {
                let state = app.state::<AppState>();
                let stopped = {
                    let db = state.db.lock().expect("db mutex poisoned");
                    TimeRepo::new(db.conn(), state.clock.as_ref()).stop()
                };
                if stopped.is_ok() {
                    let _ = app.emit(TIMER_CHANGED_EVENT, ());
                    refresh(app);
                }
            }
            "lock_vault" => {
                app.state::<AppState>().vault.lock();
                let _ = app.emit(crate::VAULT_LOCKED_EVENT, ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;

    if let Some(window) = app.get_webview_window("main") {
        let hidden = window.clone();
        let handle = app.handle().clone();
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event
                && hides_on_close(&handle.state::<AppState>())
            {
                api.prevent_close();
                let _ = hidden.hide();
            }
        });
    }
    Ok(())
}

/// The tray menu's items (id, label) in order.
fn menu_labels(russian: bool) -> [(&'static str, &'static str); 4] {
    if russian {
        [
            ("open", "Открыть"),
            ("stop_timer", "Остановить таймер"),
            ("lock_vault", "Заблокировать хранилище"),
            ("quit", "Выйти"),
        ]
    } else {
        [
            ("open", "Open"),
            ("stop_timer", "Stop timer"),
            ("lock_vault", "Lock the vault"),
            ("quit", "Quit"),
        ]
    }
}

/// Whether closing the window hides it to the tray (the setting, on by
/// default) rather than quitting.
pub fn hides_on_close(state: &AppState) -> bool {
    let db = state.db.lock().expect("db mutex poisoned");
    crm_core::settings::load(&db)
        .map(|s| s.close_to_tray)
        .unwrap_or(true)
}

/// Shows, restores and focuses the main window.
pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Updates the tooltip and the stop item from the running timer.
pub fn refresh<R: Runtime>(app: &AppHandle<R>) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };
    let state = app.state::<AppState>();
    let running = {
        let db = state.db.lock().expect("db mutex poisoned");
        TimeRepo::new(db.conn(), state.clock.as_ref())
            .running()
            .ok()
            .flatten()
    };
    let _ = tray.set_tooltip(Some(tooltip(
        running.as_ref(),
        state.clock.now().as_millis(),
    )));
    if let Some(item) = app.try_state::<TimerItem<R>>() {
        let _ = item.0.set_enabled(running.is_some());
    }
}

/// "freelance-crm", or "⏱ 1:05 · Вёрстка" with the running timer.
fn tooltip(running: Option<&TimeEntry>, now_ms: i64) -> String {
    let Some(entry) = running else {
        return "freelance-crm".into();
    };
    let minutes = (now_ms - entry.started_at.as_millis()).max(0) / 60_000;
    let what = [
        Some(entry.description.as_str()),
        entry.task_title.as_deref(),
        entry.project_name.as_deref(),
    ]
    .into_iter()
    .flatten()
    .find(|s| !s.is_empty());
    let elapsed = format!("⏱ {}:{:02}", minutes / 60, minutes % 60);
    match what {
        Some(what) => format!("{elapsed} · {what}"),
        None => elapsed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crm_core::domain::{Id, Timestamp};

    fn entry(description: &str, task: Option<&str>, project: Option<&str>) -> TimeEntry {
        TimeEntry {
            id: Id::new(),
            task_id: None,
            task_title: task.map(Into::into),
            project_id: None,
            project_name: project.map(Into::into),
            client_id: None,
            client_name: None,
            description: description.into(),
            started_at: Timestamp::from_millis(0),
            ended_at: None,
            billable: true,
            invoice_id: None,
            created_at: Timestamp::from_millis(0),
            updated_at: Timestamp::from_millis(0),
        }
    }

    #[test]
    fn closing_hides_to_the_tray_only_when_enabled() {
        use std::sync::Arc;

        use crm_core::domain::FakeClock;

        let dir = tempfile::tempdir().unwrap();
        let state = crate::reminders::tests::app(
            dir.path(),
            Arc::new(FakeClock::at(Timestamp::from_millis(0))),
            crate::reminders::tests::Recording::default(),
        );
        assert!(hides_on_close(&state), "on by default");
        {
            let mut db = state.db.lock().unwrap();
            crm_core::settings::save(
                &mut db,
                &crm_core::settings::Settings {
                    close_to_tray: false,
                    ..Default::default()
                },
            )
            .unwrap();
        }
        assert!(!hides_on_close(&state));
    }

    #[test]
    fn the_menu_has_the_timer_the_vault_and_quit() {
        let ids: Vec<_> = menu_labels(false).iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, ["open", "stop_timer", "lock_vault", "quit"]);
        assert_eq!(menu_labels(true)[2].1, "Заблокировать хранилище");
    }

    #[test]
    fn tooltip_shows_the_running_timer() {
        assert_eq!(tooltip(None, 0), "freelance-crm");
        let hour = 3_600_000;
        assert_eq!(
            tooltip(
                Some(&entry("Вёрстка", Some("Макет"), None)),
                hour + 5 * 60_000
            ),
            "⏱ 1:05 · Вёрстка"
        );
        assert_eq!(
            tooltip(Some(&entry("", Some("Макет"), Some("Сайт"))), 59_999),
            "⏱ 0:00 · Макет"
        );
        assert_eq!(
            tooltip(Some(&entry("", None, Some("Сайт"))), 10 * hour),
            "⏱ 10:00 · Сайт"
        );
        assert_eq!(tooltip(Some(&entry("", None, None)), 0), "⏱ 0:00");
    }
}
