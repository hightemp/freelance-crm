//! Kanban columns (task statuses). Each column has a category that drives
//! task logic: `done` sets `completed_at`, overdue filters skip `done`.
//! Default columns have a `name_key` and are named by the UI in the user's
//! language until renamed (FR-TS-8).

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::db::text_enum;
use std::collections::HashMap;

use crate::domain::text::{clean, is_hex_color};
use crate::domain::{Clock, Id};
use crate::tasks::TaskRepo;
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StatusCategory {
    Todo,
    InProgress,
    Done,
}

text_enum!(StatusCategory {
    Todo => "todo",
    InProgress => "in_progress",
    Done => "done",
});

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatus {
    pub id: Id,
    /// User-given name; `None` for an untouched default column.
    pub name: Option<String>,
    /// Message key of a default column (`todo`, `in_progress`, `review`, `done`).
    pub name_key: Option<String>,
    pub category: StatusCategory,
    /// `#rrggbb`.
    pub color: String,
    pub position: u32,
    pub wip_limit: Option<u32>,
}

const COLUMNS: &str = "id, name, name_key, category, color, position, wip_limit";

fn from_row(row: &Row<'_>) -> rusqlite::Result<TaskStatus> {
    Ok(TaskStatus {
        id: row.get(0)?,
        name: row.get(1)?,
        name_key: row.get(2)?,
        category: row.get(3)?,
        color: row.get(4)?,
        position: row.get(5)?,
        wip_limit: row.get(6)?,
    })
}

/// Columns in board order.
pub fn list(conn: &Connection) -> CoreResult<Vec<TaskStatus>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM task_statuses ORDER BY position, id"
    ))?;
    let rows = stmt.query_map([], from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn get(conn: &Connection, id: Id) -> CoreResult<TaskStatus> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM task_statuses WHERE id = ?1"),
        [id],
        from_row,
    )
    .optional()?
    .ok_or_else(|| CoreError::not_found("status", id))
}

/// The first column of the `todo` category: where new tasks go.
pub fn default_status(conn: &Connection) -> CoreResult<TaskStatus> {
    list(conn)?
        .into_iter()
        .find(|s| s.category == StatusCategory::Todo)
        .ok_or_else(|| CoreError::new("status.no_todo"))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StatusInput {
    /// `None` keeps a default column's localized name; required for new ones.
    pub name: Option<String>,
    pub category: StatusCategory,
    pub color: String,
    pub wip_limit: Option<u32>,
}

/// A column with its load, for the board header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BoardColumn {
    #[serde(flatten)]
    pub status: TaskStatus,
    pub task_count: u32,
    /// More tasks than the WIP limit: a warning, never an error.
    pub over_limit: bool,
}

pub struct StatusService<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> StatusService<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    pub fn columns(&self) -> CoreResult<Vec<BoardColumn>> {
        let counts: HashMap<Id, u32> = {
            let mut stmt = self
                .conn
                .prepare("SELECT status_id, count(*) FROM tasks GROUP BY status_id")?;
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<_, _>>()?
        };
        Ok(list(self.conn)?
            .into_iter()
            .map(|status| {
                let task_count = counts.get(&status.id).copied().unwrap_or(0);
                BoardColumn {
                    over_limit: status.wip_limit.is_some_and(|limit| task_count > limit),
                    task_count,
                    status,
                }
            })
            .collect())
    }

    fn validate(input: StatusInput) -> CoreResult<StatusInput> {
        let color = input.color.trim().to_ascii_lowercase();
        if !is_hex_color(&color) {
            return Err(CoreError::validation("color", "color"));
        }
        if input.wip_limit == Some(0) {
            return Err(CoreError::validation("wipLimit", "min"));
        }
        Ok(StatusInput {
            name: clean(input.name),
            color,
            ..input
        })
    }

    pub fn create(&self, input: StatusInput) -> CoreResult<TaskStatus> {
        let input = Self::validate(input)?;
        if input.name.is_none() {
            return Err(CoreError::validation("name", "required"));
        }
        let id = Id::new();
        self.conn.execute(
            "INSERT INTO task_statuses (id, name, category, color, position, wip_limit)
             SELECT ?1, ?2, ?3, ?4, coalesce(max(position) + 1, 0), ?5 FROM task_statuses",
            (
                id,
                &input.name,
                input.category,
                &input.color,
                input.wip_limit,
            ),
        )?;
        get(self.conn, id)
    }

    /// Fails if `current` is the last column of a category that must exist.
    fn ensure_not_last(&self, current: &TaskStatus) -> CoreResult<()> {
        if current.category == StatusCategory::InProgress {
            return Ok(());
        }
        let others: u32 = self.conn.query_row(
            "SELECT count(*) FROM task_statuses WHERE category = ?1 AND id != ?2",
            (current.category, current.id),
            |r| r.get(0),
        )?;
        if others == 0 {
            return Err(CoreError::new("status.last_of_category")
                .with("category", current.category.as_str()));
        }
        Ok(())
    }

    pub fn update(&self, id: Id, input: StatusInput) -> CoreResult<TaskStatus> {
        let input = Self::validate(input)?;
        let current = get(self.conn, id)?;
        if input.name.is_none() && current.name_key.is_none() {
            return Err(CoreError::validation("name", "required"));
        }
        if input.category != current.category {
            self.ensure_not_last(&current)?;
        }
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE task_statuses SET name = ?2, category = ?3, color = ?4, wip_limit = ?5
             WHERE id = ?1",
            (
                id,
                &input.name,
                input.category,
                &input.color,
                input.wip_limit,
            ),
        )?;
        // Tasks follow the column's category: done completes them.
        let was_done = current.category == StatusCategory::Done;
        let is_done = input.category == StatusCategory::Done;
        if is_done && !was_done {
            tx.execute(
                "UPDATE tasks SET completed_at = ?2 WHERE status_id = ?1 AND completed_at IS NULL",
                (id, self.clock.now()),
            )?;
        } else if was_done && !is_done {
            tx.execute(
                "UPDATE tasks SET completed_at = NULL WHERE status_id = ?1",
                [id],
            )?;
        }
        tx.commit()?;
        get(self.conn, id)
    }

    /// `ids` must be exactly all columns, in the new order.
    pub fn reorder(&self, ids: &[Id]) -> CoreResult<Vec<TaskStatus>> {
        let mut current: Vec<Id> = list(self.conn)?.into_iter().map(|s| s.id).collect();
        let mut requested = ids.to_vec();
        current.sort();
        requested.sort();
        if current != requested {
            return Err(CoreError::validation("ids", "order"));
        }
        let tx = self.conn.unchecked_transaction()?;
        for (position, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE task_statuses SET position = ?2 WHERE id = ?1",
                (id, position as i64),
            )?;
        }
        tx.commit()?;
        list(self.conn)
    }

    /// Deletes a column; its tasks must be moved to `move_to` if it has any.
    pub fn delete(&self, id: Id, move_to: Option<Id>) -> CoreResult<()> {
        let current = get(self.conn, id)?;
        self.ensure_not_last(&current)?;
        let task_ids: Vec<Id> = {
            let mut stmt = self
                .conn
                .prepare("SELECT id FROM tasks WHERE status_id = ?1 ORDER BY board_position")?;
            stmt.query_map([id], |r| r.get(0))?
                .collect::<Result<_, _>>()?
        };
        let tx = self.conn.unchecked_transaction()?;
        if !task_ids.is_empty() {
            let Some(target) = move_to else {
                return Err(CoreError::new("status.not_empty").with("count", task_ids.len()));
            };
            if target == id {
                return Err(CoreError::validation("moveTo", "same"));
            }
            get(self.conn, target)?;
            // Same rules as a manual move: end of the column, completion.
            let tasks = TaskRepo::new(&tx, self.clock);
            for task in task_ids {
                tasks.set_status(task, target)?;
            }
        }
        tx.execute("DELETE FROM task_statuses WHERE id = ?1", [id])?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::domain::{FakeClock, Timestamp};
    use crate::tasks::{Priority, TaskInput, TaskRepo};

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

        fn service(&self) -> StatusService<'_> {
            StatusService::new(self.db.conn(), &self.clock)
        }

        fn tasks(&self) -> TaskRepo<'_> {
            TaskRepo::new(self.db.conn(), &self.clock)
        }

        fn ids(&self) -> Vec<Id> {
            list(self.db.conn()).unwrap().iter().map(|s| s.id).collect()
        }

        fn task(&self, status: Id) -> Id {
            self.tasks()
                .create(TaskInput {
                    title: "T".into(),
                    description_md: String::new(),
                    status_id: Some(status),
                    priority: Priority::Normal,
                    project_id: None,
                    client_id: None,
                    due_date: None,
                    due_time: None,
                    estimate_minutes: None,
                })
                .unwrap()
                .id
        }
    }

    fn input(name: &str, category: StatusCategory) -> StatusInput {
        StatusInput {
            name: Some(name.into()),
            category,
            color: "#AABBCC".into(),
            wip_limit: None,
        }
    }

    #[test]
    fn creates_columns_at_the_end() {
        let f = Fixture::new();
        let s = f
            .service()
            .create(input(" Тестирование ", StatusCategory::InProgress))
            .unwrap();
        assert_eq!(s.name.as_deref(), Some("Тестирование"));
        assert_eq!(s.name_key, None);
        assert_eq!(s.color, "#aabbcc");
        assert_eq!(f.ids().last(), Some(&s.id));
        assert_eq!(
            f.service()
                .create(StatusInput {
                    name: None,
                    ..input("", StatusCategory::Todo)
                })
                .unwrap_err(),
            CoreError::validation("name", "required")
        );
        assert_eq!(
            f.service()
                .create(StatusInput {
                    color: "blue".into(),
                    ..input("X", StatusCategory::Todo)
                })
                .unwrap_err(),
            CoreError::validation("color", "color")
        );
        assert_eq!(
            f.service()
                .create(StatusInput {
                    wip_limit: Some(0),
                    ..input("X", StatusCategory::Todo)
                })
                .unwrap_err(),
            CoreError::validation("wipLimit", "min")
        );
    }

    #[test]
    fn renaming_replaces_the_localized_default_name() {
        let f = Fixture::new();
        let todo = list(f.db.conn()).unwrap()[0].clone();
        let kept = f
            .service()
            .update(
                todo.id,
                StatusInput {
                    name: None,
                    ..input("", StatusCategory::Todo)
                },
            )
            .unwrap();
        assert_eq!((kept.name, kept.name_key.as_deref()), (None, Some("todo")));
        let renamed = f
            .service()
            .update(todo.id, input("Бэклог", StatusCategory::Todo))
            .unwrap();
        assert_eq!(renamed.name.as_deref(), Some("Бэклог"));
        assert_eq!(
            f.service()
                .update(Id::new(), input("X", StatusCategory::Todo))
                .unwrap_err()
                .code,
            "status.not_found"
        );
    }

    #[test]
    fn changing_the_category_updates_completion_of_its_tasks() {
        let f = Fixture::new();
        let review = f.ids()[2];
        let task = f.task(review);
        f.service()
            .update(review, input("Принято", StatusCategory::Done))
            .unwrap();
        assert_eq!(
            f.tasks().get(task).unwrap().completed_at,
            Some(Timestamp::from_millis(T0))
        );
        f.service()
            .update(review, input("Проверка", StatusCategory::InProgress))
            .unwrap();
        assert_eq!(f.tasks().get(task).unwrap().completed_at, None);
    }

    #[test]
    fn reorder_keeps_the_order() {
        let f = Fixture::new();
        let ids = f.ids();
        let reversed: Vec<Id> = ids.iter().rev().copied().collect();
        let result = f.service().reorder(&reversed).unwrap();
        assert_eq!(result.iter().map(|s| s.id).collect::<Vec<_>>(), reversed);
        assert_eq!(f.ids(), reversed);
        assert_eq!(
            f.service().reorder(&ids[..3]).unwrap_err(),
            CoreError::validation("ids", "order")
        );
    }

    #[test]
    fn non_empty_column_needs_a_target_for_its_tasks() {
        let f = Fixture::new();
        let ids = f.ids();
        let task = f.task(ids[1]);
        assert_eq!(
            f.service().delete(ids[1], None).unwrap_err(),
            CoreError::new("status.not_empty").with("count", 1)
        );
        assert_eq!(
            f.service().delete(ids[1], Some(ids[1])).unwrap_err(),
            CoreError::validation("moveTo", "same")
        );
        assert_eq!(
            f.service()
                .delete(ids[1], Some(Id::new()))
                .unwrap_err()
                .code,
            "status.not_found"
        );
        f.clock.advance_millis(1_000);
        f.service().delete(ids[1], Some(ids[3])).unwrap();
        let moved = f.tasks().get(task).unwrap();
        assert_eq!(moved.status_id, ids[3]);
        // Moving into a done column completes the task.
        assert_eq!(moved.completed_at, Some(Timestamp::from_millis(T0 + 1_000)));
        assert_eq!(f.ids().len(), 3);
    }

    #[test]
    fn last_todo_or_done_column_cannot_be_deleted() {
        let f = Fixture::new();
        let ids = f.ids();
        assert_eq!(
            f.service().delete(ids[0], None).unwrap_err(),
            CoreError::new("status.last_of_category").with("category", "todo")
        );
        assert_eq!(
            f.service().delete(ids[3], None).unwrap_err(),
            CoreError::new("status.last_of_category").with("category", "done")
        );
        // Neither can the last one change its category.
        assert_eq!(
            f.service()
                .update(ids[3], input("X", StatusCategory::Todo))
                .unwrap_err(),
            CoreError::new("status.last_of_category").with("category", "done")
        );
        // In-progress columns may all go.
        f.service().delete(ids[1], None).unwrap();
        f.service().delete(ids[2], None).unwrap();
        // With a second todo column the first one can be deleted.
        f.service()
            .create(input("Входящие", StatusCategory::Todo))
            .unwrap();
        f.service().delete(ids[0], None).unwrap();
    }

    #[test]
    fn exceeding_the_wip_limit_is_a_warning() {
        let f = Fixture::new();
        let ids = f.ids();
        f.service()
            .update(
                ids[1],
                StatusInput {
                    name: None,
                    wip_limit: Some(1),
                    ..input("", StatusCategory::InProgress)
                },
            )
            .unwrap();
        f.task(ids[1]);
        let load = |f: &Fixture| f.service().columns().unwrap()[1].clone();
        assert_eq!((load(&f).task_count, load(&f).over_limit), (1, false));
        // A second task is accepted; the column is flagged.
        f.task(ids[1]);
        assert_eq!((load(&f).task_count, load(&f).over_limit), (2, true));
        assert_eq!(f.service().columns().unwrap()[0].task_count, 0);
    }

    #[test]
    fn four_default_columns_are_seeded() {
        let db = Db::open_in_memory().unwrap();
        let columns = list(db.conn()).unwrap();
        let summary: Vec<_> = columns
            .iter()
            .map(|s| (s.name_key.as_deref().unwrap(), s.category, s.name.is_none()))
            .collect();
        assert_eq!(
            summary,
            [
                ("todo", StatusCategory::Todo, true),
                ("in_progress", StatusCategory::InProgress, true),
                ("review", StatusCategory::InProgress, true),
                ("done", StatusCategory::Done, true),
            ]
        );
        assert_eq!(default_status(db.conn()).unwrap(), columns[0]);
        assert_eq!(get(db.conn(), columns[3].id).unwrap(), columns[3]);
        assert_eq!(
            get(db.conn(), Id::new()).unwrap_err().code,
            "status.not_found"
        );
    }
}
