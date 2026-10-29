//! What the calendar shows (FR-CA-3): event occurrences plus task and
//! project deadlines (and invoice due dates once invoices exist), each kind
//! a layer that can be switched off.

use jiff::tz::TimeZone;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::events::{EventRepo, Occurrence};
use crate::CoreResult;
use crate::domain::{Clock, Id, Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CalendarLayers {
    pub events: bool,
    pub tasks: bool,
    pub projects: bool,
    pub invoices: bool,
    /// Done tasks and completed or cancelled projects.
    pub show_completed: bool,
}

impl Default for CalendarLayers {
    fn default() -> Self {
        Self {
            events: true,
            tasks: true,
            projects: true,
            invoices: true,
            show_completed: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Event,
    TaskDue,
    ProjectDeadline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CalendarItem {
    pub kind: ItemKind,
    /// The event, task or project.
    pub id: Id,
    pub title: String,
    pub all_day: bool,
    /// Local values: `YYYY-MM-DD` for all-day items, else `YYYY-MM-DDTHH:MM`.
    pub start: String,
    pub end: String,
    pub start_at: Timestamp,
    pub end_at: Timestamp,
    pub client_name: Option<String>,
    pub project_name: Option<String>,
    /// Done task or finished project.
    pub completed: bool,
    /// For events: the occurrence, needed to edit a series.
    pub occurrence: Option<Occurrence>,
}

/// Items overlapping `[from, to)`, by start (all-day items first).
/// Deadlines with a time are moments in `tz`, the user's zone.
pub fn calendar_items(
    conn: &Connection,
    clock: &dyn Clock,
    from: Timestamp,
    to: Timestamp,
    layers: CalendarLayers,
    tz: &TimeZone,
) -> CoreResult<Vec<CalendarItem>> {
    let mut items = Vec::new();
    if layers.events {
        for o in EventRepo::new(conn, clock).occurrences(from, to)? {
            items.push(CalendarItem {
                kind: ItemKind::Event,
                id: o.event.id,
                title: o.event.title.clone(),
                all_day: o.event.all_day,
                start: o.start.clone(),
                end: o.end.clone(),
                start_at: o.start_at,
                end_at: o.end_at,
                client_name: o.event.client_name.clone(),
                project_name: o.event.project_name.clone(),
                completed: false,
                occurrence: Some(o),
            });
        }
    }
    // Dates are compared as text with a margin: the exact overlap is
    // checked once the instants are known.
    let (first, last) = (margin_date(from, -2)?, margin_date(to, 2)?);
    if layers.tasks {
        let mut stmt = conn.prepare(
            "SELECT t.id, t.title, t.due_date, t.due_time, t.completed_at IS NOT NULL,
                    c.display_name, p.name
             FROM tasks t
             LEFT JOIN clients c ON c.id = t.client_id
             LEFT JOIN projects p ON p.id = t.project_id
             WHERE t.due_date BETWEEN ?1 AND ?2 AND (?3 OR t.completed_at IS NULL)",
        )?;
        let rows = stmt.query_map((&first, &last, layers.show_completed), |r| {
            Ok((
                r.get::<_, Id>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, bool>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
            ))
        })?;
        for row in rows {
            let (id, title, date, time, completed, client_name, project_name) = row?;
            let Some(when) = deadline(&date, time.as_deref(), tz) else {
                continue;
            };
            items.push(CalendarItem {
                kind: ItemKind::TaskDue,
                id,
                title,
                completed,
                client_name,
                project_name,
                occurrence: None,
                all_day: when.all_day,
                end: when.start.clone(),
                start: when.start,
                start_at: when.start_at,
                end_at: when.end_at,
            });
        }
    }
    if layers.projects {
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, p.deadline, p.status IN ('completed', 'cancelled'),
                    c.display_name
             FROM projects p JOIN clients c ON c.id = p.client_id
             WHERE p.deadline BETWEEN ?1 AND ?2
               AND (?3 OR p.status NOT IN ('completed', 'cancelled'))",
        )?;
        let rows = stmt.query_map((&first, &last, layers.show_completed), |r| {
            Ok((
                r.get::<_, Id>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, bool>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?;
        for row in rows {
            let (id, name, date, completed, client_name) = row?;
            let Some(when) = deadline(&date, None, tz) else {
                continue;
            };
            items.push(CalendarItem {
                kind: ItemKind::ProjectDeadline,
                id,
                project_name: Some(name.clone()),
                title: name,
                completed,
                client_name: Some(client_name),
                occurrence: None,
                all_day: when.all_day,
                end: when.start.clone(),
                start: when.start,
                start_at: when.start_at,
                end_at: when.end_at,
            });
        }
    }
    // Invoices (M8) join here behind `layers.invoices`.
    items.retain(|i| i.start_at < to && (i.end_at > from || i.start_at == from));
    items.sort_by(|a, b| {
        (a.start_at, !a.all_day, a.kind as u8, &a.title).cmp(&(
            b.start_at,
            !b.all_day,
            b.kind as u8,
            &b.title,
        ))
    });
    Ok(items)
}

/// The date `days` away from the instant, as `YYYY-MM-DD` text.
fn margin_date(at: Timestamp, days: i64) -> CoreResult<String> {
    let date = jiff::Timestamp::from_millisecond(at.as_millis())
        .map_err(|_| crate::CoreError::new("internal.clock"))?
        .to_zoned(TimeZone::UTC)
        .date();
    Ok(date
        .checked_add(jiff::Span::new().days(days))
        .map_err(|_| crate::CoreError::new("internal.clock"))?
        .to_string())
}

/// Timing of a deadline.
struct When {
    all_day: bool,
    start: String,
    start_at: Timestamp,
    end_at: Timestamp,
}

/// A whole (floating) day, or a moment at `time` in `tz`.
fn deadline(date: &str, time: Option<&str>, tz: &TimeZone) -> Option<When> {
    let day: jiff::civil::Date = date.parse().ok()?;
    Some(match time {
        None => {
            let midnight = jiff::civil::Time::midnight();
            let next = day.tomorrow().ok()?.to_datetime(midnight);
            When {
                all_day: true,
                start: date.to_owned(),
                start_at: super::events::instant(day.to_datetime(midnight), &TimeZone::UTC).ok()?,
                end_at: super::events::instant(next, &TimeZone::UTC).ok()?,
            }
        }
        Some(time) => {
            let local = super::events::parse_datetime(&format!("{date}T{time}"))?;
            let at = super::events::instant(local, tz).ok()?;
            When {
                all_day: false,
                start: super::events::format_local(local),
                start_at: at,
                end_at: at,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::events::EventInput;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};
    use crate::tasks::{Priority, TaskInput, TaskRepo};

    struct Fixture {
        db: Db,
        clock: FakeClock,
    }

    fn utc(s: &str) -> Timestamp {
        let ts: jiff::Timestamp = s.parse().unwrap();
        Timestamp::from_millis(ts.as_millisecond())
    }

    impl Fixture {
        /// A client "Ромашка" with project "Сайт" due 2026-03-20, tasks due
        /// 2026-03-10 (all day), 2026-03-12 18:00 and 2026-03-11 (done),
        /// and an event on 2026-03-10 09:00–10:00 UTC.
        fn new() -> Self {
            let f = Self {
                db: Db::open_in_memory().unwrap(),
                clock: FakeClock::at(utc("2026-03-01T00:00:00Z")),
            };
            let conn = f.db.conn();
            let client = ClientRepo::new(conn, &f.clock)
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
            let project = |name: &str, deadline: &str, status: ProjectStatus| {
                ProjectRepo::new(conn, &f.clock)
                    .create(ProjectInput {
                        client_id: client.id,
                        name: name.into(),
                        description_md: String::new(),
                        status,
                        billing_type: BillingType::Fixed,
                        currency: Some("RUB".parse().unwrap()),
                        rate_minor: None,
                        budget_minor: Some(1),
                        start_date: None,
                        deadline: Some(deadline.into()),
                    })
                    .unwrap()
            };
            let site = project("Сайт", "2026-03-20", ProjectStatus::Active);
            project("Старый", "2026-03-21", ProjectStatus::Completed);
            let tasks = TaskRepo::new(conn, &f.clock);
            let task = |title: &str, date: &str, time: Option<&str>| {
                tasks
                    .create(TaskInput {
                        title: title.into(),
                        description_md: String::new(),
                        status_id: None,
                        priority: Priority::Normal,
                        project_id: Some(site.id),
                        client_id: None,
                        due_date: Some(date.into()),
                        due_time: time.map(Into::into),
                        estimate_minutes: None,
                    })
                    .unwrap()
            };
            task("Макет", "2026-03-10", None);
            task("Созвон по макету", "2026-03-12", Some("18:00"));
            let done = task("Готово", "2026-03-11", None);
            let done_column = crate::statuses::list(conn)
                .unwrap()
                .into_iter()
                .find(|s| s.category == crate::statuses::StatusCategory::Done)
                .unwrap();
            tasks.set_status(done.id, done_column.id).unwrap();
            EventRepo::new(conn, &f.clock)
                .create(EventInput {
                    title: "Встреча".into(),
                    description_md: String::new(),
                    location: None,
                    all_day: false,
                    start: "2026-03-10T09:00".into(),
                    end: "2026-03-10T10:00".into(),
                    tzid: "UTC".into(),
                    client_id: Some(client.id),
                    project_id: None,
                    task_id: None,
                    contact_id: None,
                    rrule: None,
                })
                .unwrap();
            f
        }

        fn titles(&self, layers: CalendarLayers) -> Vec<String> {
            calendar_items(
                self.db.conn(),
                &self.clock,
                utc("2026-03-01T00:00:00Z"),
                utc("2026-04-01T00:00:00Z"),
                layers,
                &TimeZone::get("Europe/Moscow").unwrap(),
            )
            .unwrap()
            .into_iter()
            .map(|i| i.title)
            .collect()
        }
    }

    #[test]
    fn merges_layers_in_time_order() {
        let f = Fixture::new();
        assert_eq!(
            f.titles(CalendarLayers::default()),
            ["Макет", "Встреча", "Созвон по макету", "Сайт"]
        );
    }

    #[test]
    fn each_layer_can_be_switched_off() {
        let f = Fixture::new();
        let all = CalendarLayers::default();
        assert_eq!(
            f.titles(CalendarLayers {
                events: false,
                ..all
            }),
            ["Макет", "Созвон по макету", "Сайт"]
        );
        assert_eq!(
            f.titles(CalendarLayers {
                tasks: false,
                ..all
            }),
            ["Встреча", "Сайт"]
        );
        assert_eq!(
            f.titles(CalendarLayers {
                projects: false,
                ..all
            }),
            ["Макет", "Встреча", "Созвон по макету"]
        );
    }

    #[test]
    fn completed_items_are_hidden_unless_asked() {
        let f = Fixture::new();
        assert_eq!(
            f.titles(CalendarLayers {
                show_completed: true,
                ..CalendarLayers::default()
            }),
            [
                "Макет",
                "Встреча",
                "Готово",
                "Созвон по макету",
                "Сайт",
                "Старый"
            ]
        );
    }

    #[test]
    fn deadlines_are_dates_or_moments_in_the_users_zone() {
        let f = Fixture::new();
        let items = calendar_items(
            f.db.conn(),
            &f.clock,
            utc("2026-03-12T00:00:00Z"),
            utc("2026-03-13T00:00:00Z"),
            CalendarLayers::default(),
            &TimeZone::get("Europe/Moscow").unwrap(),
        )
        .unwrap();
        assert_eq!(items.len(), 1);
        let call = &items[0];
        assert_eq!(call.kind, ItemKind::TaskDue);
        assert!(!call.all_day);
        assert_eq!(
            (call.start.as_str(), call.end.as_str()),
            ("2026-03-12T18:00", "2026-03-12T18:00")
        );
        // 18:00 in Moscow.
        assert_eq!(call.start_at, utc("2026-03-12T15:00:00Z"));
        assert_eq!(call.project_name.as_deref(), Some("Сайт"));
        assert_eq!(call.client_name.as_deref(), Some("Ромашка"));

        let site = calendar_items(
            f.db.conn(),
            &f.clock,
            utc("2026-03-20T00:00:00Z"),
            utc("2026-03-21T00:00:00Z"),
            CalendarLayers::default(),
            &TimeZone::UTC,
        )
        .unwrap();
        assert_eq!(site[0].kind, ItemKind::ProjectDeadline);
        assert!(site[0].all_day);
        assert_eq!(
            (site[0].start.as_str(), site[0].end.as_str()),
            ("2026-03-20", "2026-03-20")
        );
        assert_eq!(site[0].end_at, utc("2026-03-21T00:00:00Z"));
    }

    #[test]
    fn events_carry_their_occurrence() {
        let f = Fixture::new();
        let items = calendar_items(
            f.db.conn(),
            &f.clock,
            utc("2026-03-10T00:00:00Z"),
            utc("2026-03-11T00:00:00Z"),
            CalendarLayers {
                tasks: false,
                ..CalendarLayers::default()
            },
            &TimeZone::UTC,
        )
        .unwrap();
        assert_eq!(items[0].kind, ItemKind::Event);
        let occurrence = items[0].occurrence.as_ref().unwrap();
        assert_eq!(occurrence.event.id, items[0].id);
        assert_eq!(items[0].client_name.as_deref(), Some("Ромашка"));
    }
}
