//! SQLite connection setup and schema migrations.

mod migrations;

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

pub use migrations::{LATEST_VERSION, migrations};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Migration(#[from] rusqlite_migration::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type DbResult<T> = Result<T, DbError>;

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Owned SQLite connection with pragmas applied and the schema migrated
/// to the latest version.
pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> DbResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> DbResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> DbResult<Self> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(BUSY_TIMEOUT)?;
        migrations().to_latest(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Current schema version (`PRAGMA user_version`).
    pub fn schema_version(&self) -> DbResult<u32> {
        Ok(self
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pragma<T: rusqlite::types::FromSql>(db: &Db, name: &str) -> T {
        db.conn()
            .pragma_query_value(None, name, |row| row.get(0))
            .unwrap()
    }

    fn table_exists(db: &Db, table: &str) -> bool {
        db.conn()
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |_| Ok(()),
            )
            .is_ok()
    }

    #[test]
    fn migrations_are_valid() {
        migrations().validate().unwrap();
    }

    #[test]
    fn in_memory_db_is_migrated_to_latest() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), LATEST_VERSION);
        assert!(table_exists(&db, "settings"));
    }

    #[test]
    fn foreign_keys_are_enabled() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(pragma::<i64>(&db, "foreign_keys"), 1);
    }

    #[test]
    fn busy_timeout_is_set() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(
            pragma::<i64>(&db, "busy_timeout"),
            BUSY_TIMEOUT.as_millis() as i64
        );
    }

    #[test]
    fn file_db_uses_wal() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("crm.db")).unwrap();
        assert_eq!(pragma::<String>(&db, "journal_mode"), "wal");
    }

    #[test]
    fn reopening_is_idempotent_and_keeps_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("crm.db");
        {
            let db = Db::open(&path).unwrap();
            db.conn()
                .execute(
                    "INSERT INTO settings (key, value) VALUES ('k', '\"v\"')",
                    [],
                )
                .unwrap();
        }
        let db = Db::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), LATEST_VERSION);
        let value: String = db
            .conn()
            .query_row("SELECT value FROM settings WHERE key = 'k'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(value, "\"v\"");
    }

    #[test]
    fn open_creates_missing_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/deeper/crm.db");
        Db::open(&path).unwrap();
        assert!(path.exists());
    }
}
