//! Markdown notes attached to any main entity; pinned notes come first.

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::domain::{Clock, EntityRef, Id, Timestamp};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NoteInput {
    pub body_md: String,
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: Id,
    pub body_md: String,
    pub pinned: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl NoteInput {
    /// The body is kept as typed (Markdown is whitespace-sensitive) but must
    /// not be blank.
    pub fn validate(&self) -> CoreResult<()> {
        if self.body_md.trim().is_empty() {
            return Err(CoreError::validation("bodyMd", "required"));
        }
        Ok(())
    }
}

const COLUMNS: &str = "id, body_md, pinned, created_at, updated_at";

fn from_row(row: &Row<'_>) -> rusqlite::Result<Note> {
    Ok(Note {
        id: row.get(0)?,
        body_md: row.get(1)?,
        pinned: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn not_found(id: Id) -> CoreError {
    CoreError::not_found("note", id)
}

pub struct NoteRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> NoteRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    /// Pinned notes first, then the most recently edited.
    pub fn list(&self, owner: EntityRef) -> CoreResult<Vec<Note>> {
        let (owner_type, owner_id) = owner.parts();
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM notes
             WHERE owner_type = ?1 AND owner_id = ?2
             ORDER BY pinned DESC, updated_at DESC, id DESC"
        ))?;
        let rows = stmt.query_map((owner_type, owner_id), from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    fn get(&self, id: Id) -> CoreResult<Note> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM notes WHERE id = ?1"),
                [id],
                from_row,
            )
            .optional()?
            .ok_or_else(|| not_found(id))
    }

    pub fn create(&self, owner: EntityRef, input: NoteInput) -> CoreResult<Note> {
        input.validate()?;
        owner.ensure_exists(self.conn)?;
        let (owner_type, owner_id) = owner.parts();
        let id = Id::new();
        self.conn.execute(
            "INSERT INTO notes (id, owner_type, owner_id, body_md, pinned, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            (
                id,
                owner_type,
                owner_id,
                &input.body_md,
                input.pinned,
                self.clock.now(),
            ),
        )?;
        self.get(id)
    }

    pub fn update(&self, id: Id, input: NoteInput) -> CoreResult<Note> {
        input.validate()?;
        let changed = self.conn.execute(
            "UPDATE notes SET body_md = ?2, pinned = ?3, updated_at = ?4 WHERE id = ?1",
            (id, &input.body_md, input.pinned, self.clock.now()),
        )?;
        if changed == 0 {
            return Err(not_found(id));
        }
        self.get(id)
    }

    /// Pinning is not an edit, so `updated_at` stays the same.
    pub fn set_pinned(&self, id: Id, pinned: bool) -> CoreResult<Note> {
        self.conn
            .execute("UPDATE notes SET pinned = ?2 WHERE id = ?1", (id, pinned))?;
        self.get(id)
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        match self.conn.execute("DELETE FROM notes WHERE id = ?1", [id])? {
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

        fn client(&self) -> EntityRef {
            let input = ClientInput {
                kind: ClientKind::Company,
                last_name: None,
                first_name: None,
                middle_name: None,
                company_name: Some("Ромашка".into()),
                legal_name: None,
                status: ClientStatus::Active,
                default_currency: None,
                default_rate_minor: None,
            };
            let client = ClientRepo::new(self.db.conn(), &self.clock)
                .create(input)
                .unwrap();
            EntityRef::Client(client.id)
        }

        fn repo(&self) -> NoteRepo<'_> {
            NoteRepo::new(self.db.conn(), &self.clock)
        }

        /// Creates a note one second after the previous action.
        fn note(&self, owner: EntityRef, body: &str, pinned: bool) -> Note {
            self.clock.advance_millis(1_000);
            self.repo().create(owner, input(body, pinned)).unwrap()
        }
    }

    fn input(body: &str, pinned: bool) -> NoteInput {
        NoteInput {
            body_md: body.into(),
            pinned,
        }
    }

    fn bodies(notes: &[Note]) -> Vec<&str> {
        notes.iter().map(|n| n.body_md.as_str()).collect()
    }

    #[test]
    fn create_keeps_markdown_and_sets_times() {
        let f = Fixture::new();
        let owner = f.client();
        let note = f
            .repo()
            .create(owner, input("# Заголовок\n\n- пункт\n", false))
            .unwrap();
        assert_eq!(note.body_md, "# Заголовок\n\n- пункт\n");
        assert!(!note.pinned);
        assert_eq!(note.created_at, Timestamp::from_millis(T0));
        assert_eq!(note.updated_at, Timestamp::from_millis(T0));
    }

    #[test]
    fn blank_note_is_invalid() {
        let f = Fixture::new();
        let owner = f.client();
        assert_eq!(
            f.repo().create(owner, input(" \n\t", false)).unwrap_err(),
            CoreError::validation("bodyMd", "required")
        );
        let note = f.note(owner, "x", false);
        assert!(f.repo().update(note.id, input("", false)).is_err());
    }

    #[test]
    fn create_checks_owner() {
        let f = Fixture::new();
        let missing = Id::new();
        assert_eq!(
            f.repo()
                .create(EntityRef::Client(missing), input("x", false))
                .unwrap_err(),
            CoreError::not_found("client", missing)
        );
    }

    #[test]
    fn pinned_first_then_recently_updated() {
        let f = Fixture::new();
        let owner = f.client();
        let old = f.note(owner, "old", false);
        f.note(owner, "pinned old", true);
        f.note(owner, "middle", false);
        f.note(owner, "pinned new", true);
        f.note(owner, "new", false);
        assert_eq!(
            bodies(&f.repo().list(owner).unwrap()),
            ["pinned new", "pinned old", "new", "middle", "old"]
        );

        // Editing moves a note up within its group.
        f.clock.advance_millis(1_000);
        f.repo().update(old.id, input("old edited", false)).unwrap();
        assert_eq!(
            bodies(&f.repo().list(owner).unwrap()),
            ["pinned new", "pinned old", "old edited", "new", "middle"]
        );
    }

    #[test]
    fn update_changes_body_and_updated_at() {
        let f = Fixture::new();
        let owner = f.client();
        let note = f.note(owner, "draft", false);
        f.clock.advance_millis(500);
        let updated = f.repo().update(note.id, input("final", true)).unwrap();
        assert_eq!(updated.body_md, "final");
        assert!(updated.pinned);
        assert_eq!(updated.created_at, note.created_at);
        assert_eq!(updated.updated_at, note.updated_at.plus_millis(500));
        let missing = Id::new();
        assert_eq!(
            f.repo().update(missing, input("x", false)).unwrap_err(),
            CoreError::not_found("note", missing)
        );
    }

    #[test]
    fn pinning_keeps_updated_at() {
        let f = Fixture::new();
        let owner = f.client();
        let note = f.note(owner, "x", false);
        f.clock.advance_millis(500);
        let pinned = f.repo().set_pinned(note.id, true).unwrap();
        assert!(pinned.pinned);
        assert_eq!(pinned.updated_at, note.updated_at);
        assert!(!f.repo().set_pinned(note.id, false).unwrap().pinned);
        assert_eq!(
            f.repo().set_pinned(Id::new(), true).unwrap_err().code,
            "note.not_found"
        );
    }

    #[test]
    fn delete_removes_note() {
        let f = Fixture::new();
        let owner = f.client();
        let note = f.note(owner, "x", false);
        f.repo().delete(note.id).unwrap();
        assert!(f.repo().list(owner).unwrap().is_empty());
        assert_eq!(f.repo().delete(note.id).unwrap_err().code, "note.not_found");
    }

    #[test]
    fn owners_do_not_see_each_others_notes() {
        let f = Fixture::new();
        let (a, b) = (f.client(), f.client());
        f.note(a, "a", false);
        assert!(f.repo().list(b).unwrap().is_empty());
    }

    #[test]
    fn deleting_client_deletes_its_notes() {
        let f = Fixture::new();
        let owner = f.client();
        let (_, id) = owner.parts();
        f.note(owner, "x", true);
        ClientRepo::new(f.db.conn(), &f.clock).delete(id).unwrap();
        let left: i64 =
            f.db.conn()
                .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
                .unwrap();
        assert_eq!(left, 0);
    }
}
