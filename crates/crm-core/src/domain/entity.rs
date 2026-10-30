//! References to entities for polymorphic relations (contact methods, tags,
//! custom fields, notes). Each such table stores `owner_type` + `owner_id`;
//! cascades on owner deletion are done by triggers.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::CoreResult;
use crate::db::ensure_exists;
use crate::domain::Id;

/// Variants are added as entities appear (contacts, projects, tasks …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "id", rename_all = "lowercase")]
pub enum EntityRef {
    Client(Id),
    Contact(Id),
    Project(Id),
    Task(Id),
    Secret(Id),
    Event(Id),
    Interaction(Id),
    Expense(Id),
    Invoice(Id),
}

impl EntityRef {
    /// `owner_type` value and id as stored in polymorphic tables.
    pub fn parts(self) -> (&'static str, Id) {
        match self {
            Self::Client(id) => ("client", id),
            Self::Contact(id) => ("contact", id),
            Self::Project(id) => ("project", id),
            Self::Task(id) => ("task", id),
            Self::Secret(id) => ("secret", id),
            Self::Event(id) => ("event", id),
            Self::Interaction(id) => ("interaction", id),
            Self::Expense(id) => ("expense", id),
            Self::Invoice(id) => ("invoice", id),
        }
    }

    /// The reference stored as `owner_type` and id; `None` for an unknown type.
    pub fn from_parts(kind: &str, id: Id) -> Option<Self> {
        Some(match kind {
            "client" => Self::Client(id),
            "contact" => Self::Contact(id),
            "project" => Self::Project(id),
            "task" => Self::Task(id),
            "secret" => Self::Secret(id),
            "event" => Self::Event(id),
            "interaction" => Self::Interaction(id),
            "expense" => Self::Expense(id),
            "invoice" => Self::Invoice(id),
            _ => return None,
        })
    }

    /// Fails with `<type>.not_found` if the entity does not exist.
    pub fn ensure_exists(self, conn: &Connection) -> CoreResult<()> {
        match self {
            Self::Client(id) => ensure_exists(conn, "clients", "client", id),
            Self::Contact(id) => ensure_exists(conn, "contacts", "contact", id),
            Self::Project(id) => ensure_exists(conn, "projects", "project", id),
            Self::Task(id) => ensure_exists(conn, "tasks", "task", id),
            Self::Secret(id) => ensure_exists(conn, "secrets", "secret", id),
            Self::Event(id) => ensure_exists(conn, "events", "event", id),
            Self::Interaction(id) => ensure_exists(conn, "interactions", "interaction", id),
            Self::Expense(id) => ensure_exists(conn, "expenses", "expense", id),
            Self::Invoice(id) => ensure_exists(conn, "invoices", "invoice", id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CoreError;
    use crate::db::Db;

    #[test]
    fn serializes_as_tagged_object() {
        let id = Id::new();
        assert_eq!(
            serde_json::to_value(EntityRef::Client(id)).unwrap(),
            serde_json::json!({ "type": "client", "id": id.to_string() })
        );
    }

    #[test]
    fn round_trips_through_parts() {
        let id = Id::new();
        for r in [
            EntityRef::Client(id),
            EntityRef::Contact(id),
            EntityRef::Project(id),
            EntityRef::Task(id),
            EntityRef::Secret(id),
            EntityRef::Event(id),
            EntityRef::Interaction(id),
            EntityRef::Expense(id),
            EntityRef::Invoice(id),
        ] {
            let (kind, id) = r.parts();
            assert_eq!(EntityRef::from_parts(kind, id), Some(r));
        }
        assert_eq!(EntityRef::from_parts("planet", id), None);
    }

    #[test]
    fn missing_entity_is_not_found() {
        let db = Db::open_in_memory().unwrap();
        let id = Id::new();
        assert_eq!(
            EntityRef::Client(id).ensure_exists(db.conn()).unwrap_err(),
            CoreError::not_found("client", id)
        );
    }
}
