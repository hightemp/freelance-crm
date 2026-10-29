//! Checklists inside tasks: items with a check mark, ordered by the user
//! (FR-TS-3).

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::db::ensure_exists;
use crate::domain::{Clock, Id};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    pub id: Id,
    pub text: String,
    pub done: bool,
}

/// Percent of done items, rounded down; `None` for an empty checklist.
pub fn progress(done: u32, total: u32) -> Option<u8> {
    (total > 0).then(|| (done.min(total) * 100 / total) as u8)
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<ChecklistItem> {
    Ok(ChecklistItem {
        id: row.get(0)?,
        text: row.get(1)?,
        done: row.get(2)?,
    })
}

fn not_found(id: Id) -> CoreError {
    CoreError::not_found("checklist_item", id)
}

fn text(value: &str) -> CoreResult<String> {
    let text = value.trim();
    if text.is_empty() {
        return Err(CoreError::validation("text", "required"));
    }
    Ok(text.to_owned())
}

pub struct ChecklistRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> ChecklistRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    pub fn list(&self, task_id: Id) -> CoreResult<Vec<ChecklistItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, done FROM checklist_items WHERE task_id = ?1 ORDER BY position, id",
        )?;
        let rows = stmt.query_map([task_id], from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    fn get(&self, id: Id) -> CoreResult<ChecklistItem> {
        self.conn
            .query_row(
                "SELECT id, text, done FROM checklist_items WHERE id = ?1",
                [id],
                from_row,
            )
            .optional()?
            .ok_or_else(|| not_found(id))
    }

    /// Adds an item at the end.
    pub fn add(&self, task_id: Id, value: &str) -> CoreResult<ChecklistItem> {
        let text = text(value)?;
        ensure_exists(self.conn, "tasks", "task", task_id)?;
        let id = Id::new();
        self.conn.execute(
            "INSERT INTO checklist_items (id, task_id, text, position, created_at, updated_at)
             SELECT ?1, ?2, ?3, coalesce(max(position) + 1, 0), ?4, ?4
             FROM checklist_items WHERE task_id = ?2",
            (id, task_id, text, self.clock.now()),
        )?;
        self.get(id)
    }

    pub fn update(&self, id: Id, value: &str, done: bool) -> CoreResult<ChecklistItem> {
        let text = text(value)?;
        let changed = self.conn.execute(
            "UPDATE checklist_items SET text = ?2, done = ?3, updated_at = ?4 WHERE id = ?1",
            (id, text, done, self.clock.now()),
        )?;
        if changed == 0 {
            return Err(not_found(id));
        }
        self.get(id)
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        match self
            .conn
            .execute("DELETE FROM checklist_items WHERE id = ?1", [id])?
        {
            0 => Err(not_found(id)),
            _ => Ok(()),
        }
    }

    /// `ids` must be exactly the task's items, in the new order.
    pub fn reorder(&self, task_id: Id, ids: &[Id]) -> CoreResult<Vec<ChecklistItem>> {
        let mut current: Vec<Id> = self.list(task_id)?.into_iter().map(|i| i.id).collect();
        let mut requested = ids.to_vec();
        current.sort();
        requested.sort();
        if current != requested {
            return Err(CoreError::validation("ids", "order"));
        }
        let tx = self.conn.unchecked_transaction()?;
        for (position, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE checklist_items SET position = ?2 WHERE id = ?1",
                (id, position as i64),
            )?;
        }
        tx.commit()?;
        self.list(task_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::domain::{FakeClock, Timestamp};
    use crate::tasks::{Priority, TaskInput, TaskRepo};

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

        fn repo(&self) -> ChecklistRepo<'_> {
            ChecklistRepo::new(self.db.conn(), &self.clock)
        }

        fn tasks(&self) -> TaskRepo<'_> {
            TaskRepo::new(self.db.conn(), &self.clock)
        }

        fn task(&self) -> Id {
            self.tasks()
                .create(TaskInput {
                    title: "T".into(),
                    description_md: String::new(),
                    status_id: None,
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

    fn texts(items: &[ChecklistItem]) -> Vec<&str> {
        items.iter().map(|i| i.text.as_str()).collect()
    }

    #[test]
    fn progress_is_rounded_down_and_empty_is_none() {
        assert_eq!(progress(0, 0), None);
        assert_eq!(progress(0, 3), Some(0));
        assert_eq!(progress(1, 3), Some(33));
        assert_eq!(progress(2, 3), Some(66));
        assert_eq!(progress(3, 3), Some(100));
    }

    #[test]
    fn crud() {
        let f = Fixture::new();
        let task = f.task();
        let a = f.repo().add(task, "  Макет  ").unwrap();
        let b = f.repo().add(task, "Вёрстка").unwrap();
        assert_eq!((a.text.as_str(), a.done), ("Макет", false));
        assert_eq!(texts(&f.repo().list(task).unwrap()), ["Макет", "Вёрстка"]);

        let checked = f.repo().update(a.id, "Макет главной", true).unwrap();
        assert_eq!(
            (checked.text.as_str(), checked.done),
            ("Макет главной", true)
        );
        f.repo().delete(b.id).unwrap();
        assert_eq!(texts(&f.repo().list(task).unwrap()), ["Макет главной"]);

        assert_eq!(
            f.repo().add(task, " ").unwrap_err(),
            CoreError::validation("text", "required")
        );
        assert_eq!(
            f.repo().update(a.id, "", false).unwrap_err(),
            CoreError::validation("text", "required")
        );
        let missing = Id::new();
        assert_eq!(
            f.repo().add(missing, "x").unwrap_err(),
            CoreError::not_found("task", missing)
        );
        assert_eq!(
            f.repo().update(missing, "x", true).unwrap_err(),
            CoreError::not_found("checklist_item", missing)
        );
        assert_eq!(
            f.repo().delete(b.id).unwrap_err().code,
            "checklist_item.not_found"
        );
    }

    #[test]
    fn reorder_keeps_positions() {
        let f = Fixture::new();
        let task = f.task();
        let ids: Vec<Id> = ["A", "B", "C"]
            .iter()
            .map(|t| f.repo().add(task, t).unwrap().id)
            .collect();
        let reordered = f.repo().reorder(task, &[ids[2], ids[0], ids[1]]).unwrap();
        assert_eq!(texts(&reordered), ["C", "A", "B"]);
        f.repo().add(task, "D").unwrap();
        assert_eq!(texts(&f.repo().list(task).unwrap()), ["C", "A", "B", "D"]);
        assert_eq!(
            f.repo().reorder(task, &ids[..2]).unwrap_err(),
            CoreError::validation("ids", "order")
        );
    }

    #[test]
    fn tasks_report_their_progress_and_cascade() {
        let f = Fixture::new();
        let task = f.task();
        let a = f.repo().add(task, "A").unwrap();
        f.repo().add(task, "B").unwrap();
        f.repo().add(task, "C").unwrap();
        f.repo().update(a.id, "A", true).unwrap();
        let t = f.tasks().get(task).unwrap();
        assert_eq!((t.checklist_done, t.checklist_total), (1, 3));

        f.tasks().delete(task).unwrap();
        let left: i64 =
            f.db.conn()
                .query_row("SELECT count(*) FROM checklist_items", [], |r| r.get(0))
                .unwrap();
        assert_eq!(left, 0);
    }
}
