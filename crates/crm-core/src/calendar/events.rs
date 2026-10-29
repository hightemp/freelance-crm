//! Calendar events (FR-CA-2). Times are entered as local wall-clock time in
//! an IANA time zone; both the local values and the UTC instants are
//! stored, so ranges are queried in UTC and events are shown and repeated
//! in their own zone. All-day events are dates: the end date is inclusive
//! and the UTC range covers the days from midnight UTC (floating).

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use super::rrule::Rrule;
use crate::db::ensure_exists;
use crate::domain::{Clock, Id, Timestamp};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EventInput {
    pub title: String,
    pub description_md: String,
    pub location: Option<String>,
    pub all_day: bool,
    /// `YYYY-MM-DDTHH:MM`, or `YYYY-MM-DD` for all-day events.
    pub start: String,
    /// Same format; for all-day events the last day (inclusive).
    pub end: String,
    /// IANA time zone, e.g. `Europe/Moscow`.
    pub tzid: String,
    /// With a project, the client comes from it.
    pub client_id: Option<Id>,
    pub project_id: Option<Id>,
    pub task_id: Option<Id>,
    pub contact_id: Option<Id>,
    /// RFC 5545 RRULE value, e.g. `FREQ=WEEKLY;BYDAY=MO`.
    pub rrule: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: Id,
    pub title: String,
    pub description_md: String,
    pub location: Option<String>,
    pub all_day: bool,
    /// Normalized local values, in the input format.
    pub start: String,
    pub end: String,
    pub tzid: String,
    pub start_at: Timestamp,
    /// Exclusive.
    pub end_at: Timestamp,
    pub client_id: Option<Id>,
    pub client_name: Option<String>,
    pub project_id: Option<Id>,
    pub project_name: Option<String>,
    pub task_id: Option<Id>,
    pub contact_id: Option<Id>,
    /// Canonical RRULE; `None` for single events.
    pub rrule: Option<String>,
    /// Local starts (in the `start` format) of removed occurrences.
    pub exdates: Vec<String>,
    /// iCalendar UID, kept across export and import.
    pub uid: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

pub struct EventRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> EventRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    /// Validates the input and computes the stored values.
    fn resolve(&self, input: EventInput) -> CoreResult<Resolved> {
        let title = input.title.trim().to_owned();
        if title.is_empty() {
            return Err(CoreError::validation("title", "required"));
        }
        let tzid = input.tzid.trim().to_owned();
        let tz = TimeZone::get(&tzid).map_err(|_| CoreError::validation("tzid", "timezone"))?;
        let (start, end, start_at, end_at) = if input.all_day {
            let start =
                parse_date(&input.start).ok_or_else(|| CoreError::validation("start", "date"))?;
            let end = parse_date(&input.end).ok_or_else(|| CoreError::validation("end", "date"))?;
            if end < start {
                return Err(CoreError::validation("end", "before_start"));
            }
            let next = end
                .tomorrow()
                .map_err(|_| CoreError::validation("end", "date"))?;
            (
                start.to_string(),
                end.to_string(),
                utc_midnight(start)?,
                utc_midnight(next)?,
            )
        } else {
            let start = parse_datetime(&input.start)
                .ok_or_else(|| CoreError::validation("start", "datetime"))?;
            let end = parse_datetime(&input.end)
                .ok_or_else(|| CoreError::validation("end", "datetime"))?;
            let (start_at, end_at) = (instant(start, &tz)?, instant(end, &tz)?);
            if end_at < start_at {
                return Err(CoreError::validation("end", "before_start"));
            }
            (format_local(start), format_local(end), start_at, end_at)
        };
        let client_id = match input.project_id {
            Some(project_id) => {
                let owner: Option<Id> = self
                    .conn
                    .query_row(
                        "SELECT client_id FROM projects WHERE id = ?1",
                        [project_id],
                        |r| r.get(0),
                    )
                    .optional()?;
                let owner = owner.ok_or_else(|| CoreError::not_found("project", project_id))?;
                if input.client_id.is_some_and(|c| c != owner) {
                    return Err(CoreError::new("event.client_mismatch"));
                }
                Some(owner)
            }
            None => {
                if let Some(id) = input.client_id {
                    ensure_exists(self.conn, "clients", "client", id)?;
                }
                input.client_id
            }
        };
        if let Some(id) = input.task_id {
            ensure_exists(self.conn, "tasks", "task", id)?;
        }
        if let Some(id) = input.contact_id {
            ensure_exists(self.conn, "contacts", "contact", id)?;
        }
        let rrule = crate::domain::text::clean(input.rrule)
            .map(|r| r.parse::<Rrule>())
            .transpose()?;
        let series = Series::new(
            input.all_day,
            &start,
            &end,
            &tzid,
            rrule.clone(),
            Vec::new(),
        )
        .ok_or_else(|| CoreError::validation("start", "datetime"))?;
        let series_end_at = series.end_at(end_at);
        Ok(Resolved {
            input: EventInput {
                title,
                tzid,
                client_id,
                location: crate::domain::text::clean(input.location),
                start: start.clone(),
                end: end.clone(),
                rrule: rrule.map(|r| r.to_string()),
                ..input
            },
            start_at,
            end_at,
            series_end_at,
        })
    }

    pub fn create(&self, input: EventInput) -> CoreResult<Event> {
        let r = self.resolve(input)?;
        let id = Id::new();
        let now = self.clock.now();
        self.conn.execute(
            "INSERT INTO events (id, uid, title, description_md, location, all_day, start_local,
                 end_local, tzid, start_at, end_at, client_id, project_id, task_id, contact_id,
                 created_at, updated_at, rrule, series_end_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16,
                 ?17, ?18)",
            rusqlite::params![
                id,
                format!("{id}@freelance-crm"),
                r.input.title,
                r.input.description_md,
                r.input.location,
                r.input.all_day,
                r.input.start,
                r.input.end,
                r.input.tzid,
                r.start_at,
                r.end_at,
                r.input.client_id,
                r.input.project_id,
                r.input.task_id,
                r.input.contact_id,
                now,
                r.input.rrule,
                r.series_end_at,
            ],
        )?;
        self.get(id)
    }

    pub fn update(&self, id: Id, input: EventInput) -> CoreResult<Event> {
        self.get(id)?;
        let r = self.resolve(input)?;
        self.conn.execute(
            "UPDATE events SET title = ?2, description_md = ?3, location = ?4, all_day = ?5,
                 start_local = ?6, end_local = ?7, tzid = ?8, start_at = ?9, end_at = ?10,
                 client_id = ?11, project_id = ?12, task_id = ?13, contact_id = ?14,
                 updated_at = ?15, rrule = ?16, series_end_at = ?17
             WHERE id = ?1",
            rusqlite::params![
                id,
                r.input.title,
                r.input.description_md,
                r.input.location,
                r.input.all_day,
                r.input.start,
                r.input.end,
                r.input.tzid,
                r.start_at,
                r.end_at,
                r.input.client_id,
                r.input.project_id,
                r.input.task_id,
                r.input.contact_id,
                self.clock.now(),
                r.input.rrule,
                r.series_end_at,
            ],
        )?;
        self.get(id)
    }

    pub fn get(&self, id: Id) -> CoreResult<Event> {
        self.conn
            .query_row(&format!("{SELECT} WHERE e.id = ?1"), [id], from_row)
            .optional()?
            .ok_or_else(|| CoreError::not_found("event", id))
    }

    /// Events overlapping `[from, to)`, by start. A zero-length event at
    /// `from` counts as inside.
    pub fn list(&self, from: Timestamp, to: Timestamp) -> CoreResult<Vec<Event>> {
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT} WHERE e.start_at < ?2 AND (e.end_at > ?1 OR e.start_at = ?1)
             ORDER BY e.start_at, e.end_at DESC, e.id"
        ))?;
        let rows = stmt.query_map((from, to), from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Occurrences overlapping `[from, to)`: single events as they are and
    /// recurring ones expanded, by start.
    pub fn occurrences(&self, from: Timestamp, to: Timestamp) -> CoreResult<Vec<Occurrence>> {
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT} WHERE e.start_at < ?2 AND (
                 (e.rrule IS NULL AND (e.end_at > ?1 OR e.start_at = ?1))
                 OR (e.rrule IS NOT NULL AND (e.series_end_at IS NULL OR e.series_end_at > ?1)))"
        ))?;
        let events = stmt
            .query_map((from, to), from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        let mut out = Vec::new();
        for event in events {
            if event.rrule.is_none() {
                out.push(Occurrence {
                    start: event.start.clone(),
                    end: event.end.clone(),
                    start_at: event.start_at,
                    end_at: event.end_at,
                    recurrence_id: None,
                    event,
                });
                continue;
            }
            let Some(series) = Series::from_event(&event) else {
                continue; // Unreadable stored rule: skip rather than fail the view.
            };
            for slot in series.occurrences(from, to) {
                out.push(Occurrence {
                    event: event.clone(),
                    start: slot.start,
                    end: slot.end,
                    start_at: slot.start_at,
                    end_at: slot.end_at,
                    recurrence_id: Some(slot.recurrence_id),
                });
            }
        }
        out.sort_by(|a, b| {
            (a.start_at, std::cmp::Reverse(a.end_at), a.event.id).cmp(&(
                b.start_at,
                std::cmp::Reverse(b.end_at),
                b.event.id,
            ))
        });
        Ok(out)
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        if self
            .conn
            .execute("DELETE FROM events WHERE id = ?1", [id])?
            == 0
        {
            return Err(CoreError::not_found("event", id));
        }
        Ok(())
    }
}

/// One occurrence of an event; for single events the event itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Occurrence {
    pub event: Event,
    /// Local start and end of this occurrence (event format).
    pub start: String,
    pub end: String,
    pub start_at: Timestamp,
    pub end_at: Timestamp,
    /// The occurrence's original local start (iCalendar RECURRENCE-ID);
    /// `None` for single events.
    pub recurrence_id: Option<String>,
}

struct Resolved {
    input: EventInput,
    start_at: Timestamp,
    end_at: Timestamp,
    series_end_at: Option<Timestamp>,
}

/// Occurrences a single call expands at most per event (an endless daily
/// series over decades).
const MAX_OCCURRENCES: usize = 50_000;

/// Timing of one occurrence of a series.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Slot {
    pub start: String,
    pub end: String,
    pub start_at: Timestamp,
    pub end_at: Timestamp,
    pub recurrence_id: String,
}

/// An event's timing in local terms, for expansion.
pub(crate) struct Series {
    all_day: bool,
    /// Local start; midnight for all-day events.
    start: DateTime,
    /// Local length (days for all-day events).
    span: jiff::Span,
    /// UTC for all-day events (floating).
    tz: TimeZone,
    rule: Option<Rrule>,
    exdates: Vec<String>,
}

impl Series {
    pub(crate) fn new(
        all_day: bool,
        start: &str,
        end: &str,
        tzid: &str,
        rule: Option<Rrule>,
        exdates: Vec<String>,
    ) -> Option<Self> {
        let (start, end, tz) = if all_day {
            let s = parse_date(start)?.to_datetime(jiff::civil::Time::midnight());
            let e = parse_date(end)?
                .tomorrow()
                .ok()?
                .to_datetime(jiff::civil::Time::midnight());
            (s, e, TimeZone::UTC)
        } else {
            (
                parse_datetime(start)?,
                parse_datetime(end)?,
                TimeZone::get(tzid).ok()?,
            )
        };
        Some(Self {
            all_day,
            start,
            span: start.until(end).ok()?,
            tz,
            rule,
            exdates,
        })
    }

    pub(crate) fn from_event(event: &Event) -> Option<Self> {
        let rule = event.rrule.as_deref().map(str::parse).transpose().ok()?;
        Self::new(
            event.all_day,
            &event.start,
            &event.end,
            &event.tzid,
            rule,
            event.exdates.clone(),
        )
    }

    /// The occurrence starting at local `start`.
    fn at(&self, start: DateTime) -> Option<Slot> {
        let end = start.checked_add(self.span).ok()?;
        let (start_at, end_at) = (instant(start, &self.tz).ok()?, instant(end, &self.tz).ok()?);
        let (start_text, end_text) = if self.all_day {
            (
                start.date().to_string(),
                end.date().yesterday().ok()?.to_string(),
            )
        } else {
            (format_local(start), format_local(end))
        };
        Some(Slot {
            recurrence_id: start_text.clone(),
            start: start_text,
            end: end_text,
            start_at,
            end_at,
        })
    }

    /// Local starts of the series; just the start for single events.
    fn starts(&self) -> Box<dyn Iterator<Item = DateTime> + '_> {
        match &self.rule {
            Some(rule) => Box::new(rule.starts(self.start, &self.tz)),
            None => Box::new(std::iter::once(self.start)),
        }
    }

    /// Occurrences overlapping `[from, to)`, without the removed ones.
    pub(crate) fn occurrences(&self, from: Timestamp, to: Timestamp) -> Vec<Slot> {
        let mut out = Vec::new();
        for start in self.starts().take(MAX_OCCURRENCES) {
            let Some(o) = self.at(start) else { break };
            if o.start_at >= to {
                break;
            }
            let overlaps = o.end_at > from || o.start_at == from;
            let removed = self.exdates.contains(&o.recurrence_id);
            if overlaps && !removed {
                out.push(o);
            }
        }
        out
    }

    /// End of the last occurrence for bounded series; `None` if endless.
    pub(crate) fn end_at(&self, single_end: Timestamp) -> Option<Timestamp> {
        let Some(rule) = &self.rule else {
            return Some(single_end);
        };
        if rule.count.is_none() && rule.until.is_none() {
            return None;
        }
        // An unreachable bound is treated as endless.
        let mut last = None;
        for (i, start) in self.starts().enumerate() {
            if i >= MAX_OCCURRENCES {
                return None;
            }
            last = Some(start);
        }
        last.and_then(|s| self.at(s)).map(|o| o.end_at)
    }
}

const SELECT: &str = "SELECT e.id, e.uid, e.title, e.description_md, e.location, e.all_day,
        e.start_local, e.end_local, e.tzid, e.start_at, e.end_at, e.client_id,
        c.display_name AS client_name, e.project_id, p.name AS project_name, e.task_id,
        e.contact_id, e.rrule, e.exdates, e.created_at, e.updated_at
     FROM events e
     LEFT JOIN clients c ON c.id = e.client_id
     LEFT JOIN projects p ON p.id = e.project_id";

fn from_row(r: &Row) -> rusqlite::Result<Event> {
    Ok(Event {
        id: r.get("id")?,
        uid: r.get("uid")?,
        title: r.get("title")?,
        description_md: r.get("description_md")?,
        location: r.get("location")?,
        all_day: r.get("all_day")?,
        start: r.get("start_local")?,
        end: r.get("end_local")?,
        tzid: r.get("tzid")?,
        start_at: r.get("start_at")?,
        end_at: r.get("end_at")?,
        client_id: r.get("client_id")?,
        client_name: r.get("client_name")?,
        project_id: r.get("project_id")?,
        project_name: r.get("project_name")?,
        task_id: r.get("task_id")?,
        contact_id: r.get("contact_id")?,
        rrule: r.get("rrule")?,
        exdates: serde_json::from_str(&r.get::<_, String>("exdates")?).unwrap_or_default(),
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

/// Strict `YYYY-MM-DD`; a time part (from a switch to all-day) is dropped.
fn parse_date(value: &str) -> Option<Date> {
    let date = value.trim().split('T').next()?;
    if !crate::domain::date::is_iso_date(date) {
        return None;
    }
    date.parse().ok()
}

/// Strict `YYYY-MM-DDTHH:MM`.
pub(crate) fn parse_datetime(value: &str) -> Option<DateTime> {
    let value = value.trim();
    let (date, time) = value.split_once('T')?;
    let b = time.as_bytes();
    let time_ok = b.len() == 5
        && b[2] == b':'
        && [0, 1, 3, 4].iter().all(|&i| b[i].is_ascii_digit())
        && time[..2] < *"24"
        && time[3..] < *"60";
    if !time_ok || !crate::domain::date::is_iso_date(date) {
        return None;
    }
    value.parse().ok()
}

pub(crate) fn format_local(dt: DateTime) -> String {
    dt.strftime("%Y-%m-%dT%H:%M").to_string()
}

/// The instant of a wall-clock time; times skipped by a DST change move
/// forward, repeated ones take the earlier offset (RFC 5545).
pub(crate) fn instant(dt: DateTime, tz: &TimeZone) -> CoreResult<Timestamp> {
    let zoned = tz
        .to_ambiguous_zoned(dt)
        .compatible()
        .map_err(|_| CoreError::validation("start", "datetime"))?;
    Ok(Timestamp::from_millis(zoned.timestamp().as_millisecond()))
}

fn utc_midnight(date: Date) -> CoreResult<Timestamp> {
    instant(
        date.to_datetime(jiff::civil::Time::midnight()),
        &TimeZone::UTC,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};

    struct Fixture {
        db: Db,
        clock: FakeClock,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                db: Db::open_in_memory().unwrap(),
                clock: FakeClock::at(Timestamp::from_millis(1_700_000_000_000)),
            }
        }

        fn repo(&self) -> EventRepo<'_> {
            EventRepo::new(self.db.conn(), &self.clock)
        }

        fn client(&self, name: &str) -> Id {
            ClientRepo::new(self.db.conn(), &self.clock)
                .create(ClientInput {
                    kind: ClientKind::Company,
                    last_name: None,
                    first_name: None,
                    middle_name: None,
                    company_name: Some(name.into()),
                    legal_name: None,
                    status: ClientStatus::Active,
                    default_currency: None,
                    default_rate_minor: None,
                })
                .unwrap()
                .id
        }

        fn project(&self, client: Id) -> Id {
            ProjectRepo::new(self.db.conn(), &self.clock)
                .create(ProjectInput {
                    client_id: client,
                    name: "Сайт".into(),
                    description_md: String::new(),
                    status: ProjectStatus::Active,
                    billing_type: BillingType::Fixed,
                    currency: Some("RUB".parse().unwrap()),
                    rate_minor: None,
                    budget_minor: Some(1),
                    start_date: None,
                    deadline: None,
                })
                .unwrap()
                .id
        }
    }

    pub(crate) fn input(start: &str, end: &str, tzid: &str) -> EventInput {
        EventInput {
            title: "Созвон".into(),
            description_md: String::new(),
            location: None,
            all_day: false,
            start: start.into(),
            end: end.into(),
            tzid: tzid.into(),
            client_id: None,
            project_id: None,
            task_id: None,
            contact_id: None,
            rrule: None,
        }
    }

    fn utc(s: &str) -> Timestamp {
        let ts: jiff::Timestamp = s.parse().unwrap();
        Timestamp::from_millis(ts.as_millisecond())
    }

    #[test]
    fn stores_local_time_and_utc_instants() {
        let f = Fixture::new();
        let moscow = f
            .repo()
            .create(input(
                "2026-03-10T10:00",
                "2026-03-10T11:30",
                "Europe/Moscow",
            ))
            .unwrap();
        assert_eq!(moscow.start_at, utc("2026-03-10T07:00:00Z"));
        assert_eq!(moscow.end_at, utc("2026-03-10T08:30:00Z"));
        assert_eq!(moscow.start, "2026-03-10T10:00");
        assert_eq!(f.repo().get(moscow.id).unwrap(), moscow);

        // The day New York switches to summer time.
        let ny = f
            .repo()
            .create(input(
                "2026-03-08T10:00",
                "2026-03-08T11:00",
                "America/New_York",
            ))
            .unwrap();
        assert_eq!(ny.start_at, utc("2026-03-08T14:00:00Z"));
        let winter = f
            .repo()
            .create(input(
                "2026-03-01T10:00",
                "2026-03-01T11:00",
                "America/New_York",
            ))
            .unwrap();
        assert_eq!(winter.start_at, utc("2026-03-01T15:00:00Z"));
        assert_eq!(
            (ny.start.as_str(), ny.tzid.as_str()),
            ("2026-03-08T10:00", "America/New_York")
        );
        assert!(ny.uid.ends_with("@freelance-crm"));
    }

    #[test]
    fn end_is_not_before_start() {
        let f = Fixture::new();
        assert_eq!(
            f.repo()
                .create(input("2026-03-10T10:00", "2026-03-10T09:59", "UTC"))
                .unwrap_err(),
            CoreError::validation("end", "before_start")
        );
        // Zero length is a moment, which is fine.
        assert!(
            f.repo()
                .create(input("2026-03-10T10:00", "2026-03-10T10:00", "UTC"))
                .is_ok()
        );
    }

    #[test]
    fn validates_fields() {
        let f = Fixture::new();
        let check = |patch: &dyn Fn(&mut EventInput), field: &str, rule: &str| {
            let mut i = input("2026-03-10T10:00", "2026-03-10T11:00", "UTC");
            patch(&mut i);
            assert_eq!(
                f.repo().create(i).unwrap_err(),
                CoreError::validation(field, rule)
            );
        };
        check(&|i| i.title = "  ".into(), "title", "required");
        check(
            &|i| i.start = "2026-02-30T10:00".into(),
            "start",
            "datetime",
        );
        check(&|i| i.end = "10:00".into(), "end", "datetime");
        check(&|i| i.tzid = "Mars/Olympus".into(), "tzid", "timezone");
    }

    #[test]
    fn all_day_events_are_dates() {
        let f = Fixture::new();
        let mut i = input("2026-05-01T15:00", "2026-05-03", "Europe/Moscow");
        i.all_day = true;
        let e = f.repo().create(i).unwrap();
        assert_eq!(
            (e.start.as_str(), e.end.as_str()),
            ("2026-05-01", "2026-05-03")
        );
        // Three whole days, the same everywhere.
        assert_eq!(e.start_at, utc("2026-05-01T00:00:00Z"));
        assert_eq!(e.end_at, utc("2026-05-04T00:00:00Z"));

        let mut bad = input("2026-05-03", "2026-05-01", "UTC");
        bad.all_day = true;
        assert_eq!(
            f.repo().create(bad).unwrap_err(),
            CoreError::validation("end", "before_start")
        );
    }

    #[test]
    fn lists_events_overlapping_a_range() {
        let f = Fixture::new();
        let early = f
            .repo()
            .create(input("2026-03-10T08:00", "2026-03-10T09:00", "UTC"))
            .unwrap();
        let late = f
            .repo()
            .create(input("2026-03-10T13:00", "2026-03-10T14:00", "UTC"))
            .unwrap();
        let long = f
            .repo()
            .create(input("2026-03-09T00:00", "2026-03-12T00:00", "UTC"))
            .unwrap();
        let ids = |from: &str, to: &str| -> Vec<Id> {
            f.repo()
                .list(utc(from), utc(to))
                .unwrap()
                .into_iter()
                .map(|e| e.id)
                .collect()
        };
        assert_eq!(
            ids("2026-03-10T00:00:00Z", "2026-03-11T00:00:00Z"),
            [long.id, early.id, late.id]
        );
        // Touching the end is not overlapping: ranges are half-open.
        assert_eq!(
            ids("2026-03-10T09:00:00Z", "2026-03-10T13:00:00Z"),
            [long.id]
        );
        assert!(ids("2026-03-13T00:00:00Z", "2026-03-14T00:00:00Z").is_empty());
    }

    #[test]
    fn links_to_projects_and_clients() {
        let f = Fixture::new();
        let client = f.client("Ромашка");
        let project = f.project(client);
        let mut i = input("2026-03-10T10:00", "2026-03-10T11:00", "UTC");
        i.project_id = Some(project);
        let e = f.repo().create(i.clone()).unwrap();
        assert_eq!(e.client_id, Some(client));
        assert_eq!(e.client_name.as_deref(), Some("Ромашка"));
        assert_eq!(e.project_name.as_deref(), Some("Сайт"));

        i.client_id = Some(f.client("Другой"));
        assert_eq!(
            f.repo().create(i).unwrap_err(),
            CoreError::new("event.client_mismatch")
        );
        let missing = Id::new();
        let mut i = input("2026-03-10T10:00", "2026-03-10T11:00", "UTC");
        i.task_id = Some(missing);
        assert_eq!(
            f.repo().create(i).unwrap_err(),
            CoreError::not_found("task", missing)
        );
    }

    #[test]
    fn updates_and_deletes() {
        let f = Fixture::new();
        let e = f
            .repo()
            .create(input("2026-03-10T10:00", "2026-03-10T11:00", "UTC"))
            .unwrap();
        f.clock.advance_millis(1000);
        let mut i = input("2026-03-11T10:00", "2026-03-11T12:00", "Asia/Kuala_Lumpur");
        i.title = "Встреча".into();
        let updated = f.repo().update(e.id, i).unwrap();
        assert_eq!(updated.title, "Встреча");
        assert_eq!(updated.start_at, utc("2026-03-11T02:00:00Z"));
        assert_eq!(updated.uid, e.uid, "the UID is stable");
        assert_eq!(updated.created_at, e.created_at);
        assert!(updated.updated_at > e.updated_at);
        f.repo().delete(e.id).unwrap();
        assert_eq!(
            f.repo().get(e.id).unwrap_err(),
            CoreError::not_found("event", e.id)
        );
        assert_eq!(f.repo().delete(e.id).unwrap_err().code, "event.not_found");
    }

    fn starts(occurrences: &[Occurrence]) -> Vec<(&str, Timestamp)> {
        occurrences
            .iter()
            .map(|o| (o.start.as_str(), o.start_at))
            .collect()
    }

    #[test]
    fn recurring_events_are_validated_and_normalized() {
        let f = Fixture::new();
        let mut i = input("2026-03-02T10:00", "2026-03-02T11:00", "UTC");
        i.rrule = Some("rrule:freq=weekly;interval=1;byday=mo".into());
        assert_eq!(
            f.repo().create(i.clone()).unwrap().rrule.as_deref(),
            Some("FREQ=WEEKLY;BYDAY=MO")
        );
        i.rrule = Some("FREQ=FORTNIGHTLY".into());
        assert_eq!(
            f.repo().create(i.clone()).unwrap_err().code,
            "rrule.invalid"
        );
        i.rrule = Some("   ".into());
        assert_eq!(f.repo().create(i).unwrap().rrule, None);
    }

    #[test]
    fn weekly_events_keep_local_time_across_dst() {
        let f = Fixture::new();
        // Mondays 10:00 in Berlin; summer time starts on 2026-03-29.
        let mut i = input("2026-03-23T10:00", "2026-03-23T11:00", "Europe/Berlin");
        i.rrule = Some("FREQ=WEEKLY".into());
        let e = f.repo().create(i).unwrap();
        let all = f
            .repo()
            .occurrences(utc("2026-03-01T00:00:00Z"), utc("2026-04-07T00:00:00Z"))
            .unwrap();
        assert_eq!(
            starts(&all),
            [
                ("2026-03-23T10:00", utc("2026-03-23T09:00:00Z")),
                ("2026-03-30T10:00", utc("2026-03-30T08:00:00Z")),
                ("2026-04-06T10:00", utc("2026-04-06T08:00:00Z")),
            ]
        );
        assert_eq!(all[1].end, "2026-03-30T11:00");
        assert_eq!(all[1].end_at, utc("2026-03-30T09:00:00Z"));
        assert_eq!(all[1].recurrence_id.as_deref(), Some("2026-03-30T10:00"));
        assert_eq!(all[1].event.id, e.id);
    }

    #[test]
    fn ranges_cut_series_at_their_boundaries() {
        let f = Fixture::new();
        let mut i = input("2026-03-01T09:00", "2026-03-01T10:00", "UTC");
        i.rrule = Some("FREQ=DAILY".into());
        f.repo().create(i).unwrap();
        let single = f
            .repo()
            .create(input("2026-03-05T12:00", "2026-03-05T13:00", "UTC"))
            .unwrap();
        // The occurrence ending exactly at `from` is out; the one starting
        // exactly at `to` is out too.
        let got = f
            .repo()
            .occurrences(utc("2026-03-04T10:00:00Z"), utc("2026-03-06T09:00:00Z"))
            .unwrap();
        assert_eq!(
            got.iter().map(|o| o.start.as_str()).collect::<Vec<_>>(),
            ["2026-03-05T09:00", "2026-03-05T12:00"]
        );
        assert_eq!(got[1].event.id, single.id);
        assert_eq!(got[1].recurrence_id, None);
        // Before the series starts there is nothing.
        assert!(
            f.repo()
                .occurrences(utc("2026-02-01T00:00:00Z"), utc("2026-03-01T09:00:00Z"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn count_until_and_exdates_limit_the_series() {
        let f = Fixture::new();
        let mut i = input("2026-03-01T09:00", "2026-03-01T10:00", "UTC");
        i.rrule = Some("FREQ=DAILY;COUNT=3".into());
        let e = f.repo().create(i).unwrap();
        f.db.conn()
            .execute(
                "UPDATE events SET exdates = '[\"2026-03-02T09:00\"]' WHERE id = ?1",
                [e.id],
            )
            .unwrap();
        let got = f
            .repo()
            .occurrences(utc("2026-01-01T00:00:00Z"), utc("2027-01-01T00:00:00Z"))
            .unwrap();
        // COUNT counts the removed occurrence too (RFC 5545).
        assert_eq!(
            got.iter().map(|o| o.start.as_str()).collect::<Vec<_>>(),
            ["2026-03-01T09:00", "2026-03-03T09:00"]
        );
        assert_eq!(f.repo().get(e.id).unwrap().exdates, ["2026-03-02T09:00"]);
    }

    #[test]
    fn all_day_series_repeat_whole_days() {
        let f = Fixture::new();
        let mut i = input("2026-12-31", "2027-01-01", "Europe/Moscow");
        i.all_day = true;
        i.rrule = Some("FREQ=YEARLY".into());
        f.repo().create(i).unwrap();
        let got = f
            .repo()
            .occurrences(utc("2027-12-01T00:00:00Z"), utc("2028-01-01T00:00:00Z"))
            .unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(
            (got[0].start.as_str(), got[0].end.as_str()),
            ("2027-12-31", "2028-01-01")
        );
        assert_eq!(got[0].start_at, utc("2027-12-31T00:00:00Z"));
        assert_eq!(got[0].end_at, utc("2028-01-02T00:00:00Z"));
    }

    #[test]
    fn deleting_the_client_deletes_its_events_but_tasks_only_unlink() {
        use crate::tasks::{Priority, TaskInput, TaskRepo};
        let f = Fixture::new();
        let client = f.client("Ромашка");
        let mut i = input("2026-03-10T10:00", "2026-03-10T11:00", "UTC");
        i.client_id = Some(client);
        let of_client = f.repo().create(i).unwrap();
        let task = TaskRepo::new(f.db.conn(), &f.clock)
            .create(TaskInput {
                title: "Макет".into(),
                description_md: String::new(),
                status_id: None,
                priority: Priority::Normal,
                project_id: None,
                client_id: None,
                due_date: None,
                due_time: None,
                estimate_minutes: None,
            })
            .unwrap();
        let mut i = input("2026-03-10T10:00", "2026-03-10T11:00", "UTC");
        i.task_id = Some(task.id);
        let of_task = f.repo().create(i).unwrap();

        assert_eq!(
            ClientRepo::new(f.db.conn(), &f.clock)
                .delete_impact(client)
                .unwrap()
                .events,
            1
        );
        ClientRepo::new(f.db.conn(), &f.clock)
            .delete(client)
            .unwrap();
        assert_eq!(
            f.repo().get(of_client.id).unwrap_err().code,
            "event.not_found"
        );
        TaskRepo::new(f.db.conn(), &f.clock)
            .delete(task.id)
            .unwrap();
        assert_eq!(f.repo().get(of_task.id).unwrap().task_id, None);
    }
}
