//! Time tracking (FR-TT-1, 2, 3, 5): entries for a task, a project or a
//! client, and one running timer (an entry without an end). The timer
//! lives in the database, so it survives restarts. Entries billed on an
//! invoice are locked.

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::domain::{Clock, Id, Timestamp};
use crate::{CoreError, CoreResult};

/// What time is spent on: a task (its project and client follow), a
/// project (its client follows) or a client.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TimeTarget {
    pub task_id: Option<Id>,
    pub project_id: Option<Id>,
    pub client_id: Option<Id>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntryInput {
    pub target: TimeTarget,
    pub description: String,
    pub started_at: Timestamp,
    pub ended_at: Timestamp,
    pub billable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntry {
    pub id: Id,
    pub task_id: Option<Id>,
    pub task_title: Option<String>,
    pub project_id: Option<Id>,
    pub project_name: Option<String>,
    pub client_id: Option<Id>,
    pub client_name: Option<String>,
    pub description: String,
    pub started_at: Timestamp,
    /// `None` while the timer runs.
    pub ended_at: Option<Timestamp>,
    pub billable: bool,
    /// Set once billed; the entry is then locked.
    pub invoice_id: Option<Id>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// A saved entry with the entries it overlaps (a warning, not an error).
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedEntry {
    pub entry: TimeEntry,
    pub overlaps: Vec<Id>,
}

pub struct TimeRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> TimeRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    /// Starts the timer. A running one is stopped first if
    /// `stop_previous`, otherwise this fails with `timer.already_running`.
    pub fn start(
        &self,
        target: TimeTarget,
        description: &str,
        billable: bool,
        stop_previous: bool,
    ) -> CoreResult<TimeEntry> {
        let target = self.resolve(target)?;
        if self.running()?.is_some() {
            if !stop_previous {
                return Err(CoreError::new("timer.already_running"));
            }
            self.stop()?;
        }
        let now = self.clock.now();
        self.insert(&target, description.trim(), now, None, billable)
    }

    /// Stops the running timer now.
    pub fn stop(&self) -> CoreResult<TimeEntry> {
        let running = self
            .running()?
            .ok_or_else(|| CoreError::new("timer.not_running"))?;
        let now = self.clock.now().max(running.started_at);
        self.conn.execute(
            "UPDATE time_entries SET ended_at = ?2, updated_at = ?3 WHERE id = ?1",
            (running.id, now, self.clock.now()),
        )?;
        self.get(running.id)
    }

    pub fn running(&self) -> CoreResult<Option<TimeEntry>> {
        Ok(self
            .conn
            .query_row(&format!("{SELECT} WHERE e.ended_at IS NULL"), [], from_row)
            .optional()?)
    }

    pub fn create(&self, input: TimeEntryInput) -> CoreResult<SavedEntry> {
        let target = self.validate(&input)?;
        let entry = self.insert(
            &target,
            input.description.trim(),
            input.started_at,
            Some(input.ended_at),
            input.billable,
        )?;
        self.saved(entry)
    }

    pub fn update(&self, id: Id, input: TimeEntryInput) -> CoreResult<SavedEntry> {
        self.editable(id)?;
        let target = self.validate(&input)?;
        self.conn.execute(
            "UPDATE time_entries SET task_id = ?2, project_id = ?3, client_id = ?4,
                 description = ?5, started_at = ?6, ended_at = ?7, billable = ?8,
                 updated_at = ?9
             WHERE id = ?1",
            rusqlite::params![
                id,
                target.task_id,
                target.project_id,
                target.client_id,
                input.description.trim(),
                input.started_at,
                input.ended_at,
                input.billable,
                self.clock.now(),
            ],
        )?;
        let entry = self.get(id)?;
        self.saved(entry)
    }

    pub fn get(&self, id: Id) -> CoreResult<TimeEntry> {
        self.conn
            .query_row(&format!("{SELECT} WHERE e.id = ?1"), [id], from_row)
            .optional()?
            .ok_or_else(|| CoreError::not_found("time_entry", id))
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        self.editable(id)?;
        self.conn
            .execute("DELETE FROM time_entries WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Fails for billed entries.
    fn editable(&self, id: Id) -> CoreResult<TimeEntry> {
        let entry = self.get(id)?;
        if entry.invoice_id.is_some() {
            return Err(CoreError::new("time.locked"));
        }
        Ok(entry)
    }

    fn validate(&self, input: &TimeEntryInput) -> CoreResult<TimeTarget> {
        if input.ended_at <= input.started_at {
            return Err(CoreError::validation("endedAt", "before_start"));
        }
        self.resolve(input.target)
    }

    /// Fills in the project and client implied by a task or project.
    fn resolve(&self, target: TimeTarget) -> CoreResult<TimeTarget> {
        let mismatch = || CoreError::new("time.client_mismatch");
        let mut out = target;
        if let Some(task) = target.task_id {
            let (project, client): (Option<Id>, Option<Id>) = self
                .conn
                .query_row(
                    "SELECT project_id, client_id FROM tasks WHERE id = ?1",
                    [task],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?
                .ok_or_else(|| CoreError::not_found("task", task))?;
            if target.project_id.is_some_and(|p| Some(p) != project)
                || target.client_id.is_some_and(|c| Some(c) != client)
            {
                return Err(mismatch());
            }
            out.project_id = project;
            out.client_id = client;
        } else if let Some(project) = target.project_id {
            let client: Id = self
                .conn
                .query_row(
                    "SELECT client_id FROM projects WHERE id = ?1",
                    [project],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or_else(|| CoreError::not_found("project", project))?;
            if target.client_id.is_some_and(|c| c != client) {
                return Err(mismatch());
            }
            out.client_id = Some(client);
        } else if let Some(client) = target.client_id {
            crate::db::ensure_exists(self.conn, "clients", "client", client)?;
        }
        Ok(out)
    }

    fn insert(
        &self,
        target: &TimeTarget,
        description: &str,
        started_at: Timestamp,
        ended_at: Option<Timestamp>,
        billable: bool,
    ) -> CoreResult<TimeEntry> {
        let id = Id::new();
        let now = self.clock.now();
        self.conn.execute(
            "INSERT INTO time_entries (id, task_id, project_id, client_id, description,
                 started_at, ended_at, billable, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            rusqlite::params![
                id,
                target.task_id,
                target.project_id,
                target.client_id,
                description,
                started_at,
                ended_at,
                billable,
                now,
            ],
        )?;
        self.get(id)
    }

    /// Other entries sharing time with `entry` (a running one counts until now).
    fn saved(&self, entry: TimeEntry) -> CoreResult<SavedEntry> {
        let end = entry.ended_at.unwrap_or_else(|| self.clock.now());
        let mut stmt = self.conn.prepare(
            "SELECT id FROM time_entries
             WHERE id != ?1 AND started_at < ?3 AND coalesce(ended_at, ?4) > ?2
             ORDER BY started_at",
        )?;
        let overlaps = stmt
            .query_map((entry.id, entry.started_at, end, self.clock.now()), |r| {
                r.get(0)
            })?
            .collect::<Result<_, _>>()?;
        Ok(SavedEntry { entry, overlaps })
    }
}

pub(crate) const SELECT: &str = "SELECT e.id, e.task_id, t.title AS task_title, e.project_id,
        p.name AS project_name, e.client_id, c.display_name AS client_name, e.description,
        e.started_at, e.ended_at, e.billable, e.invoice_id, e.created_at, e.updated_at
     FROM time_entries e
     LEFT JOIN tasks t ON t.id = e.task_id
     LEFT JOIN projects p ON p.id = e.project_id
     LEFT JOIN clients c ON c.id = e.client_id";

pub(crate) fn from_row(r: &Row) -> rusqlite::Result<TimeEntry> {
    Ok(TimeEntry {
        id: r.get("id")?,
        task_id: r.get("task_id")?,
        task_title: r.get("task_title")?,
        project_id: r.get("project_id")?,
        project_name: r.get("project_name")?,
        client_id: r.get("client_id")?,
        client_name: r.get("client_name")?,
        description: r.get("description")?,
        started_at: r.get("started_at")?,
        ended_at: r.get("ended_at")?,
        billable: r.get("billable")?,
        invoice_id: r.get("invoice_id")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};
    use crate::tasks::{Priority, TaskInput, TaskRepo};

    const MIN: i64 = 60_000;

    fn utc(s: &str) -> Timestamp {
        Timestamp::from_millis(s.parse::<jiff::Timestamp>().unwrap().as_millisecond())
    }

    struct Fixture {
        db: Db,
        clock: FakeClock,
        client: Id,
        project: Id,
        task: Id,
    }

    impl Fixture {
        fn new() -> Self {
            let db = Db::open_in_memory().unwrap();
            let clock = FakeClock::at(utc("2026-03-10T09:00:00Z"));
            let client = ClientRepo::new(db.conn(), &clock)
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
                .unwrap()
                .id;
            let project = ProjectRepo::new(db.conn(), &clock)
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
                .id;
            let task = TaskRepo::new(db.conn(), &clock)
                .create(TaskInput {
                    title: "Макет".into(),
                    description_md: String::new(),
                    status_id: None,
                    priority: Priority::Normal,
                    project_id: Some(project),
                    client_id: None,
                    due_date: None,
                    due_time: None,
                    estimate_minutes: None,
                })
                .unwrap()
                .id;
            Self {
                db,
                clock,
                client,
                project,
                task,
            }
        }

        fn repo(&self) -> TimeRepo<'_> {
            TimeRepo::new(self.db.conn(), &self.clock)
        }

        fn on_task(&self) -> TimeTarget {
            TimeTarget {
                task_id: Some(self.task),
                ..Default::default()
            }
        }

        fn manual(&self, from: &str, to: &str) -> TimeEntryInput {
            TimeEntryInput {
                target: self.on_task(),
                description: "Вёрстка".into(),
                started_at: utc(from),
                ended_at: utc(to),
                billable: true,
            }
        }
    }

    #[test]
    fn a_timer_runs_until_stopped() {
        let f = Fixture::new();
        let started = f
            .repo()
            .start(f.on_task(), " Макет главной ", true, true)
            .unwrap();
        assert_eq!(started.ended_at, None);
        assert_eq!(started.started_at, utc("2026-03-10T09:00:00Z"));
        assert_eq!(started.description, "Макет главной");
        // The task brings its project and client.
        assert_eq!(started.project_id, Some(f.project));
        assert_eq!(started.client_name.as_deref(), Some("Ромашка"));
        assert_eq!(started.task_title.as_deref(), Some("Макет"));
        assert_eq!(f.repo().running().unwrap(), Some(started.clone()));

        f.clock.advance_millis(25 * MIN);
        let stopped = f.repo().stop().unwrap();
        assert_eq!(stopped.id, started.id);
        assert_eq!(stopped.ended_at, Some(utc("2026-03-10T09:25:00Z")));
        assert_eq!(f.repo().running().unwrap(), None);
        assert_eq!(
            f.repo().stop().unwrap_err(),
            CoreError::new("timer.not_running")
        );
    }

    #[test]
    fn starting_again_stops_the_previous_timer_or_fails() {
        let f = Fixture::new();
        let first = f.repo().start(f.on_task(), "", true, true).unwrap();
        f.clock.advance_millis(10 * MIN);
        let on_project = TimeTarget {
            project_id: Some(f.project),
            ..Default::default()
        };
        assert_eq!(
            f.repo().start(on_project, "", true, false).unwrap_err(),
            CoreError::new("timer.already_running")
        );
        let second = f.repo().start(on_project, "", false, true).unwrap();
        assert_eq!(
            f.repo().get(first.id).unwrap().ended_at,
            Some(utc("2026-03-10T09:10:00Z"))
        );
        assert_eq!(second.client_id, Some(f.client));
        assert!(!second.billable);
        assert_eq!(f.repo().running().unwrap().unwrap().id, second.id);
    }

    #[test]
    fn the_timer_survives_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("crm.db");
        let clock = FakeClock::at(utc("2026-03-10T09:00:00Z"));
        let id = {
            let db = Db::open(&path).unwrap();
            TimeRepo::new(db.conn(), &clock)
                .start(TimeTarget::default(), "Без проекта", false, true)
                .unwrap()
                .id
        };
        clock.advance_millis(60 * MIN);
        let db = Db::open(&path).unwrap();
        let repo = TimeRepo::new(db.conn(), &clock);
        assert_eq!(repo.running().unwrap().unwrap().id, id);
        assert_eq!(
            repo.stop().unwrap().ended_at,
            Some(utc("2026-03-10T10:00:00Z"))
        );
    }

    #[test]
    fn manual_entries_warn_about_overlaps() {
        let f = Fixture::new();
        let first = f
            .repo()
            .create(f.manual("2026-03-09T10:00:00Z", "2026-03-09T12:00:00Z"))
            .unwrap();
        assert!(first.overlaps.is_empty());
        let second = f
            .repo()
            .create(f.manual("2026-03-09T11:30:00Z", "2026-03-09T13:00:00Z"))
            .unwrap();
        assert_eq!(second.overlaps, [first.entry.id]);
        // Touching is not overlapping.
        let third = f
            .repo()
            .create(f.manual("2026-03-09T13:00:00Z", "2026-03-09T14:00:00Z"))
            .unwrap();
        assert!(third.overlaps.is_empty());
        // An entry does not overlap itself when edited.
        let edited = f
            .repo()
            .update(
                third.entry.id,
                f.manual("2026-03-09T13:00:00Z", "2026-03-09T14:30:00Z"),
            )
            .unwrap();
        assert!(edited.overlaps.is_empty());
        assert_eq!(edited.entry.ended_at, Some(utc("2026-03-09T14:30:00Z")));
    }

    #[test]
    fn validates_entries() {
        let f = Fixture::new();
        assert_eq!(
            f.repo()
                .create(f.manual("2026-03-09T12:00:00Z", "2026-03-09T12:00:00Z"))
                .unwrap_err(),
            CoreError::validation("endedAt", "before_start")
        );
        let mut wrong = f.manual("2026-03-09T10:00:00Z", "2026-03-09T11:00:00Z");
        wrong.target = TimeTarget {
            project_id: Some(f.project),
            client_id: Some(Id::new()),
            ..Default::default()
        };
        assert_eq!(
            f.repo().create(wrong).unwrap_err(),
            CoreError::new("time.client_mismatch")
        );
        let missing = Id::new();
        let mut gone = f.manual("2026-03-09T10:00:00Z", "2026-03-09T11:00:00Z");
        gone.target = TimeTarget {
            task_id: Some(missing),
            ..Default::default()
        };
        assert_eq!(
            f.repo().create(gone).unwrap_err(),
            CoreError::not_found("task", missing)
        );
    }

    #[test]
    fn invoiced_entries_are_locked() {
        let f = Fixture::new();
        let saved = f
            .repo()
            .create(f.manual("2026-03-09T10:00:00Z", "2026-03-09T11:00:00Z"))
            .unwrap();
        let invoice = Id::new();
        f.db.conn()
            .execute(
                "UPDATE time_entries SET invoice_id = ?2 WHERE id = ?1",
                (saved.entry.id, invoice),
            )
            .unwrap();
        assert_eq!(
            f.repo().get(saved.entry.id).unwrap().invoice_id,
            Some(invoice)
        );
        let locked = CoreError::new("time.locked");
        assert_eq!(
            f.repo()
                .update(
                    saved.entry.id,
                    f.manual("2026-03-09T10:00:00Z", "2026-03-09T12:00:00Z")
                )
                .unwrap_err(),
            locked
        );
        assert_eq!(f.repo().delete(saved.entry.id).unwrap_err(), locked);
    }

    #[test]
    fn deleting_a_task_keeps_its_time_on_the_project() {
        let f = Fixture::new();
        let saved = f
            .repo()
            .create(f.manual("2026-03-09T10:00:00Z", "2026-03-09T11:00:00Z"))
            .unwrap();
        TaskRepo::new(f.db.conn(), &f.clock).delete(f.task).unwrap();
        let kept = f.repo().get(saved.entry.id).unwrap();
        assert_eq!((kept.task_id, kept.project_id), (None, Some(f.project)));
        f.repo().delete(saved.entry.id).unwrap();
        assert_eq!(
            f.repo().get(saved.entry.id).unwrap_err().code,
            "time_entry.not_found"
        );
    }
}
