//! Tasks: in a project, only for a client, or in the inbox (FR-TS-1, 2).
//! Deadlines are local wall-clock dates (and optional times), so a task
//! due on Friday stays due on Friday when the time zone changes.

use rusqlite::{Connection, OptionalExtension, Row, named_params};
use serde::{Deserialize, Serialize};

use crate::db::text_enum;
use crate::domain::date::is_iso_date;
use crate::domain::position::key_between;
use crate::domain::{Clock, Id, Timestamp};
use crate::statuses::{self, StatusCategory};
use crate::{CoreError, CoreResult};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}

text_enum!(Priority {
    Low => "low",
    Normal => "normal",
    High => "high",
    Urgent => "urgent",
});

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub title: String,
    pub description_md: String,
    /// `None` puts the task into the first `todo` column.
    pub status_id: Option<Id>,
    pub priority: Priority,
    /// With a project, the client comes from it.
    pub project_id: Option<Id>,
    pub client_id: Option<Id>,
    /// Local date `YYYY-MM-DD`.
    pub due_date: Option<String>,
    /// Local time `HH:MM`; only with a date.
    pub due_time: Option<String>,
    pub estimate_minutes: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: Id,
    pub title: String,
    pub description_md: String,
    pub status_id: Id,
    pub priority: Priority,
    pub project_id: Option<Id>,
    pub project_name: Option<String>,
    pub client_id: Option<Id>,
    pub client_name: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub estimate_minutes: Option<u32>,
    /// Fractional index within the status column.
    pub board_position: String,
    pub completed_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

const SELECT: &str = "SELECT t.id, t.title, t.description_md, t.status_id, t.priority,
    t.project_id, p.name AS project_name, t.client_id, c.display_name AS client_name,
    t.due_date, t.due_time, t.estimate_minutes, t.board_position, t.completed_at,
    t.created_at, t.updated_at
    FROM tasks t
    LEFT JOIN projects p ON p.id = t.project_id
    LEFT JOIN clients c ON c.id = t.client_id";

fn from_row(row: &Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get("id")?,
        title: row.get("title")?,
        description_md: row.get("description_md")?,
        status_id: row.get("status_id")?,
        priority: row.get("priority")?,
        project_id: row.get("project_id")?,
        project_name: row.get("project_name")?,
        client_id: row.get("client_id")?,
        client_name: row.get("client_name")?,
        due_date: row.get("due_date")?,
        due_time: row.get("due_time")?,
        estimate_minutes: row.get("estimate_minutes")?,
        board_position: row.get("board_position")?,
        completed_at: row.get("completed_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn not_found(id: Id) -> CoreError {
    CoreError::not_found("task", id)
}

/// `HH:MM`, 24-hour.
fn is_time(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 5
        && b[2] == b':'
        && [0, 1, 3, 4].iter().all(|&i| b[i].is_ascii_digit())
        && value[..2] < *"24"
        && value[3..] < *"60"
}

/// Validated input with the client taken from the project.
struct Resolved {
    input: TaskInput,
    status_id: Id,
    done: bool,
}

pub struct TaskRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> TaskRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    fn resolve(&self, input: TaskInput) -> CoreResult<Resolved> {
        let title = input.title.trim().to_owned();
        if title.is_empty() {
            return Err(CoreError::validation("title", "required"));
        }
        let due_date = crate::domain::text::clean(input.due_date);
        let due_time = crate::domain::text::clean(input.due_time);
        if due_date.as_deref().is_some_and(|d| !is_iso_date(d)) {
            return Err(CoreError::validation("dueDate", "date"));
        }
        if let Some(time) = &due_time {
            if due_date.is_none() {
                return Err(CoreError::validation("dueTime", "needs_date"));
            }
            if !is_time(time) {
                return Err(CoreError::validation("dueTime", "time"));
            }
        }
        let client_id = match input.project_id {
            Some(project_id) => {
                let project_client: Option<Id> = self
                    .conn
                    .query_row(
                        "SELECT client_id FROM projects WHERE id = ?1",
                        [project_id],
                        |r| r.get(0),
                    )
                    .optional()?;
                let project_client =
                    project_client.ok_or_else(|| CoreError::not_found("project", project_id))?;
                if input.client_id.is_some_and(|c| c != project_client) {
                    return Err(CoreError::new("task.client_mismatch"));
                }
                Some(project_client)
            }
            None => {
                if let Some(id) = input.client_id {
                    crate::db::ensure_exists(self.conn, "clients", "client", id)?;
                }
                input.client_id
            }
        };
        let status = match input.status_id {
            Some(id) => statuses::get(self.conn, id)?,
            None => statuses::default_status(self.conn)?,
        };
        Ok(Resolved {
            status_id: status.id,
            done: status.category == StatusCategory::Done,
            input: TaskInput {
                title,
                due_date,
                due_time,
                client_id,
                status_id: Some(status.id),
                ..input
            },
        })
    }

    /// A position after the last task of the column.
    fn end_of(&self, status_id: Id) -> CoreResult<String> {
        let last: Option<String> = self.conn.query_row(
            "SELECT max(board_position) FROM tasks WHERE status_id = ?1",
            [status_id],
            |r| r.get(0),
        )?;
        Ok(key_between(last.as_deref(), None))
    }

    pub fn create(&self, input: TaskInput) -> CoreResult<Task> {
        let Resolved {
            input: t,
            status_id,
            done,
        } = self.resolve(input)?;
        let id = Id::new();
        let now = self.clock.now();
        self.conn.execute(
            "INSERT INTO tasks (id, title, description_md, status_id, priority, project_id,
                 client_id, due_date, due_time, estimate_minutes, board_position, completed_at,
                 created_at, updated_at)
             VALUES (:id, :title, :description, :status, :priority, :project, :client, :due_date,
                 :due_time, :estimate, :position, :completed, :now, :now)",
            named_params! {
                ":id": id,
                ":title": t.title,
                ":description": t.description_md,
                ":status": status_id,
                ":priority": t.priority,
                ":project": t.project_id,
                ":client": t.client_id,
                ":due_date": t.due_date,
                ":due_time": t.due_time,
                ":estimate": t.estimate_minutes,
                ":position": self.end_of(status_id)?,
                ":completed": done.then_some(now),
                ":now": now,
            },
        )?;
        self.get(id)
    }

    pub fn get(&self, id: Id) -> CoreResult<Task> {
        self.conn
            .query_row(&format!("{SELECT} WHERE t.id = ?1"), [id], from_row)
            .optional()?
            .ok_or_else(|| not_found(id))
    }

    /// Position and completion time after (possibly) changing the column.
    fn placement(
        &self,
        current: &Task,
        status_id: Id,
        done: bool,
    ) -> CoreResult<(String, Option<Timestamp>)> {
        let position = if status_id == current.status_id {
            current.board_position.clone()
        } else {
            self.end_of(status_id)?
        };
        let completed_at = match (done, current.completed_at) {
            (false, _) => None,
            (true, Some(at)) => Some(at),
            (true, None) => Some(self.clock.now()),
        };
        Ok((position, completed_at))
    }

    pub fn update(&self, id: Id, input: TaskInput) -> CoreResult<Task> {
        let current = self.get(id)?;
        let Resolved {
            input: t,
            status_id,
            done,
        } = self.resolve(input)?;
        let (position, completed_at) = self.placement(&current, status_id, done)?;
        self.conn.execute(
            "UPDATE tasks SET title = :title, description_md = :description, status_id = :status,
                 priority = :priority, project_id = :project, client_id = :client,
                 due_date = :due_date, due_time = :due_time, estimate_minutes = :estimate,
                 board_position = :position, completed_at = :completed, updated_at = :now
             WHERE id = :id",
            named_params! {
                ":id": id,
                ":title": t.title,
                ":description": t.description_md,
                ":status": status_id,
                ":priority": t.priority,
                ":project": t.project_id,
                ":client": t.client_id,
                ":due_date": t.due_date,
                ":due_time": t.due_time,
                ":estimate": t.estimate_minutes,
                ":position": position,
                ":completed": completed_at,
                ":now": self.clock.now(),
            },
        )?;
        self.get(id)
    }

    /// Moves the task to another column (at its end); entering a `done`
    /// column completes it, leaving one reopens it.
    pub fn set_status(&self, id: Id, status_id: Id) -> CoreResult<Task> {
        let current = self.get(id)?;
        let status = statuses::get(self.conn, status_id)?;
        let (position, completed_at) =
            self.placement(&current, status.id, status.category == StatusCategory::Done)?;
        self.conn.execute(
            "UPDATE tasks SET status_id = ?2, board_position = ?3, completed_at = ?4,
                 updated_at = ?5
             WHERE id = ?1",
            (id, status.id, position, completed_at, self.clock.now()),
        )?;
        self.get(id)
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        match self.conn.execute("DELETE FROM tasks WHERE id = ?1", [id])? {
            0 => Err(not_found(id)),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};
    use crate::statuses::TaskStatus;

    const T0: i64 = 1_700_000_000_000;

    struct Fixture {
        db: Db,
        clock: FakeClock,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                db: Db::open_in_memory().unwrap(),
                clock: FakeClock::at(Timestamp::from_millis(T0)),
            }
        }

        fn repo(&self) -> TaskRepo<'_> {
            TaskRepo::new(self.db.conn(), &self.clock)
        }

        fn columns(&self) -> Vec<TaskStatus> {
            statuses::list(self.db.conn()).unwrap()
        }

        fn client(&self, name: &str) -> Id {
            let input = ClientInput {
                kind: ClientKind::Company,
                last_name: None,
                first_name: None,
                middle_name: None,
                company_name: Some(name.into()),
                legal_name: None,
                status: ClientStatus::Active,
                default_currency: None,
                default_rate_minor: None,
            };
            ClientRepo::new(self.db.conn(), &self.clock)
                .create(input)
                .unwrap()
                .id
        }

        fn project(&self, client: Id) -> Id {
            let input = ProjectInput {
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
            };
            ProjectRepo::new(self.db.conn(), &self.clock)
                .create(input)
                .unwrap()
                .id
        }
    }

    fn input(title: &str) -> TaskInput {
        TaskInput {
            title: title.into(),
            description_md: String::new(),
            status_id: None,
            priority: Priority::Normal,
            project_id: None,
            client_id: None,
            due_date: None,
            due_time: None,
            estimate_minutes: None,
        }
    }

    #[test]
    fn creates_an_inbox_task_in_the_first_todo_column() {
        let f = Fixture::new();
        let t = f.repo().create(input("  Позвонить  ")).unwrap();
        assert_eq!(t.title, "Позвонить");
        assert_eq!((t.project_id, t.client_id), (None, None));
        assert_eq!(t.status_id, f.columns()[0].id);
        assert_eq!(t.completed_at, None);
        assert_eq!(t.created_at, Timestamp::from_millis(T0));
        assert_eq!(f.repo().get(t.id).unwrap(), t);
        assert_eq!(f.repo().get(Id::new()).unwrap_err().code, "task.not_found");
    }

    #[test]
    fn creates_a_task_for_a_client_only() {
        let f = Fixture::new();
        let client = f.client("Ромашка");
        let t = f
            .repo()
            .create(TaskInput {
                client_id: Some(client),
                ..input("Счёт")
            })
            .unwrap();
        assert_eq!(t.client_id, Some(client));
        assert_eq!(t.client_name.as_deref(), Some("Ромашка"));
        assert_eq!(t.project_id, None);
    }

    #[test]
    fn a_project_task_takes_the_client_from_the_project() {
        let f = Fixture::new();
        let client = f.client("Ромашка");
        let project = f.project(client);
        let t = f
            .repo()
            .create(TaskInput {
                project_id: Some(project),
                ..input("Макет")
            })
            .unwrap();
        assert_eq!(t.client_id, Some(client));
        assert_eq!(t.project_name.as_deref(), Some("Сайт"));

        // Naming the same client is fine; another one is a contradiction.
        assert!(
            f.repo()
                .create(TaskInput {
                    project_id: Some(project),
                    client_id: Some(client),
                    ..input("Вёрстка")
                })
                .is_ok()
        );
        let other = f.client("Василёк");
        assert_eq!(
            f.repo()
                .create(TaskInput {
                    project_id: Some(project),
                    client_id: Some(other),
                    ..input("Вёрстка")
                })
                .unwrap_err(),
            CoreError::new("task.client_mismatch")
        );
    }

    #[test]
    fn unknown_links_are_errors() {
        let f = Fixture::new();
        let missing = Id::new();
        let with = |patch: fn(&mut TaskInput, Id)| {
            let mut data = input("X");
            patch(&mut data, missing);
            f.repo().create(data).unwrap_err()
        };
        assert_eq!(
            with(|t, id| t.project_id = Some(id)),
            CoreError::not_found("project", missing)
        );
        assert_eq!(
            with(|t, id| t.client_id = Some(id)),
            CoreError::not_found("client", missing)
        );
        assert_eq!(
            with(|t, id| t.status_id = Some(id)),
            CoreError::not_found("status", missing)
        );
    }

    #[test]
    fn fields_are_validated() {
        let f = Fixture::new();
        let err = |data: TaskInput| f.repo().create(data).unwrap_err();
        assert_eq!(err(input(" ")), CoreError::validation("title", "required"));
        assert_eq!(
            err(TaskInput {
                due_date: Some("2024-02-30".into()),
                ..input("X")
            }),
            CoreError::validation("dueDate", "date")
        );
        assert_eq!(
            err(TaskInput {
                due_date: Some("2024-02-28".into()),
                due_time: Some("25:00".into()),
                ..input("X")
            }),
            CoreError::validation("dueTime", "time")
        );
        assert_eq!(
            err(TaskInput {
                due_time: Some("10:00".into()),
                ..input("X")
            }),
            CoreError::validation("dueTime", "needs_date")
        );
        let ok = f
            .repo()
            .create(TaskInput {
                due_date: Some("2024-02-28".into()),
                due_time: Some("09:30".into()),
                estimate_minutes: Some(90),
                priority: Priority::Urgent,
                ..input("X")
            })
            .unwrap();
        assert_eq!(ok.due_time.as_deref(), Some("09:30"));
        assert_eq!(ok.estimate_minutes, Some(90));
    }

    #[test]
    fn new_tasks_go_to_the_end_of_their_column() {
        let f = Fixture::new();
        let a = f.repo().create(input("A")).unwrap();
        let b = f.repo().create(input("B")).unwrap();
        assert!(a.board_position < b.board_position);
    }

    #[test]
    fn done_columns_complete_and_others_reopen() {
        let f = Fixture::new();
        let columns = f.columns();
        let (todo, review, done) = (columns[0].id, columns[2].id, columns[3].id);
        let t = f.repo().create(input("Сдать")).unwrap();

        f.clock.advance_millis(1_000);
        let finished = f.repo().set_status(t.id, done).unwrap();
        assert_eq!(finished.status_id, done);
        assert_eq!(
            finished.completed_at,
            Some(Timestamp::from_millis(T0 + 1_000))
        );

        f.clock.advance_millis(1_000);
        let reopened = f.repo().set_status(t.id, review).unwrap();
        assert_eq!(reopened.completed_at, None);

        // Created straight into a done column, a task is completed at once.
        let t = f
            .repo()
            .create(TaskInput {
                status_id: Some(done),
                ..input("Готово")
            })
            .unwrap();
        assert!(t.completed_at.is_some());
        // Updating keeps the completion time while the column stays done.
        let kept = f
            .repo()
            .update(
                t.id,
                TaskInput {
                    status_id: Some(done),
                    ..input("Готово!")
                },
            )
            .unwrap();
        assert_eq!(kept.completed_at, t.completed_at);
        let moved = f
            .repo()
            .update(
                t.id,
                TaskInput {
                    status_id: Some(todo),
                    ..input("Готово!")
                },
            )
            .unwrap();
        assert_eq!(moved.completed_at, None);
        assert_eq!(
            f.repo().set_status(t.id, Id::new()).unwrap_err().code,
            "status.not_found"
        );
    }

    #[test]
    fn update_changes_fields() {
        let f = Fixture::new();
        let client = f.client("Ромашка");
        let t = f.repo().create(input("Черновик")).unwrap();
        f.clock.advance_millis(500);
        let updated = f
            .repo()
            .update(
                t.id,
                TaskInput {
                    client_id: Some(client),
                    description_md: "- пункт".into(),
                    ..input("Чистовик")
                },
            )
            .unwrap();
        assert_eq!(updated.title, "Чистовик");
        assert_eq!(updated.client_id, Some(client));
        assert_eq!(updated.status_id, t.status_id);
        assert_eq!(updated.board_position, t.board_position);
        assert_eq!(updated.updated_at, Timestamp::from_millis(T0 + 500));
        assert_eq!(
            f.repo().update(Id::new(), input("X")).unwrap_err().code,
            "task.not_found"
        );
    }

    #[test]
    fn delete_and_cascades() {
        let f = Fixture::new();
        let t = f.repo().create(input("X")).unwrap();
        f.repo().delete(t.id).unwrap();
        assert_eq!(f.repo().delete(t.id).unwrap_err().code, "task.not_found");

        let client = f.client("Ромашка");
        let project = f.project(client);
        let in_project = f
            .repo()
            .create(TaskInput {
                project_id: Some(project),
                ..input("P")
            })
            .unwrap();
        let for_client = f
            .repo()
            .create(TaskInput {
                client_id: Some(client),
                ..input("C")
            })
            .unwrap();
        ProjectRepo::new(f.db.conn(), &f.clock)
            .delete(project)
            .unwrap();
        assert_eq!(
            f.repo().get(in_project.id).unwrap_err().code,
            "task.not_found"
        );
        assert!(f.repo().get(for_client.id).is_ok());
        let clients = ClientRepo::new(f.db.conn(), &f.clock);
        assert_eq!(clients.delete_impact(client).unwrap().tasks, 1);
        clients.delete(client).unwrap();
        assert_eq!(
            f.repo().get(for_client.id).unwrap_err().code,
            "task.not_found"
        );
    }
}
