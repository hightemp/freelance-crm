//! Importing parsed iCalendar events (FR-CA-7). An event whose UID is
//! already known is updated, not duplicated; edited occurrences are laid
//! over their series; alarms become reminders.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::events::{EditScope, EventInput, EventRepo};
use super::ics::{IcsEvent, IcsIssue, ParsedIcs};
use super::reminders::ReminderRepo;
use crate::domain::{Clock, EntityRef, Id};
use crate::{CoreError, CoreResult};

/// What imported events get linked to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportLink {
    pub client_id: Option<Id>,
    pub project_id: Option<Id>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub created: u32,
    pub updated: u32,
    /// Problems found while reading plus events that could not be saved.
    pub issues: Vec<IcsIssue>,
}

/// Imports the events whose UIDs are in `uids` (all if `None`).
pub fn import(
    conn: &Connection,
    clock: &dyn Clock,
    parsed: &ParsedIcs,
    uids: Option<&[String]>,
    link: ImportLink,
) -> CoreResult<ImportReport> {
    let selected = |e: &IcsEvent| uids.is_none_or(|u| u.contains(&e.uid));
    let mut report = ImportReport {
        issues: parsed.issues.clone(),
        ..ImportReport::default()
    };
    // All or nothing for database failures; bad events are only skipped.
    let tx = conn.unchecked_transaction()?;
    let events = EventRepo::new(&tx, clock);
    let reminders = ReminderRepo::new(&tx, clock);
    let mut failed = |e: &IcsEvent, error: CoreError| {
        report.issues.push(IcsIssue {
            uid: Some(e.uid.clone()),
            line: 0,
            error,
            warning: false,
        });
    };
    let mut created = 0;
    let mut updated = 0;
    // Series first, so that their edited occurrences find them.
    for e in parsed
        .events
        .iter()
        .filter(|e| e.recurrence_id.is_none() && selected(e))
    {
        let saved = (|| -> CoreResult<bool> {
            let existing = events.find_by_uid(&e.uid)?;
            let id = match existing {
                Some(id) => events.update(id, input(e, link))?.id,
                None => events.create_with_uid(input(e, link), &e.uid)?.id,
            };
            if e.rrule.is_some() {
                events.set_exdates(id, &e.exdates)?;
            }
            reminders.set(EntityRef::Event(id), &e.reminders)?;
            Ok(existing.is_some())
        })();
        match saved {
            Ok(true) => updated += 1,
            Ok(false) => created += 1,
            Err(error) => failed(e, error),
        }
    }
    for e in parsed
        .events
        .iter()
        .filter(|e| e.recurrence_id.is_some() && selected(e))
    {
        let rid = e.recurrence_id.as_deref().expect("filtered");
        let saved = (|| -> CoreResult<bool> {
            let series = events
                .find_by_uid(&e.uid)?
                .ok_or_else(|| CoreError::new("ics.orphan_occurrence"))?;
            let existed = events.override_of(series, rid)?.is_some();
            events.update_occurrence(series, rid, EditScope::This, input(e, link))?;
            Ok(existed)
        })();
        match saved {
            Ok(true) => updated += 1,
            Ok(false) => created += 1,
            Err(error) => failed(e, error),
        }
    }
    tx.commit()?;
    report.created = created;
    report.updated = updated;
    Ok(report)
}

fn input(e: &IcsEvent, link: ImportLink) -> EventInput {
    EventInput {
        // Titles are required; untitled entries exist in the wild.
        title: if e.title.is_empty() {
            "—".into()
        } else {
            e.title.clone()
        },
        description_md: e.description.clone(),
        location: e.location.clone(),
        all_day: e.all_day,
        start: e.start.clone(),
        end: e.end.clone(),
        tzid: e.tzid.clone(),
        client_id: link.client_id,
        project_id: link.project_id,
        task_id: None,
        contact_id: None,
        rrule: e.rrule.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::ics::parse;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::{FakeClock, Timestamp};

    const GOOGLE: &str = include_str!("../../tests/fixtures/ics/google.ics");
    const BROKEN: &str = include_str!("../../tests/fixtures/ics/broken.ics");

    fn utc(s: &str) -> Timestamp {
        Timestamp::from_millis(s.parse::<jiff::Timestamp>().unwrap().as_millisecond())
    }

    struct Fixture {
        db: Db,
        clock: FakeClock,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                db: Db::open_in_memory().unwrap(),
                clock: FakeClock::at(utc("2026-03-01T00:00:00Z")),
            }
        }

        fn import(&self, text: &str, uids: Option<&[String]>, link: ImportLink) -> ImportReport {
            import(
                self.db.conn(),
                &self.clock,
                &parse(text, "Europe/Moscow"),
                uids,
                link,
            )
            .unwrap()
        }

        fn march(&self) -> Vec<(String, String)> {
            EventRepo::new(self.db.conn(), &self.clock)
                .occurrences(utc("2026-03-01T00:00:00Z"), utc("2026-04-01T00:00:00Z"))
                .unwrap()
                .into_iter()
                .map(|o| (o.start, o.event.title))
                .collect()
        }

        fn count(&self) -> i64 {
            self.db
                .conn()
                .query_row("SELECT count(*) FROM events", [], |r| r.get(0))
                .unwrap()
        }
    }

    #[test]
    fn imports_series_overrides_and_reminders() {
        let f = Fixture::new();
        let report = f.import(GOOGLE, None, ImportLink::default());
        assert_eq!((report.created, report.updated), (4, 0));
        assert_eq!(report.issues, []);
        // Occurrences in each event's own zone.
        let want: Vec<(String, String)> = [
            ("2026-03-02T10:00", "Планёрка"),
            ("2026-03-10T12:00", "Планёрка (перенос)"),
            (
                "2026-03-20T15:00",
                "Call with a very long title that will certainly be folded by the exporter \
                 because lines are limited to 75 octets",
            ),
            ("2026-03-23T10:00", "Планёрка"),
            ("2026-03-30T10:00", "Планёрка"),
        ]
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
        assert_eq!(f.march(), want);

        let events = EventRepo::new(f.db.conn(), &f.clock);
        let standup: Id = f
            .db
            .conn()
            .query_row(
                "SELECT id FROM events WHERE uid = 'standup-1@google.com' AND series_id IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let standup = events.get(standup).unwrap();
        assert_eq!(standup.uid, "standup-1@google.com");
        assert_eq!(standup.exdates, ["2026-03-09T10:00", "2026-03-16T10:00"]);
        assert_eq!(
            ReminderRepo::new(f.db.conn(), &f.clock)
                .list(EntityRef::Event(standup.id))
                .unwrap()
                .iter()
                .map(|r| r.minutes_before)
                .collect::<Vec<_>>(),
            [10]
        );
    }

    #[test]
    fn importing_again_updates_instead_of_duplicating() {
        let f = Fixture::new();
        f.import(GOOGLE, None, ImportLink::default());
        let changed = GOOGLE.replace("SUMMARY:Майские", "SUMMARY:Майские праздники");
        let report = f.import(&changed, None, ImportLink::default());
        assert_eq!((report.created, report.updated), (0, 4));
        assert_eq!(f.count(), 4);
        let title: String =
            f.db.conn()
                .query_row(
                    "SELECT title FROM events WHERE uid = 'holidays@google.com'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
        assert_eq!(title, "Майские праздники");
    }

    #[test]
    fn imports_selected_events_linked_to_a_client() {
        let f = Fixture::new();
        let client = ClientRepo::new(f.db.conn(), &f.clock)
            .create(ClientInput {
                kind: ClientKind::Company,
                last_name: None,
                first_name: None,
                middle_name: None,
                company_name: Some("Ромашка".into()),
                legal_name: None,
                status: ClientStatus::Active,
                default_currency: None,
                default_rate_minor: None,
            })
            .unwrap();
        let only = ["holidays@google.com".to_owned()];
        let report = f.import(
            GOOGLE,
            Some(&only),
            ImportLink {
                client_id: Some(client.id),
                project_id: None,
            },
        );
        assert_eq!(report.created, 1);
        let linked: Option<Id> =
            f.db.conn()
                .query_row("SELECT client_id FROM events", [], |r| r.get(0))
                .unwrap();
        assert_eq!(linked, Some(client.id));
    }

    #[test]
    fn broken_files_import_what_they_can() {
        let f = Fixture::new();
        let report = f.import(BROKEN, None, ImportLink::default());
        assert_eq!(report.created, 2);
        assert_eq!(report.issues.len(), 4);
        assert_eq!(f.count(), 2);
    }

    #[test]
    fn occurrences_without_their_series_are_reported() {
        let f = Fixture::new();
        let text = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:lonely\r\n\
                    RECURRENCE-ID:20260310T100000Z\r\nDTSTART:20260310T120000Z\r\nSUMMARY:x\r\n\
                    END:VEVENT\r\nEND:VCALENDAR\r\n";
        let report = f.import(text, None, ImportLink::default());
        assert_eq!(report.created, 0);
        assert_eq!(
            report.issues[0].error,
            CoreError::new("ics.orphan_occurrence")
        );
        assert_eq!(report.issues[0].uid.as_deref(), Some("lonely"));
    }
}
