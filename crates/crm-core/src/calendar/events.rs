//! Calendar events (FR-CA-2). Times are entered as local wall-clock time in
//! an IANA time zone; both the local values and the UTC instants are
//! stored, so ranges are queried in UTC and events are shown and repeated
//! in their own zone. All-day events are dates: the end date is inclusive
//! and the UTC range covers the days from midnight UTC (floating).

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use super::rrule::{Rrule, Until};
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
    /// For an edited occurrence: its series and original local start.
    pub series_id: Option<Id>,
    pub recurrence_id: Option<String>,
    /// iCalendar UID, kept across export and import.
    pub uid: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Which occurrences of a series an edit or deletion applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum EditScope {
    /// Only this occurrence.
    This,
    /// This occurrence and all later ones.
    Following,
    /// The whole series.
    All,
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
        self.insert(&r, None)
    }

    /// Inserts a resolved event, optionally as the override of occurrence
    /// `recurrence_id` of a series.
    fn insert(&self, r: &Resolved, of_series: Option<(Id, &str)>) -> CoreResult<Event> {
        let id = Id::new();
        let now = self.clock.now();
        self.conn.execute(
            "INSERT INTO events (id, uid, title, description_md, location, all_day, start_local,
                 end_local, tzid, start_at, end_at, client_id, project_id, task_id, contact_id,
                 created_at, updated_at, rrule, series_end_at, series_id, recurrence_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16,
                 ?17, ?18, ?19, ?20)",
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
                of_series.map(|(series, _)| series),
                of_series.map(|(_, rid)| rid),
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
                    // Overrides tell which occurrence of their series they are.
                    recurrence_id: event.recurrence_id.clone(),
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

    /// Edits occurrence `recurrence_id` of series `id`. Returns the event
    /// holding the result: the override, the new series or the series.
    pub fn update_occurrence(
        &self,
        id: Id,
        recurrence_id: &str,
        scope: EditScope,
        input: EventInput,
    ) -> CoreResult<Event> {
        let (series, event, position) = self.occurrence_of(id, recurrence_id)?;
        match scope {
            EditScope::This => {
                let r = self.resolve(EventInput {
                    rrule: None,
                    ..input
                })?;
                if let Some(over) = self.override_of(id, recurrence_id)? {
                    return self.update(over, r.input);
                }
                let mut exdates = event.exdates.clone();
                exdates.push(recurrence_id.to_owned());
                exdates.sort();
                self.set_rule(&event, series.rule.as_ref(), &exdates)?;
                self.insert(&r, Some((id, recurrence_id)))
            }
            EditScope::All => self.update(id, input),
            EditScope::Following if position == 0 => self.update(id, input),
            EditScope::Following => {
                let rule = series.rule.as_ref().expect("checked recurring");
                let mut input = input;
                // An unchanged COUNT continues where the old series stops.
                let same_rule = crate::domain::text::clean(input.rrule.clone())
                    .and_then(|r| r.parse::<Rrule>().ok())
                    .is_some_and(|r| &r == rule);
                if let (true, Some(count)) = (same_rule, rule.count) {
                    let left = count.saturating_sub(position as u32).max(1);
                    input.rrule = Some(
                        Rrule {
                            count: Some(left),
                            ..rule.clone()
                        }
                        .to_string(),
                    );
                }
                let r = self.resolve(input)?;
                self.truncate(&event, &series, recurrence_id)?;
                self.insert(&r, None)
            }
        }
    }

    /// Deletes occurrence `recurrence_id` of series `id` within `scope`.
    pub fn delete_occurrence(
        &self,
        id: Id,
        recurrence_id: &str,
        scope: EditScope,
    ) -> CoreResult<()> {
        let (series, event, position) = self.occurrence_of(id, recurrence_id)?;
        match scope {
            EditScope::This => {
                if let Some(over) = self.override_of(id, recurrence_id)? {
                    self.delete(over)?;
                }
                let mut exdates = event.exdates.clone();
                if !exdates.iter().any(|x| x == recurrence_id) {
                    exdates.push(recurrence_id.to_owned());
                    exdates.sort();
                }
                self.set_rule(&event, series.rule.as_ref(), &exdates)
            }
            EditScope::Following if position > 0 => self.truncate(&event, &series, recurrence_id),
            EditScope::Following | EditScope::All => self.delete(id),
        }
    }

    /// The series, its event and the position of occurrence
    /// `recurrence_id` in it.
    fn occurrence_of(&self, id: Id, recurrence_id: &str) -> CoreResult<(Series, Event, usize)> {
        let event = self.get(id)?;
        if event.rrule.is_none() {
            return Err(CoreError::new("event.not_recurring"));
        }
        let series =
            Series::from_event(&event).ok_or_else(|| CoreError::new("event.not_recurring"))?;
        let position = series
            .position(recurrence_id)
            .ok_or_else(|| CoreError::new("event.no_occurrence"))?;
        Ok((series, event, position))
    }

    fn override_of(&self, series: Id, recurrence_id: &str) -> CoreResult<Option<Id>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id FROM events WHERE series_id = ?1 AND recurrence_id = ?2",
                (series, recurrence_id),
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Ends the series just before occurrence `recurrence_id`, dropping
    /// the exceptions and overrides of the cut-off part.
    fn truncate(&self, event: &Event, series: &Series, recurrence_id: &str) -> CoreResult<()> {
        let rule = series.rule.as_ref().expect("checked recurring");
        let until = series
            .until_before(recurrence_id)
            .ok_or_else(|| CoreError::new("event.no_occurrence"))?;
        let rule = Rrule {
            count: None,
            until: Some(until),
            ..rule.clone()
        };
        let exdates: Vec<String> = event
            .exdates
            .iter()
            .filter(|x| x.as_str() < recurrence_id)
            .cloned()
            .collect();
        self.conn.execute(
            "DELETE FROM events WHERE series_id = ?1 AND recurrence_id >= ?2",
            (event.id, recurrence_id),
        )?;
        self.set_rule(event, Some(&rule), &exdates)
    }

    /// Stores a new rule and exceptions for `event`, with the series end.
    fn set_rule(&self, event: &Event, rule: Option<&Rrule>, exdates: &[String]) -> CoreResult<()> {
        let series = Series::new(
            event.all_day,
            &event.start,
            &event.end,
            &event.tzid,
            rule.cloned(),
            exdates.to_vec(),
        )
        .ok_or_else(|| CoreError::new("event.not_recurring"))?;
        self.conn.execute(
            "UPDATE events SET rrule = ?2, exdates = ?3, series_end_at = ?4, updated_at = ?5
             WHERE id = ?1",
            rusqlite::params![
                event.id,
                rule.map(|r| r.to_string()),
                serde_json::to_string(exdates).expect("strings serialize"),
                series.end_at(event.end_at),
                self.clock.now(),
            ],
        )?;
        Ok(())
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

    /// Text of a local start, as stored in RECURRENCE-ID and EXDATE.
    fn text(&self, start: DateTime) -> String {
        if self.all_day {
            start.date().to_string()
        } else {
            format_local(start)
        }
    }

    /// Index of occurrence `recurrence_id` among all starts (removed ones
    /// included), if it is one.
    pub(crate) fn position(&self, recurrence_id: &str) -> Option<usize> {
        for (i, start) in self.starts().take(MAX_OCCURRENCES).enumerate() {
            match self.text(start).as_str().cmp(recurrence_id) {
                std::cmp::Ordering::Equal => return Some(i),
                std::cmp::Ordering::Greater => return None,
                std::cmp::Ordering::Less => {}
            }
        }
        None
    }

    /// An UNTIL ending the series right before occurrence `recurrence_id`.
    fn until_before(&self, recurrence_id: &str) -> Option<Until> {
        if self.all_day {
            return parse_date(recurrence_id)?.yesterday().ok().map(Until::Date);
        }
        let at = instant(parse_datetime(recurrence_id)?, &self.tz).ok()?;
        jiff::Timestamp::from_millisecond(at.as_millis() - 1000)
            .ok()
            .map(Until::Instant)
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
        e.contact_id, e.rrule, e.exdates, e.series_id, e.recurrence_id, e.created_at,
        e.updated_at
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
        series_id: r.get("series_id")?,
        recurrence_id: r.get("recurrence_id")?,
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

    /// Weekly on Tuesdays 10:00–11:00 UTC from 2026-03-03, optionally bounded.
    fn weekly(f: &Fixture, rule: &str) -> Event {
        let mut i = input("2026-03-03T10:00", "2026-03-03T11:00", "UTC");
        i.title = "Планёрка".into();
        i.rrule = Some(rule.into());
        f.repo().create(i).unwrap()
    }

    fn march(f: &Fixture) -> Vec<(String, String)> {
        f.repo()
            .occurrences(utc("2026-03-01T00:00:00Z"), utc("2026-04-01T00:00:00Z"))
            .unwrap()
            .into_iter()
            .map(|o| (o.start, o.event.title))
            .collect()
    }

    fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    #[test]
    fn editing_only_this_occurrence_creates_an_override() {
        let f = Fixture::new();
        let series = weekly(&f, "FREQ=WEEKLY");
        let mut moved = input("2026-03-11T15:00", "2026-03-11T16:00", "UTC");
        moved.title = "Перенесли".into();
        let over = f
            .repo()
            .update_occurrence(
                series.id,
                "2026-03-10T10:00",
                EditScope::This,
                moved.clone(),
            )
            .unwrap();
        assert_eq!(over.series_id, Some(series.id));
        assert_eq!(over.recurrence_id.as_deref(), Some("2026-03-10T10:00"));
        assert_eq!(over.rrule, None);
        assert_eq!(
            f.repo().get(series.id).unwrap().exdates,
            ["2026-03-10T10:00"]
        );
        assert_eq!(
            march(&f),
            pairs(&[
                ("2026-03-03T10:00", "Планёрка"),
                ("2026-03-11T15:00", "Перенесли"),
                ("2026-03-17T10:00", "Планёрка"),
                ("2026-03-24T10:00", "Планёрка"),
                ("2026-03-31T10:00", "Планёрка"),
            ])
        );
        // Its occurrence still tells which one of the series it replaces.
        let occ = f
            .repo()
            .occurrences(utc("2026-03-11T00:00:00Z"), utc("2026-03-12T00:00:00Z"))
            .unwrap();
        assert_eq!(occ[0].recurrence_id.as_deref(), Some("2026-03-10T10:00"));

        // Editing it again updates the same override.
        moved.title = "Ещё раз".into();
        let again = f
            .repo()
            .update_occurrence(series.id, "2026-03-10T10:00", EditScope::This, moved)
            .unwrap();
        assert_eq!(again.id, over.id);
        assert_eq!(
            f.repo().get(series.id).unwrap().exdates,
            ["2026-03-10T10:00"]
        );
    }

    #[test]
    fn editing_this_and_following_splits_the_series() {
        let f = Fixture::new();
        let series = weekly(&f, "FREQ=WEEKLY;COUNT=5");
        // An override after the split point is dropped with the old tail.
        let mut early = input("2026-03-24T12:00", "2026-03-24T13:00", "UTC");
        early.title = "Старое исключение".into();
        f.repo()
            .update_occurrence(series.id, "2026-03-24T10:00", EditScope::This, early)
            .unwrap();

        let mut later = input("2026-03-17T14:00", "2026-03-17T15:00", "UTC");
        later.title = "После обеда".into();
        later.rrule = Some("FREQ=WEEKLY;COUNT=5".into());
        let tail = f
            .repo()
            .update_occurrence(series.id, "2026-03-17T10:00", EditScope::Following, later)
            .unwrap();
        let head = f.repo().get(series.id).unwrap();
        assert_eq!(
            head.rrule.as_deref(),
            Some("FREQ=WEEKLY;UNTIL=20260317T095959Z")
        );
        assert!(head.exdates.is_empty());
        assert_ne!(tail.id, series.id);
        assert_ne!(tail.uid, series.uid);
        // The unchanged COUNT continues: 2 before the split, 3 after.
        assert_eq!(tail.rrule.as_deref(), Some("FREQ=WEEKLY;COUNT=3"));
        assert_eq!(
            march(&f),
            pairs(&[
                ("2026-03-03T10:00", "Планёрка"),
                ("2026-03-10T10:00", "Планёрка"),
                ("2026-03-17T14:00", "После обеда"),
                ("2026-03-24T14:00", "После обеда"),
                ("2026-03-31T14:00", "После обеда"),
            ])
        );
    }

    #[test]
    fn following_from_the_first_occurrence_edits_everything() {
        let f = Fixture::new();
        let series = weekly(&f, "FREQ=WEEKLY");
        let mut i = input("2026-03-03T10:00", "2026-03-03T11:00", "UTC");
        i.title = "Новое имя".into();
        i.rrule = Some("FREQ=WEEKLY".into());
        let e = f
            .repo()
            .update_occurrence(series.id, "2026-03-03T10:00", EditScope::Following, i)
            .unwrap();
        assert_eq!(e.id, series.id);
        assert_eq!(e.title, "Новое имя");
    }

    #[test]
    fn editing_all_keeps_exceptions() {
        let f = Fixture::new();
        let series = weekly(&f, "FREQ=WEEKLY");
        f.repo()
            .delete_occurrence(series.id, "2026-03-10T10:00", EditScope::This)
            .unwrap();
        let mut moved = input("2026-03-18T10:00", "2026-03-18T11:00", "UTC");
        moved.title = "Исключение".into();
        f.repo()
            .update_occurrence(series.id, "2026-03-17T10:00", EditScope::This, moved)
            .unwrap();
        let mut i = input("2026-03-03T10:00", "2026-03-03T11:00", "UTC");
        i.title = "Все".into();
        i.rrule = Some("FREQ=WEEKLY".into());
        f.repo()
            .update_occurrence(series.id, "2026-03-24T10:00", EditScope::All, i)
            .unwrap();
        assert_eq!(
            march(&f),
            pairs(&[
                ("2026-03-03T10:00", "Все"),
                ("2026-03-18T10:00", "Исключение"),
                ("2026-03-24T10:00", "Все"),
                ("2026-03-31T10:00", "Все"),
            ])
        );
    }

    #[test]
    fn deleting_occurrences() {
        let f = Fixture::new();
        let series = weekly(&f, "FREQ=WEEKLY");
        let mut moved = input("2026-03-10T12:00", "2026-03-10T13:00", "UTC");
        moved.title = "Исключение".into();
        f.repo()
            .update_occurrence(series.id, "2026-03-10T10:00", EditScope::This, moved)
            .unwrap();
        // Deleting an edited occurrence removes its override too.
        f.repo()
            .delete_occurrence(series.id, "2026-03-10T10:00", EditScope::This)
            .unwrap();
        f.repo()
            .delete_occurrence(series.id, "2026-03-17T10:00", EditScope::This)
            .unwrap();
        assert_eq!(
            f.repo().get(series.id).unwrap().exdates,
            ["2026-03-10T10:00", "2026-03-17T10:00"]
        );
        f.repo()
            .delete_occurrence(series.id, "2026-03-31T10:00", EditScope::Following)
            .unwrap();
        assert_eq!(
            march(&f),
            pairs(&[
                ("2026-03-03T10:00", "Планёрка"),
                ("2026-03-24T10:00", "Планёрка")
            ])
        );
        f.repo()
            .delete_occurrence(series.id, "2026-03-03T10:00", EditScope::Following)
            .unwrap();
        assert_eq!(f.repo().get(series.id).unwrap_err().code, "event.not_found");
        let overrides: i64 =
            f.db.conn()
                .query_row("SELECT count(*) FROM events", [], |r| r.get(0))
                .unwrap();
        assert_eq!(overrides, 0);
    }

    #[test]
    fn rejects_unknown_occurrences_and_single_events() {
        let f = Fixture::new();
        let series = weekly(&f, "FREQ=WEEKLY");
        assert_eq!(
            f.repo()
                .delete_occurrence(series.id, "2026-03-11T10:00", EditScope::This)
                .unwrap_err(),
            CoreError::new("event.no_occurrence")
        );
        let single = f
            .repo()
            .create(input("2026-03-03T10:00", "2026-03-03T11:00", "UTC"))
            .unwrap();
        assert_eq!(
            f.repo()
                .delete_occurrence(single.id, "2026-03-03T10:00", EditScope::This)
                .unwrap_err(),
            CoreError::new("event.not_recurring")
        );
    }

    #[test]
    fn all_day_series_split_by_date() {
        let f = Fixture::new();
        let mut i = input("2026-03-02", "2026-03-02", "UTC");
        i.all_day = true;
        i.rrule = Some("FREQ=DAILY".into());
        let series = f.repo().create(i.clone()).unwrap();
        i.start = "2026-03-05".into();
        i.end = "2026-03-05".into();
        f.repo()
            .update_occurrence(series.id, "2026-03-05", EditScope::Following, i)
            .unwrap();
        assert_eq!(
            f.repo().get(series.id).unwrap().rrule.as_deref(),
            Some("FREQ=DAILY;UNTIL=20260304")
        );
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
