//! Tasks: in a project, only for a client, or in the inbox (FR-TS-1, 2).
//! Deadlines are local wall-clock dates (and optional times), so a task
//! due on Friday stays due on Friday when the time zone changes.

use rusqlite::{Connection, OptionalExtension, Row, named_params};
use serde::{Deserialize, Serialize};

use crate::db::{id_list, text_enum};
use crate::domain::date::is_iso_date;
use crate::domain::position::key_between;
use crate::domain::text::{contains_pattern, fold};
use jiff::tz::TimeZone;

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
    /// Attached tags, ordered by tag name.
    pub tag_ids: Vec<Id>,
    pub completed_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

const SELECT: &str = "SELECT t.id, t.title, t.description_md, t.status_id, t.priority,
    t.project_id, p.name AS project_name, t.client_id, c.display_name AS client_name,
    t.due_date, t.due_time, t.estimate_minutes, t.board_position, t.completed_at,
    t.created_at, t.updated_at,
    (SELECT json_group_array(g.tag_id ORDER BY tg.name_key)
     FROM taggings g JOIN tags tg ON tg.id = g.tag_id
     WHERE g.owner_type = 'task' AND g.owner_id = t.id) AS tag_ids
    FROM tasks t
    JOIN task_statuses s ON s.id = t.status_id
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
        tag_ids: id_list(row, "tag_ids")?,
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

/// Deadline presets, computed in the user's time zone. Tasks in `done`
/// columns never match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum DuePreset {
    /// Past the date, or past the time on today's date.
    Overdue,
    /// Due today.
    Today,
    /// Due today or within the next six days.
    Week,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum TaskSort {
    /// Column order, then position on the board.
    #[default]
    Board,
    /// Most urgent first, then by deadline.
    Priority,
    /// Earliest deadline first; tasks without one last.
    Due,
    /// Newest first.
    Created,
}

/// Where a task belongs; `Inbox` means no client and no project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "id", rename_all = "lowercase")]
pub enum TaskScope {
    Inbox,
    Client(Id),
    Project(Id),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaskFilter {
    /// Case-insensitive substring of the title.
    pub query: Option<String>,
    /// Any of these columns; empty means all.
    pub status_ids: Vec<Id>,
    pub priority: Option<Priority>,
    pub scope: Option<TaskScope>,
    /// All of these tags.
    pub tag_ids: Vec<Id>,
    pub due: Option<DuePreset>,
    /// Hide tasks in `done` columns.
    pub hide_done: bool,
    pub sort: TaskSort,
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
                 created_at, updated_at, title_key)
             VALUES (:id, :title, :description, :status, :priority, :project, :client, :due_date,
                 :due_time, :estimate, :position, :completed, :now, :now, :title_key)",
            named_params! {
                ":id": id,
                ":title_key": fold(&t.title),
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

    /// Completion time in a column: kept while staying done, set when
    /// entering done, cleared otherwise.
    fn completion(&self, current: &Task, done: bool) -> Option<Timestamp> {
        match (done, current.completed_at) {
            (false, _) => None,
            (true, Some(at)) => Some(at),
            (true, None) => Some(self.clock.now()),
        }
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
        Ok((position, self.completion(current, done)))
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
                 board_position = :position, completed_at = :completed, updated_at = :now,
                 title_key = :title_key
             WHERE id = :id",
            named_params! {
                ":id": id,
                ":title_key": fold(&t.title),
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

    /// Tasks matching the filter; presets use `tz` to find "today".
    pub fn list(&self, filter: &TaskFilter, tz: &TimeZone) -> CoreResult<Vec<Task>> {
        let now = jiff::Timestamp::from_millisecond(self.clock.now().as_millis())
            .map_err(|_| CoreError::new("internal.clock"))?
            .to_zoned(tz.clone());
        let today = now.date();
        let week_end = today
            .checked_add(jiff::Span::new().days(6))
            .map_err(|_| CoreError::new("internal.clock"))?;
        let ids = |ids: &[Id]| serde_json::to_string(ids).expect("ids serialize");
        let (scope_type, scope_id) = match filter.scope {
            None => (None, None),
            Some(TaskScope::Inbox) => (Some("inbox"), None),
            Some(TaskScope::Client(id)) => (Some("client"), Some(id)),
            Some(TaskScope::Project(id)) => (Some("project"), Some(id)),
        };
        const RANK: &str = "CASE t.priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 \
            WHEN 'normal' THEN 2 ELSE 3 END";
        let order = match filter.sort {
            TaskSort::Board => "s.position, t.board_position".to_owned(),
            TaskSort::Priority => {
                format!("{RANK}, t.due_date IS NULL, t.due_date, t.due_time IS NULL, t.due_time")
            }
            TaskSort::Due => {
                format!("t.due_date IS NULL, t.due_date, t.due_time IS NULL, t.due_time, {RANK}")
            }
            TaskSort::Created => "t.created_at DESC".to_owned(),
        };
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT}
               WHERE (:query IS NULL OR t.title_key LIKE :query ESCAPE '\\')
                 AND (json_array_length(:statuses) = 0
                      OR t.status_id IN (SELECT value FROM json_each(:statuses)))
                 AND (:priority IS NULL OR t.priority = :priority)
                 AND (:scope IS NULL
                      OR (:scope = 'inbox' AND t.client_id IS NULL)
                      OR (:scope = 'client' AND t.client_id = :scope_id)
                      OR (:scope = 'project' AND t.project_id = :scope_id))
                 AND (SELECT count(*) FROM taggings g
                      WHERE g.owner_type = 'task' AND g.owner_id = t.id
                        AND g.tag_id IN (SELECT value FROM json_each(:tags)))
                     = json_array_length(:tags)
                 AND (NOT :hide_done OR s.category != 'done')
                 AND (:due IS NULL OR (s.category != 'done' AND (
                      (:due = 'overdue' AND (t.due_date < :today
                          OR (t.due_date = :today AND t.due_time < :time)))
                      OR (:due = 'today' AND t.due_date = :today)
                      OR (:due = 'week' AND t.due_date BETWEEN :today AND :week_end))))
             ORDER BY {order}, t.created_at, t.id"
        ))?;
        let mut tag_ids = filter.tag_ids.clone();
        tag_ids.sort();
        tag_ids.dedup();
        let query = filter
            .query
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
            .map(contains_pattern);
        let rows = stmt.query_map(
            named_params! {
                ":query": query,
                ":statuses": ids(&filter.status_ids),
                ":priority": filter.priority,
                ":scope": scope_type,
                ":scope_id": scope_id,
                ":tags": ids(&tag_ids),
                ":hide_done": filter.hide_done,
                ":due": filter.due.map(|d| match d {
                    DuePreset::Overdue => "overdue",
                    DuePreset::Today => "today",
                    DuePreset::Week => "week",
                }),
                ":today": today.to_string(),
                ":week_end": week_end.to_string(),
                ":time": now.strftime("%H:%M").to_string(),
            },
            from_row,
        )?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Drops the task into `status_id` between two tasks of that column:
    /// `after` is the task above it, `before` the one below (`None` at the
    /// edges). Without either, it goes to the end of the column.
    pub fn move_task(
        &self,
        id: Id,
        status_id: Id,
        after: Option<Id>,
        before: Option<Id>,
    ) -> CoreResult<Task> {
        let current = self.get(id)?;
        let status = statuses::get(self.conn, status_id)?;
        let stale = || CoreError::new("task.stale_position");
        // A neighbour must be another task of the target column.
        let neighbour = |other: Option<Id>| -> CoreResult<Option<String>> {
            let Some(other) = other else { return Ok(None) };
            if other == id {
                return Err(stale());
            }
            let row: Option<(Id, String)> = self
                .conn
                .query_row(
                    "SELECT status_id, board_position FROM tasks WHERE id = ?1",
                    [other],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            match row {
                Some((column, position)) if column == status_id => Ok(Some(position)),
                _ => Err(stale()),
            }
        };
        let above = neighbour(after)?;
        let below = neighbour(before)?;
        let position = match (&above, &below) {
            (None, None) => self.end_of(status_id)?,
            (Some(a), Some(b)) if a >= b => return Err(stale()),
            _ => key_between(above.as_deref(), below.as_deref()),
        };
        let completed_at = self.completion(&current, status.category == StatusCategory::Done);
        self.conn.execute(
            "UPDATE tasks SET status_id = ?2, board_position = ?3, completed_at = ?4,
                 updated_at = ?5
             WHERE id = ?1",
            (id, status_id, position, completed_at, self.clock.now()),
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

    /// Unix ms of a wall-clock time in a zone.
    fn at(zone: &str, civil: &str) -> i64 {
        let tz = TimeZone::get(zone).unwrap();
        let dt: jiff::civil::DateTime = civil.parse().unwrap();
        dt.to_zoned(tz).unwrap().timestamp().as_millisecond()
    }

    fn due(f: &Fixture, title: &str, date: &str, time: Option<&str>) -> Task {
        f.repo()
            .create(TaskInput {
                due_date: Some(date.into()),
                due_time: time.map(Into::into),
                ..input(title)
            })
            .unwrap()
    }

    fn titles(f: &Fixture, filter: TaskFilter, zone: &str) -> Vec<String> {
        let tz = TimeZone::get(zone).unwrap();
        let mut titles: Vec<String> = f
            .repo()
            .list(&filter, &tz)
            .unwrap()
            .into_iter()
            .map(|t| t.title)
            .collect();
        if filter.sort == TaskSort::Board {
            titles.sort();
        }
        titles
    }

    fn preset(due: DuePreset) -> TaskFilter {
        TaskFilter {
            due: Some(due),
            ..TaskFilter::default()
        }
    }

    #[test]
    fn due_presets_use_the_local_date() {
        let f = Fixture::new();
        due(&f, "вчера", "2024-06-09", None);
        due(&f, "утром", "2024-06-10", Some("09:00"));
        due(&f, "вечером", "2024-06-10", Some("18:00"));
        due(&f, "сегодня", "2024-06-10", None);
        due(&f, "через 6 дней", "2024-06-16", None);
        due(&f, "через 7 дней", "2024-06-17", None);
        let done = due(&f, "сделано вчера", "2024-06-09", None);
        f.repo().set_status(done.id, f.columns()[3].id).unwrap();
        f.repo().create(input("без срока")).unwrap();

        // 12:00 in Moscow on 2024-06-10.
        f.clock
            .advance_millis(at("Europe/Moscow", "2024-06-10T12:00") - T0);
        let moscow = |p| titles(&f, preset(p), "Europe/Moscow");
        assert_eq!(moscow(DuePreset::Overdue), ["вчера", "утром"]);
        assert_eq!(moscow(DuePreset::Today), ["вечером", "сегодня", "утром"]);
        assert_eq!(
            moscow(DuePreset::Week),
            ["вечером", "сегодня", "утром", "через 6 дней"]
        );
    }

    #[test]
    fn the_day_boundary_follows_the_time_zone() {
        let f = Fixture::new();
        due(&f, "10-го", "2024-06-10", None);
        // 23:30 on the 9th in New York is already the 10th in Moscow.
        f.clock
            .advance_millis(at("America/New_York", "2024-06-09T23:30") - T0);
        assert!(titles(&f, preset(DuePreset::Today), "America/New_York").is_empty());
        assert_eq!(
            titles(&f, preset(DuePreset::Today), "Europe/Moscow"),
            ["10-го"]
        );
        // One minute after local midnight it is today in New York too.
        f.clock.advance_millis(31 * 60_000);
        assert_eq!(
            titles(&f, preset(DuePreset::Today), "America/New_York"),
            ["10-го"]
        );
    }

    #[test]
    fn presets_survive_dst_changes() {
        let f = Fixture::new();
        due(&f, "переход", "2024-03-10", Some("03:30"));
        due(&f, "после", "2024-03-11", None);
        // New York skips 02:00-03:00 on 2024-03-10: 01:59 EST is followed by 03:00 EDT.
        f.clock
            .advance_millis(at("America/New_York", "2024-03-10T01:59") - T0);
        let ny = |p| titles(&f, preset(p), "America/New_York");
        assert_eq!(ny(DuePreset::Today), ["переход"]);
        assert!(ny(DuePreset::Overdue).is_empty());
        // Two real minutes later it is 03:01 local time.
        f.clock.advance_millis(2 * 60_000);
        assert!(ny(DuePreset::Overdue).is_empty());
        f.clock.advance_millis(30 * 60_000);
        assert_eq!(ny(DuePreset::Overdue), ["переход"]);
        // 23:30 on the day clocks fall back in Europe is still that day.
        f.clock
            .advance_millis(at("Europe/Berlin", "2024-10-27T23:30") - f.clock.now().as_millis());
        assert!(titles(&f, preset(DuePreset::Today), "Europe/Berlin").is_empty());
    }

    #[test]
    fn filters_combine() {
        let f = Fixture::new();
        let client = f.client("Ромашка");
        let project = f.project(client);
        let mk =
            |title: &str, priority: Priority, project_id: Option<Id>, client_id: Option<Id>| {
                f.repo()
                    .create(TaskInput {
                        priority,
                        project_id,
                        client_id,
                        ..input(title)
                    })
                    .unwrap()
            };
        mk("Входящая", Priority::Normal, None, None);
        mk("Клиентская", Priority::High, None, Some(client));
        let urgent = mk("Срочная в проекте", Priority::Urgent, Some(project), None);
        mk("Обычная в проекте", Priority::Normal, Some(project), None);
        let doing = f.columns()[1].id;
        f.repo().set_status(urgent.id, doing).unwrap();

        let list = |filter: TaskFilter| titles(&f, filter, "UTC");
        assert_eq!(
            list(TaskFilter {
                scope: Some(TaskScope::Inbox),
                ..Default::default()
            }),
            ["Входящая"]
        );
        // A client's scope includes the tasks of its projects.
        assert_eq!(
            list(TaskFilter {
                scope: Some(TaskScope::Client(client)),
                ..Default::default()
            }),
            ["Клиентская", "Обычная в проекте", "Срочная в проекте"]
        );
        assert_eq!(
            list(TaskFilter {
                scope: Some(TaskScope::Project(project)),
                priority: Some(Priority::Urgent),
                ..Default::default()
            }),
            ["Срочная в проекте"]
        );
        assert_eq!(
            list(TaskFilter {
                status_ids: vec![doing],
                ..Default::default()
            }),
            ["Срочная в проекте"]
        );
        assert_eq!(
            list(TaskFilter {
                query: Some("ПРОЕКТЕ".into()),
                ..Default::default()
            }),
            ["Обычная в проекте", "Срочная в проекте"]
        );
    }

    #[test]
    fn hides_done_and_filters_by_tags() {
        use crate::tags::{TagInput, TagService};
        let f = Fixture::new();
        let a = f.repo().create(input("A")).unwrap();
        let b = f.repo().create(input("B")).unwrap();
        f.repo().set_status(b.id, f.columns()[3].id).unwrap();
        let tags = TagService::new(f.db.conn(), &f.clock);
        let tag = tags
            .create(TagInput {
                name: "дизайн".into(),
                color: "#000000".into(),
            })
            .unwrap();
        tags.set_tags(crate::domain::EntityRef::Task(a.id), &[tag.id])
            .unwrap();

        let list = |filter: TaskFilter| titles(&f, filter, "UTC");
        assert_eq!(
            list(TaskFilter {
                hide_done: true,
                ..Default::default()
            }),
            ["A"]
        );
        assert_eq!(
            list(TaskFilter {
                tag_ids: vec![tag.id],
                ..Default::default()
            }),
            ["A"]
        );
        assert_eq!(f.repo().get(a.id).unwrap().tag_ids, [tag.id]);
    }

    #[test]
    fn sorts_by_priority_due_and_creation() {
        let f = Fixture::new();
        let mk = |title: &str, priority: Priority, date: Option<&str>, time: Option<&str>| {
            f.clock.advance_millis(1_000);
            f.repo()
                .create(TaskInput {
                    priority,
                    due_date: date.map(Into::into),
                    due_time: time.map(Into::into),
                    ..input(title)
                })
                .unwrap();
        };
        mk("low-early", Priority::Low, Some("2024-01-01"), None);
        mk("urgent-late", Priority::Urgent, Some("2024-03-01"), None);
        mk("urgent-early", Priority::Urgent, Some("2024-02-01"), None);
        mk("normal-none", Priority::Normal, None, None);
        mk(
            "normal-timed",
            Priority::Normal,
            Some("2024-02-01"),
            Some("10:00"),
        );
        mk("normal-day", Priority::Normal, Some("2024-02-01"), None);

        let sorted = |sort| {
            titles(
                &f,
                TaskFilter {
                    sort,
                    ..Default::default()
                },
                "UTC",
            )
        };
        assert_eq!(
            sorted(TaskSort::Priority),
            [
                "urgent-early",
                "urgent-late",
                "normal-timed",
                "normal-day",
                "normal-none",
                "low-early"
            ]
        );
        // On the same day a timed deadline comes before all-day ones, which
        // are then ordered by priority.
        assert_eq!(
            sorted(TaskSort::Due),
            [
                "low-early",
                "normal-timed",
                "urgent-early",
                "normal-day",
                "urgent-late",
                "normal-none"
            ]
        );
        assert_eq!(
            sorted(TaskSort::Created),
            [
                "normal-day",
                "normal-timed",
                "normal-none",
                "urgent-early",
                "urgent-late",
                "low-early"
            ]
        );
    }

    fn column(f: &Fixture, status: Id) -> Vec<String> {
        let filter = TaskFilter {
            status_ids: vec![status],
            ..TaskFilter::default()
        };
        f.repo()
            .list(&filter, &TimeZone::UTC)
            .unwrap()
            .into_iter()
            .map(|t| t.title)
            .collect()
    }

    #[test]
    fn moves_between_neighbours_and_to_the_edges() {
        let f = Fixture::new();
        let todo = f.columns()[0].id;
        let ids: Vec<Id> = ["A", "B", "C"]
            .iter()
            .map(|t| f.repo().create(input(t)).unwrap().id)
            .collect();
        let (a, b, c) = (ids[0], ids[1], ids[2]);

        f.repo().move_task(c, todo, Some(a), Some(b)).unwrap();
        assert_eq!(column(&f, todo), ["A", "C", "B"]);
        f.repo().move_task(b, todo, None, Some(a)).unwrap();
        assert_eq!(column(&f, todo), ["B", "A", "C"]);
        f.repo().move_task(b, todo, Some(c), None).unwrap();
        assert_eq!(column(&f, todo), ["A", "C", "B"]);
        f.repo().move_task(a, todo, None, None).unwrap();
        assert_eq!(column(&f, todo), ["C", "B", "A"]);
    }

    #[test]
    fn moving_to_another_column_changes_the_status() {
        let f = Fixture::new();
        let columns = f.columns();
        let (todo, done) = (columns[0].id, columns[3].id);
        let a = f.repo().create(input("A")).unwrap().id;
        let x = f
            .repo()
            .create(TaskInput {
                status_id: Some(done),
                ..input("X")
            })
            .unwrap()
            .id;
        let moved = f.repo().move_task(a, done, None, Some(x)).unwrap();
        assert_eq!(moved.status_id, done);
        assert!(moved.completed_at.is_some());
        assert_eq!(column(&f, done), ["A", "X"]);
        assert!(column(&f, todo).is_empty());
        let back = f.repo().move_task(a, todo, None, None).unwrap();
        assert_eq!(back.completed_at, None);
    }

    #[test]
    fn neighbours_must_be_in_the_target_column_and_ordered() {
        let f = Fixture::new();
        let columns = f.columns();
        let a = f.repo().create(input("A")).unwrap().id;
        let b = f.repo().create(input("B")).unwrap().id;
        let elsewhere = f
            .repo()
            .create(TaskInput {
                status_id: Some(columns[1].id),
                ..input("E")
            })
            .unwrap()
            .id;
        let todo = columns[0].id;
        assert_eq!(
            f.repo()
                .move_task(a, todo, Some(elsewhere), None)
                .unwrap_err(),
            CoreError::new("task.stale_position")
        );
        // The board was out of date: "after" is below "before".
        assert_eq!(
            f.repo().move_task(a, todo, Some(b), Some(a)).unwrap_err(),
            CoreError::new("task.stale_position")
        );
        assert_eq!(
            f.repo()
                .move_task(Id::new(), todo, None, None)
                .unwrap_err()
                .code,
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
