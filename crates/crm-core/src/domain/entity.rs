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
}

impl EntityRef {
    /// `owner_type` value and id as stored in polymorphic tables.
    pub fn parts(self) -> (&'static str, Id) {
        match self {
            Self::Client(id) => ("client", id),
            Self::Contact(id) => ("contact", id),
            Self::Project(id) => ("project", id),
        }
    }

    /// Fails with `<type>.not_found` if the entity does not exist.
    pub fn ensure_exists(self, conn: &Connection) -> CoreResult<()> {
        match self {
            Self::Client(id) => ensure_exists(conn, "clients", "client", id),
            Self::Contact(id) => ensure_exists(conn, "contacts", "contact", id),
            Self::Project(id) => ensure_exists(conn, "projects", "project", id),
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
    fn missing_entity_is_not_found() {
        let db = Db::open_in_memory().unwrap();
        let id = Id::new();
        assert_eq!(
            EntityRef::Client(id).ensure_exists(db.conn()).unwrap_err(),
            CoreError::not_found("client", id)
        );
    }
}
