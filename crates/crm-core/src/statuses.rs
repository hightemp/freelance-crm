//! Kanban columns (task statuses). Each column has a category that drives
//! task logic: `done` sets `completed_at`, overdue filters skip `done`.
//! Default columns have a `name_key` and are named by the UI in the user's
//! language until renamed (FR-TS-8).

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::db::text_enum;
use crate::domain::Id;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

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
