//! Entity identifiers: time-ordered UUIDv7 stored as TEXT.

use std::fmt;
use std::str::FromStr;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id(Uuid);

impl Id {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.hyphenated().fmt(f)
    }
}

impl FromStr for Id {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

impl ToSql for Id {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

impl FromSql for Id {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ids_are_v7_and_time_ordered() {
        let a = Id::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = Id::new();
        assert_eq!(a.0.get_version_num(), 7);
        assert!(a < b);
        assert!(a.to_string() < b.to_string());
    }

    #[test]
    fn string_round_trip() {
        let id = Id::new();
        assert_eq!(id.to_string().parse::<Id>().unwrap(), id);
        assert!("not-an-id".parse::<Id>().is_err());
    }

    #[test]
    fn sqlite_round_trip() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let id = Id::new();
        let back: Id = conn.query_row("SELECT ?1", [id], |r| r.get(0)).unwrap();
        assert_eq!(back, id);
        let bad: rusqlite::Result<Id> = conn.query_row("SELECT 'zzz'", [], |r| r.get(0));
        assert!(bad.is_err());
    }

    #[test]
    fn serializes_as_plain_string() {
        let id = Id::new();
        assert_eq!(serde_json::to_string(&id).unwrap(), format!("\"{id}\""));
    }
}
