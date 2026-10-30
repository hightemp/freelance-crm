//! The home screen (FR-SR-3): what needs attention now, in one call:
//! overdue tasks and today's, the next events, invoices awaiting payment
//! and the clients worked with most recently. The running timer is shown
//! from its own state.

use jiff::tz::TimeZone;
use rusqlite::Connection;
use serde::Serialize;

use crate::CoreResult;
use crate::calendar::items::CalendarItem;
use crate::domain::{Clock, Id, Timestamp};
use crate::finance::dashboard::OpenInvoice;
use crate::tasks::Task;

/// How many of each list the home screen shows.
pub const LIMIT: usize = 5;
/// How far ahead events are looked for.
pub const EVENTS_AHEAD_MS: i64 = 7 * 24 * 3_600_000;

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecentClient {
    pub id: Id,
    pub display_name: String,
    /// Latest change to the client, its projects, tasks, time, interactions
    /// or invoices.
    pub active_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HomeSummary {
    /// Past their deadline, earliest first.
    pub overdue_tasks: Vec<Task>,
    /// Due today and not overdue yet.
    pub today_tasks: Vec<Task>,
    /// Events not over yet within the next week, soonest first.
    pub events: Vec<CalendarItem>,
    pub open_invoices: Vec<OpenInvoice>,
    pub recent_clients: Vec<RecentClient>,
}

pub fn summary(conn: &Connection, clock: &dyn Clock, tz: &TimeZone) -> CoreResult<HomeSummary> {
    use crate::calendar::items::{CalendarLayers, calendar_items};
    use crate::tasks::{DuePreset, TaskFilter, TaskRepo};

    let tasks = TaskRepo::new(conn, clock);
    let due = |preset| {
        let mut list = tasks.list(
            &TaskFilter {
                due: Some(preset),
                hide_done: true,
                ..TaskFilter::default()
            },
            tz,
        )?;
        // By deadline; a whole day before the times on it.
        list.sort_by(|a: &Task, b: &Task| {
            (&a.due_date, &a.due_time, &a.title).cmp(&(&b.due_date, &b.due_time, &b.title))
        });
        CoreResult::Ok(list)
    };
    let mut overdue_tasks = due(DuePreset::Overdue)?;
    let mut today_tasks = due(DuePreset::Today)?;
    today_tasks.retain(|t| !overdue_tasks.iter().any(|o| o.id == t.id));
    overdue_tasks.truncate(LIMIT);
    today_tasks.truncate(LIMIT);

    let now = clock.now();
    let mut events = calendar_items(
        conn,
        clock,
        now,
        Timestamp::from_millis(now.as_millis() + EVENTS_AHEAD_MS),
        CalendarLayers {
            events: true,
            tasks: false,
            projects: false,
            invoices: false,
            show_completed: false,
        },
        tz,
    )?;
    events.truncate(LIMIT);

    let mut open_invoices = crate::finance::dashboard::open_invoices(conn, clock, tz.clone())?;
    open_invoices.truncate(LIMIT);

    // The client's own changes and those of what belongs to it.
    let mut stmt = conn.prepare(
        "SELECT c.id, c.display_name, max(a.at) AS active_at
         FROM clients c JOIN (
             SELECT id AS client_id, updated_at AS at FROM clients
             UNION ALL SELECT client_id, updated_at FROM projects
             UNION ALL SELECT coalesce(t.client_id, p.client_id), t.updated_at
                 FROM tasks t LEFT JOIN projects p ON p.id = t.project_id
             UNION ALL SELECT coalesce(e.client_id, p.client_id), e.updated_at
                 FROM time_entries e LEFT JOIN projects p ON p.id = e.project_id
             UNION ALL SELECT client_id, occurred_at FROM interactions
             UNION ALL SELECT client_id, updated_at FROM invoices
         ) a ON a.client_id = c.id
         WHERE c.archived_at IS NULL
         GROUP BY c.id
         ORDER BY active_at DESC, c.sort_key
         LIMIT ?1",
    )?;
    let recent_clients = stmt
        .query_map([LIMIT as i64], |r| {
            Ok(RecentClient {
                id: r.get(0)?,
                display_name: r.get(1)?,
                active_at: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(HomeSummary {
        overdue_tasks,
        today_tasks,
        events,
        open_invoices,
        recent_clients,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::events::{EventInput, EventRepo};
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::tasks::{Priority, TaskInput, TaskRepo};

    fn utc(s: &str) -> Timestamp {
        let ts: jiff::Timestamp = s.parse().unwrap();
        Timestamp::from_millis(ts.as_millisecond())
    }

    struct Fixture {
        db: Db,
        /// 2026-03-10 12:00 UTC.
        clock: FakeClock,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                db: Db::open_in_memory().unwrap(),
                clock: FakeClock::at(utc("2026-03-10T12:00:00Z")),
            }
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

        fn task(
            &self,
            title: &str,
            due: Option<&str>,
            time: Option<&str>,
            client: Option<Id>,
        ) -> Id {
            TaskRepo::new(self.db.conn(), &self.clock)
                .create(TaskInput {
                    title: title.into(),
                    description_md: String::new(),
                    status_id: None,
                    priority: Priority::Normal,
                    project_id: None,
                    client_id: client,
                    due_date: due.map(Into::into),
                    due_time: time.map(Into::into),
                    estimate_minutes: None,
                })
                .unwrap()
                .id
        }

        fn event(&self, title: &str, start: &str, end: &str) {
            EventRepo::new(self.db.conn(), &self.clock)
                .create(EventInput {
                    title: title.into(),
                    description_md: String::new(),
                    location: None,
                    all_day: false,
                    start: start.into(),
                    end: end.into(),
                    tzid: "UTC".into(),
                    client_id: None,
                    project_id: None,
                    task_id: None,
                    contact_id: None,
                    rrule: None,
                })
                .unwrap();
        }

        fn summary(&self) -> HomeSummary {
            summary(self.db.conn(), &self.clock, &TimeZone::UTC).unwrap()
        }
    }

    fn titles(tasks: &[Task]) -> Vec<&str> {
        tasks.iter().map(|t| t.title.as_str()).collect()
    }

    #[test]
    fn empty_lists_without_data() {
        let s = Fixture::new().summary();
        assert!(s.overdue_tasks.is_empty());
        assert!(s.today_tasks.is_empty());
        assert!(s.events.is_empty());
        assert!(s.open_invoices.is_empty());
        assert!(s.recent_clients.is_empty());
    }

    #[test]
    fn overdue_and_today_tasks_do_not_repeat() {
        let f = Fixture::new();
        f.task("Вчерашнее", Some("2026-03-09"), None, None);
        f.task("Утром", Some("2026-03-10"), Some("09:00"), None);
        f.task("Вечером", Some("2026-03-10"), Some("18:00"), None);
        f.task("Сегодня", Some("2026-03-10"), None, None);
        f.task("Завтра", Some("2026-03-11"), None, None);
        f.task("Когда-нибудь", None, None, None);
        let s = f.summary();
        assert_eq!(titles(&s.overdue_tasks), ["Вчерашнее", "Утром"]);
        assert_eq!(titles(&s.today_tasks), ["Сегодня", "Вечером"]);
    }

    #[test]
    fn the_next_events_of_the_week() {
        let f = Fixture::new();
        f.event("Прошло", "2026-03-10T08:00", "2026-03-10T09:00");
        f.event("Идёт", "2026-03-10T11:30", "2026-03-10T12:30");
        f.event("Завтра", "2026-03-11T10:00", "2026-03-11T11:00");
        f.event("Через месяц", "2026-04-10T10:00", "2026-04-10T11:00");
        let s = f.summary();
        let events: Vec<_> = s.events.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(events, ["Идёт", "Завтра"]);
    }

    #[test]
    fn recent_clients_by_their_latest_activity() {
        let f = Fixture::new();
        let old = f.client("Давний");
        f.clock.advance_millis(60_000);
        let quiet = f.client("Тихий");
        f.clock.advance_millis(60_000);
        f.client("Новый");
        f.clock.advance_millis(60_000);
        // Work for the oldest client makes it the most recent.
        f.task("Позвонить", None, None, Some(old));
        let s = f.summary();
        let names: Vec<_> = s
            .recent_clients
            .iter()
            .map(|c| c.display_name.as_str())
            .collect();
        assert_eq!(names, ["Давний", "Новый", "Тихий"]);
        assert_eq!(s.recent_clients[0].active_at, f.clock.now());
        // Archived clients are left out.
        ClientRepo::new(f.db.conn(), &f.clock)
            .archive(quiet)
            .unwrap();
        assert_eq!(f.summary().recent_clients.len(), 2);
    }

    #[test]
    fn lists_are_short() {
        let f = Fixture::new();
        for i in 0..8 {
            f.task(&format!("Задача {i}"), Some("2026-03-01"), None, None);
            f.client(&format!("Клиент {i}"));
        }
        let s = f.summary();
        assert_eq!(s.overdue_tasks.len(), LIMIT);
        assert_eq!(s.recent_clients.len(), LIMIT);
    }
}
