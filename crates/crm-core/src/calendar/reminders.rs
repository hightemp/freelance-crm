//! Reminders (FR-CA-5, 6; PRD §6.5): "N minutes before" an event
//! occurrence or a task deadline. All-day items count from 09:00 local
//! time on their day. An edited occurrence of a series uses the series'
//! reminders.
//!
//! [`due_reminders`] decides what fires in a check window `(last_check,
//! now]`; anything due more than [`MISSED_AFTER_MS`] ago was missed while
//! the app was closed.

use jiff::tz::TimeZone;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::events::EventRepo;
use crate::domain::{Clock, EntityRef, Id, Timestamp};
use crate::{CoreError, CoreResult};

/// Older than this, a due reminder counts as missed.
pub const MISSED_AFTER_MS: i64 = 5 * 60_000;
/// How far back missed reminders are looked for.
pub const LOOKBACK_MS: i64 = 7 * 24 * 3_600_000;
/// The longest reminder offset: four weeks.
pub const MAX_MINUTES_BEFORE: u32 = 4 * 7 * 24 * 60;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: Id,
    pub minutes_before: u32,
}

/// A reminder due at `fire_at` for one occurrence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub reminder_id: Id,
    /// The event (series) or task.
    pub owner: EntityRef,
    /// RECURRENCE-ID of the occurrence; empty for single items.
    pub occurrence: String,
    pub title: String,
    pub fire_at: Timestamp,
    /// When the event or deadline itself starts.
    pub start_at: Timestamp,
    pub all_day: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Due {
    pub candidate: Candidate,
    /// Due while the app was not running.
    pub missed: bool,
}

/// The reminders to fire at `now`: due in `(last_check, now]` (on the
/// first check, the last few minutes) and not yet handled.
pub fn due_reminders(
    now: Timestamp,
    last_check: Option<Timestamp>,
    candidates: Vec<Candidate>,
    handled: impl Fn(&Candidate) -> bool,
) -> Vec<Due> {
    let (start, _) = window(now, last_check);
    let mut due: Vec<Due> = candidates
        .into_iter()
        .filter(|c| c.fire_at > start && c.fire_at <= now && !handled(c))
        .map(|c| Due {
            missed: now.as_millis() - c.fire_at.as_millis() > MISSED_AFTER_MS,
            candidate: c,
        })
        .collect();
    due.sort_by_key(|d| d.candidate.fire_at);
    due
}

/// The window [`ReminderRepo::candidates`] must cover for a check.
pub fn window(now: Timestamp, last_check: Option<Timestamp>) -> (Timestamp, Timestamp) {
    let start = match last_check {
        None => now.as_millis() - MISSED_AFTER_MS,
        Some(last) => last.as_millis().max(now.as_millis() - LOOKBACK_MS),
    };
    (Timestamp::from_millis(start), now)
}

/// When an all-day item's reminders count from: 09:00 local on its day.
fn nine_am(date: &str, tz: &TimeZone) -> Option<Timestamp> {
    let date: jiff::civil::Date = date.get(..10)?.parse().ok()?;
    super::events::instant(date.to_datetime(jiff::civil::time(9, 0, 0, 0)), tz).ok()
}

pub struct ReminderRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> ReminderRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    /// Replaces the reminders of an event or task.
    pub fn set(&self, owner: EntityRef, minutes_before: &[u32]) -> CoreResult<Vec<Reminder>> {
        if !matches!(owner, EntityRef::Event(_) | EntityRef::Task(_)) {
            return Err(CoreError::new("reminder.unsupported_owner"));
        }
        owner.ensure_exists(self.conn)?;
        if minutes_before.iter().any(|&m| m > MAX_MINUTES_BEFORE) {
            return Err(CoreError::validation("minutesBefore", "range"));
        }
        let mut minutes = minutes_before.to_vec();
        minutes.sort_unstable();
        minutes.dedup();
        let (owner_type, owner_id) = owner.parts();
        self.conn.execute(
            "DELETE FROM reminders WHERE owner_type = ?1 AND owner_id = ?2",
            (owner_type, owner_id),
        )?;
        let now = self.clock.now();
        for m in minutes {
            self.conn.execute(
                "INSERT INTO reminders (id, owner_type, owner_id, minutes_before, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                (Id::new(), owner_type, owner_id, m, now),
            )?;
        }
        self.list(owner)
    }

    /// By offset, nearest to the start first.
    pub fn list(&self, owner: EntityRef) -> CoreResult<Vec<Reminder>> {
        let (owner_type, owner_id) = owner.parts();
        let mut stmt = self.conn.prepare(
            "SELECT id, minutes_before FROM reminders WHERE owner_type = ?1 AND owner_id = ?2
             ORDER BY minutes_before",
        )?;
        let rows = stmt.query_map((owner_type, owner_id), |r| {
            Ok(Reminder {
                id: r.get(0)?,
                minutes_before: r.get(1)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Offsets of all reminders of one type, by owner.
    fn offsets(
        &self,
        owner_type: &str,
    ) -> CoreResult<std::collections::HashMap<Id, Vec<Reminder>>> {
        let mut stmt = self.conn.prepare(
            "SELECT owner_id, id, minutes_before FROM reminders WHERE owner_type = ?1
             ORDER BY minutes_before",
        )?;
        let mut map: std::collections::HashMap<Id, Vec<Reminder>> = Default::default();
        let rows = stmt.query_map([owner_type], |r| {
            Ok((
                r.get::<_, Id>(0)?,
                Reminder {
                    id: r.get(1)?,
                    minutes_before: r.get(2)?,
                },
            ))
        })?;
        for row in rows {
            let (owner, reminder) = row?;
            map.entry(owner).or_default().push(reminder);
        }
        Ok(map)
    }

    /// Reminders firing in `(from, to]`; `tz` places task deadlines and
    /// all-day items.
    pub fn candidates(
        &self,
        from: Timestamp,
        to: Timestamp,
        tz: &TimeZone,
    ) -> CoreResult<Vec<Candidate>> {
        let mut out = Vec::new();
        let in_window = |at: Timestamp| at > from && at <= to;
        let fire = |base: Timestamp, minutes: u32| {
            Timestamp::from_millis(base.as_millis() - i64::from(minutes) * 60_000)
        };

        let events = self.offsets("event")?;
        if let Some(longest) = events.values().flatten().map(|r| r.minutes_before).max() {
            // Occurrences starting up to the longest offset after the window
            // (plus the 09:00 shift of all-day ones) can fire inside it.
            let ahead = (i64::from(longest) * 60_000) + 24 * 3_600_000 + 1;
            let until = Timestamp::from_millis(to.as_millis() + ahead);
            for o in EventRepo::new(self.conn, self.clock).occurrences(from, until)? {
                let series = o.event.series_id.unwrap_or(o.event.id);
                let Some(reminders) = events.get(&series) else {
                    continue;
                };
                let base = if o.event.all_day {
                    nine_am(&o.start, tz)
                } else {
                    Some(o.start_at)
                };
                let Some(base) = base else { continue };
                for r in reminders {
                    let fire_at = fire(base, r.minutes_before);
                    if in_window(fire_at) {
                        out.push(Candidate {
                            reminder_id: r.id,
                            owner: EntityRef::Event(series),
                            occurrence: o.recurrence_id.clone().unwrap_or_default(),
                            title: o.event.title.clone(),
                            fire_at,
                            start_at: o.start_at,
                            all_day: o.event.all_day,
                        });
                    }
                }
            }
        }

        let tasks = self.offsets("task")?;
        if !tasks.is_empty() {
            let mut stmt = self.conn.prepare(
                "SELECT t.id, t.title, t.due_date, t.due_time FROM tasks t
                 WHERE t.due_date IS NOT NULL AND t.completed_at IS NULL
                   AND EXISTS (SELECT 1 FROM reminders r
                               WHERE r.owner_type = 'task' AND r.owner_id = t.id)",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, Id>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            })?;
            for row in rows {
                let (id, title, date, time) = row?;
                let base = match &time {
                    Some(time) => super::events::parse_datetime(&format!("{date}T{time}"))
                        .and_then(|local| super::events::instant(local, tz).ok()),
                    None => nine_am(&date, tz),
                };
                let Some(base) = base else { continue };
                for r in tasks.get(&id).into_iter().flatten() {
                    let fire_at = fire(base, r.minutes_before);
                    if in_window(fire_at) {
                        out.push(Candidate {
                            reminder_id: r.id,
                            owner: EntityRef::Task(id),
                            occurrence: String::new(),
                            title: title.clone(),
                            fire_at,
                            start_at: base,
                            all_day: time.is_none(),
                        });
                    }
                }
            }
        }
        out.sort_by(|a, b| (a.fire_at, &a.title).cmp(&(b.fire_at, &b.title)));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::events::{EditScope, EventInput};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::tasks::{Priority, TaskInput, TaskRepo};

    fn utc(s: &str) -> Timestamp {
        let ts: jiff::Timestamp = s.parse().unwrap();
        Timestamp::from_millis(ts.as_millisecond())
    }

    fn candidate(fire_at: &str) -> Candidate {
        Candidate {
            reminder_id: Id::new(),
            owner: EntityRef::Event(Id::new()),
            occurrence: String::new(),
            title: "Созвон".into(),
            fire_at: utc(fire_at),
            start_at: utc(fire_at),
            all_day: false,
        }
    }

    fn fired(due: &[Due]) -> Vec<(Timestamp, bool)> {
        due.iter()
            .map(|d| (d.candidate.fire_at, d.missed))
            .collect()
    }

    #[test]
    fn fires_exactly_within_the_window() {
        let now = utc("2026-03-10T10:00:00Z");
        let last = Some(utc("2026-03-10T09:59:00Z"));
        let cs = vec![
            candidate("2026-03-10T09:59:00Z"), // fired by the previous check
            candidate("2026-03-10T09:59:30Z"),
            candidate("2026-03-10T10:00:00Z"),
            candidate("2026-03-10T10:00:01Z"), // not yet
        ];
        assert_eq!(
            fired(&due_reminders(now, last, cs, |_| false)),
            [
                (utc("2026-03-10T09:59:30Z"), false),
                (utc("2026-03-10T10:00:00Z"), false)
            ]
        );
    }

    #[test]
    fn handled_reminders_do_not_fire_again() {
        let now = utc("2026-03-10T10:00:00Z");
        let c = candidate("2026-03-10T09:59:30Z");
        let id = c.reminder_id;
        assert!(
            due_reminders(now, Some(utc("2026-03-10T09:00:00Z")), vec![c], |c| c
                .reminder_id
                == id)
            .is_empty()
        );
    }

    #[test]
    fn reminders_due_while_closed_are_missed() {
        // Closed at 08:00, started at 10:00.
        let now = utc("2026-03-10T10:00:00Z");
        let cs = vec![
            candidate("2026-03-10T09:00:00Z"),
            candidate("2026-03-10T09:57:00Z"),
        ];
        assert_eq!(
            fired(&due_reminders(
                now,
                Some(utc("2026-03-10T08:00:00Z")),
                cs,
                |_| false
            )),
            [
                (utc("2026-03-10T09:00:00Z"), true),
                (utc("2026-03-10T09:57:00Z"), false)
            ]
        );
    }

    #[test]
    fn the_first_check_and_long_breaks_look_back_only_so_far() {
        let now = utc("2026-03-10T10:00:00Z");
        assert_eq!(window(now, None), (utc("2026-03-10T09:55:00Z"), now));
        assert_eq!(
            window(now, Some(utc("2026-01-01T00:00:00Z"))),
            (utc("2026-03-03T10:00:00Z"), now)
        );
        let cs = vec![
            candidate("2026-03-10T09:00:00Z"),
            candidate("2026-03-10T09:58:00Z"),
        ];
        assert_eq!(
            fired(&due_reminders(now, None, cs, |_| false)),
            [(utc("2026-03-10T09:58:00Z"), false)]
        );
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

        fn repo(&self) -> ReminderRepo<'_> {
            ReminderRepo::new(self.db.conn(), &self.clock)
        }

        fn events(&self) -> EventRepo<'_> {
            EventRepo::new(self.db.conn(), &self.clock)
        }

        fn event(&self, start: &str, end: &str, tzid: &str, rrule: Option<&str>) -> Id {
            self.events()
                .create(EventInput {
                    title: "Планёрка".into(),
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
                    rrule: rrule.map(Into::into),
                })
                .unwrap()
                .id
        }

        fn fire_times(&self, from: &str, to: &str, tz: &TimeZone) -> Vec<(String, Timestamp)> {
            self.repo()
                .candidates(utc(from), utc(to), tz)
                .unwrap()
                .into_iter()
                .map(|c| (c.occurrence, c.fire_at))
                .collect()
        }
    }

    #[test]
    fn sets_and_lists_reminders() {
        let f = Fixture::new();
        let owner = EntityRef::Event(f.event("2026-03-10T10:00", "2026-03-10T11:00", "UTC", None));
        let set = f.repo().set(owner, &[60, 15, 60]).unwrap();
        assert_eq!(
            set.iter().map(|r| r.minutes_before).collect::<Vec<_>>(),
            [15, 60]
        );
        assert_eq!(f.repo().list(owner).unwrap(), set);
        assert_eq!(
            f.repo().set(owner, &[MAX_MINUTES_BEFORE + 1]).unwrap_err(),
            CoreError::validation("minutesBefore", "range")
        );
        assert_eq!(
            f.repo()
                .set(EntityRef::Client(Id::new()), &[5])
                .unwrap_err()
                .code,
            "reminder.unsupported_owner"
        );
        f.repo().set(owner, &[]).unwrap();
        assert!(f.repo().list(owner).unwrap().is_empty());
    }

    #[test]
    fn recurring_events_remind_for_every_occurrence_across_dst() {
        let f = Fixture::new();
        // Mondays 10:00 in Berlin; summer time starts on 2026-03-29.
        let id = f.event(
            "2026-03-23T10:00",
            "2026-03-23T11:00",
            "Europe/Berlin",
            Some("FREQ=WEEKLY"),
        );
        f.repo().set(EntityRef::Event(id), &[15]).unwrap();
        assert_eq!(
            f.fire_times(
                "2026-03-20T00:00:00Z",
                "2026-04-07T00:00:00Z",
                &TimeZone::UTC
            ),
            [
                ("2026-03-23T10:00".to_owned(), utc("2026-03-23T08:45:00Z")),
                ("2026-03-30T10:00".to_owned(), utc("2026-03-30T07:45:00Z")),
                ("2026-04-06T10:00".to_owned(), utc("2026-04-06T07:45:00Z")),
            ]
        );
    }

    #[test]
    fn edited_occurrences_use_the_series_reminders() {
        let f = Fixture::new();
        let id = f.event(
            "2026-03-02T10:00",
            "2026-03-02T11:00",
            "UTC",
            Some("FREQ=WEEKLY;COUNT=2"),
        );
        f.repo().set(EntityRef::Event(id), &[10]).unwrap();
        let mut moved = f.events().get(id).unwrap();
        moved.start = "2026-03-09T14:00".into();
        moved.end = "2026-03-09T15:00".into();
        f.events()
            .update_occurrence(
                id,
                "2026-03-09T10:00",
                EditScope::This,
                EventInput {
                    title: moved.title,
                    description_md: moved.description_md,
                    location: None,
                    all_day: false,
                    start: moved.start,
                    end: moved.end,
                    tzid: moved.tzid,
                    client_id: None,
                    project_id: None,
                    task_id: None,
                    contact_id: None,
                    rrule: None,
                },
            )
            .unwrap();
        let cs = f
            .repo()
            .candidates(
                utc("2026-03-01T00:00:00Z"),
                utc("2026-03-31T00:00:00Z"),
                &TimeZone::UTC,
            )
            .unwrap();
        assert_eq!(
            cs.iter()
                .map(|c| (c.owner, c.occurrence.as_str(), c.fire_at))
                .collect::<Vec<_>>(),
            [
                (
                    EntityRef::Event(id),
                    "2026-03-02T10:00",
                    utc("2026-03-02T09:50:00Z")
                ),
                (
                    EntityRef::Event(id),
                    "2026-03-09T10:00",
                    utc("2026-03-09T13:50:00Z")
                ),
            ]
        );
    }

    #[test]
    fn task_deadlines_and_all_day_items_count_from_nine() {
        let f = Fixture::new();
        let tasks = TaskRepo::new(f.db.conn(), &f.clock);
        let task = |title: &str, date: &str, time: Option<&str>| {
            tasks
                .create(TaskInput {
                    title: title.into(),
                    description_md: String::new(),
                    status_id: None,
                    priority: Priority::Normal,
                    project_id: None,
                    client_id: None,
                    due_date: Some(date.into()),
                    due_time: time.map(Into::into),
                    estimate_minutes: None,
                })
                .unwrap()
                .id
        };
        let timed = task("Сдать отчёт", "2026-03-12", Some("18:00"));
        let day = task("Оплатить хостинг", "2026-03-13", None);
        f.repo().set(EntityRef::Task(timed), &[60]).unwrap();
        f.repo().set(EntityRef::Task(day), &[24 * 60]).unwrap();
        let moscow = TimeZone::get("Europe/Moscow").unwrap();
        let cs = f
            .repo()
            .candidates(
                utc("2026-03-01T00:00:00Z"),
                utc("2026-03-31T00:00:00Z"),
                &moscow,
            )
            .unwrap();
        assert_eq!(
            cs.iter()
                .map(|c| (c.title.as_str(), c.fire_at, c.all_day))
                .collect::<Vec<_>>(),
            [
                // 17:00 Moscow.
                ("Сдать отчёт", utc("2026-03-12T14:00:00Z"), false),
                // A day before 09:00 Moscow on the 13th.
                ("Оплатить хостинг", utc("2026-03-12T06:00:00Z"), true),
            ]
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
        );
        // Done tasks do not remind.
        let done = crate::statuses::list(f.db.conn())
            .unwrap()
            .into_iter()
            .find(|s| s.category == crate::statuses::StatusCategory::Done)
            .unwrap();
        tasks.set_status(day, done.id).unwrap();
        assert_eq!(
            f.repo()
                .candidates(
                    utc("2026-03-01T00:00:00Z"),
                    utc("2026-03-31T00:00:00Z"),
                    &moscow
                )
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn deleting_the_owner_deletes_its_reminders() {
        let f = Fixture::new();
        let id = f.event("2026-03-10T10:00", "2026-03-10T11:00", "UTC", None);
        f.repo().set(EntityRef::Event(id), &[5]).unwrap();
        f.events().delete(id).unwrap();
        let left: i64 =
            f.db.conn()
                .query_row("SELECT count(*) FROM reminders", [], |r| r.get(0))
                .unwrap();
        assert_eq!(left, 0);
    }
}
