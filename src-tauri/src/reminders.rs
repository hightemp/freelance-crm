//! The reminder scheduler: each tick checks what is due, shows system
//! notifications and reports reminders missed while the app was closed.

use crm_core::CoreResult;
use crm_core::calendar::reminders::{Candidate, ReminderRepo};
use jiff::tz::TimeZone;

use crate::AppState;

/// Emitted when reminders were missed while the app was closed.
pub const MISSED_EVENT: &str = "reminders://missed";

/// Notifies about due reminders; returns whether any were missed.
pub fn tick(state: &AppState, tz: &TimeZone) -> CoreResult<bool> {
    let due = {
        let db = state.db.lock().expect("db mutex poisoned");
        ReminderRepo::new(db.conn(), state.clock.as_ref()).check(tz)?
    };
    for d in due.iter().filter(|d| !d.missed) {
        // A failed notification is not retried: it is logged as handled.
        let _ = state
            .notifier
            .notify(&d.candidate.title, &when(&d.candidate, tz));
    }
    Ok(due.iter().any(|d| d.missed))
}

/// "14:30" on the reminder's day, else with the date; the date alone for
/// all-day items. Language-neutral, as the backend has no UI locale.
fn when(c: &Candidate, tz: &TimeZone) -> String {
    let at = |ms: i64| {
        jiff::Timestamp::from_millisecond(ms)
            .unwrap_or_default()
            .to_zoned(tz.clone())
    };
    let (start, fire) = (at(c.start_at.as_millis()), at(c.fire_at.as_millis()));
    if c.all_day {
        // All-day starts are floating UTC dates.
        return at(c.start_at.as_millis())
            .with_time_zone(TimeZone::UTC)
            .date()
            .to_string();
    }
    if start.date() == fire.date() {
        start.strftime("%H:%M").to_string()
    } else {
        start.strftime("%Y-%m-%d %H:%M").to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crm_core::calendar::events::{EventInput, EventRepo};
    use crm_core::db::Db;
    use crm_core::domain::{EntityRef, FakeClock, Timestamp};
    use crm_core::vault::crypto::KdfParams;

    use super::*;
    use crate::clipboard::{Clipboard, RgbaImage};
    use crate::desktop::Desktop;
    use crate::notifier::Notifier;

    #[derive(Clone, Default)]
    struct Recording(Arc<Mutex<Vec<(String, String)>>>);

    impl Notifier for Recording {
        fn notify(&self, title: &str, body: &str) -> Result<(), String> {
            self.0.lock().unwrap().push((title.into(), body.into()));
            Ok(())
        }
    }

    struct Nothing;

    impl Clipboard for Nothing {
        fn write_text(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn read_image(&self) -> Result<Option<RgbaImage>, String> {
            Ok(None)
        }
    }

    impl Desktop for Nothing {
        fn open_path(&self, _: &std::path::Path) -> Result<(), String> {
            Ok(())
        }
        fn reveal(&self, _: &std::path::Path) -> Result<(), String> {
            Ok(())
        }
    }

    fn utc(s: &str) -> Timestamp {
        Timestamp::from_millis(s.parse::<jiff::Timestamp>().unwrap().as_millisecond())
    }

    fn app(dir: &std::path::Path, clock: Arc<FakeClock>, notifier: Recording) -> AppState {
        AppState::new(
            Db::open(&dir.join("crm.db")).unwrap(),
            clock,
            KdfParams {
                m_kib: 64,
                t: 1,
                p: 1,
            },
            Box::new(Nothing),
            dir.to_owned(),
            Box::new(Nothing),
            Box::new(notifier),
        )
    }

    #[test]
    fn notifies_once_even_across_restarts() {
        let dir = tempfile::tempdir().unwrap();
        let clock = Arc::new(FakeClock::at(utc("2026-03-10T09:00:00Z")));
        let sent = Recording::default();
        let state = app(dir.path(), clock.clone(), sent.clone());
        {
            let db = state.db.lock().unwrap();
            let event = EventRepo::new(db.conn(), clock.as_ref())
                .create(EventInput {
                    title: "Созвон".into(),
                    description_md: String::new(),
                    location: None,
                    all_day: false,
                    start: "2026-03-10T10:00".into(),
                    end: "2026-03-10T11:00".into(),
                    tzid: "UTC".into(),
                    client_id: None,
                    project_id: None,
                    task_id: None,
                    contact_id: None,
                    rrule: None,
                })
                .unwrap();
            ReminderRepo::new(db.conn(), clock.as_ref())
                .set(EntityRef::Event(event.id), &[15])
                .unwrap();
        }
        assert!(!tick(&state, &TimeZone::UTC).unwrap());
        assert!(sent.0.lock().unwrap().is_empty());
        clock.advance_millis(45 * 60_000);
        tick(&state, &TimeZone::UTC).unwrap();
        tick(&state, &TimeZone::UTC).unwrap();
        assert_eq!(
            *sent.0.lock().unwrap(),
            [("Созвон".to_owned(), "10:00".to_owned())]
        );

        drop(state);
        let restarted = app(dir.path(), clock.clone(), sent.clone());
        tick(&restarted, &TimeZone::UTC).unwrap();
        assert_eq!(sent.0.lock().unwrap().len(), 1);
    }

    #[test]
    fn missed_reminders_are_reported_not_notified() {
        let dir = tempfile::tempdir().unwrap();
        let clock = Arc::new(FakeClock::at(utc("2026-03-10T08:00:00Z")));
        let sent = Recording::default();
        let state = app(dir.path(), clock.clone(), sent.clone());
        {
            let db = state.db.lock().unwrap();
            let event = EventRepo::new(db.conn(), clock.as_ref())
                .create(EventInput {
                    title: "Созвон".into(),
                    description_md: String::new(),
                    location: None,
                    all_day: false,
                    start: "2026-03-10T10:00".into(),
                    end: "2026-03-10T11:00".into(),
                    tzid: "UTC".into(),
                    client_id: None,
                    project_id: None,
                    task_id: None,
                    contact_id: None,
                    rrule: None,
                })
                .unwrap();
            ReminderRepo::new(db.conn(), clock.as_ref())
                .set(EntityRef::Event(event.id), &[60])
                .unwrap();
        }
        tick(&state, &TimeZone::UTC).unwrap();
        clock.advance_millis(90 * 60_000);
        assert!(tick(&state, &TimeZone::UTC).unwrap());
        assert!(sent.0.lock().unwrap().is_empty());
    }

    #[test]
    fn notification_text_shows_the_start() {
        let c = |start: &str, fire: &str, all_day: bool| Candidate {
            reminder_id: crm_core::domain::Id::new(),
            owner: EntityRef::Task(crm_core::domain::Id::new()),
            occurrence: String::new(),
            title: String::new(),
            fire_at: utc(fire),
            start_at: utc(start),
            all_day,
        };
        let moscow = TimeZone::get("Europe/Moscow").unwrap();
        assert_eq!(
            when(
                &c("2026-03-10T15:00:00Z", "2026-03-10T14:00:00Z", false),
                &moscow
            ),
            "18:00"
        );
        assert_eq!(
            when(
                &c("2026-03-11T15:00:00Z", "2026-03-10T15:00:00Z", false),
                &moscow
            ),
            "2026-03-11 18:00"
        );
        assert_eq!(
            when(
                &c("2026-03-13T00:00:00Z", "2026-03-12T06:00:00Z", true),
                &moscow
            ),
            "2026-03-13"
        );
    }
}
