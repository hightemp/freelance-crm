//! Global colored tags attached to clients (later also projects, tasks and
//! secrets). Tag names are unique regardless of letter case.

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::domain::text::fold;
use crate::domain::{Clock, EntityRef, Id, Timestamp};
use crate::{CoreError, CoreResult};

pub const MAX_NAME_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TagInput {
    pub name: String,
    /// `#rrggbb`.
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: Id,
    pub name: String,
    pub color: String,
    pub created_at: Timestamp,
}

/// Contacts and other entities without tags are rejected before touching
/// the database (its `CHECK` would fail with an opaque error).
fn ensure_taggable(target: EntityRef) -> CoreResult<()> {
    match target {
        EntityRef::Client(_) => Ok(()),
        EntityRef::Contact(_) => {
            Err(CoreError::new("tag.unsupported_target").with("type", target.parts().0))
        }
    }
}

fn is_hex_color(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit())
}

impl TagInput {
    pub fn normalized(self) -> CoreResult<Self> {
        let name = self.name.trim().to_owned();
        if name.is_empty() {
            return Err(CoreError::validation("name", "required"));
        }
        if name.chars().count() > MAX_NAME_LEN {
            return Err(CoreError::validation("name", "maxLength").with("max", MAX_NAME_LEN));
        }
        let color = self.color.trim().to_ascii_lowercase();
        if !is_hex_color(&color) {
            return Err(CoreError::validation("color", "color"));
        }
        Ok(Self { name, color })
    }
}

const COLUMNS: &str = "tags.id, tags.name, tags.color, tags.created_at";

fn from_row(row: &Row<'_>) -> rusqlite::Result<Tag> {
    Ok(Tag {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
        created_at: row.get(3)?,
    })
}

pub struct TagService<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> TagService<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    /// All tags ordered by name.
    pub fn list(&self) -> CoreResult<Vec<Tag>> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM tags ORDER BY name_key"))?;
        let rows = stmt.query_map([], from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    fn get(&self, id: Id) -> CoreResult<Tag> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM tags WHERE id = ?1"),
                [id],
                from_row,
            )
            .optional()?
            .ok_or_else(|| CoreError::not_found("tag", id))
    }

    /// Fails with `tag.duplicate` if another tag has the same folded name.
    fn ensure_unique(&self, name: &str, except: Option<Id>) -> CoreResult<()> {
        let taken: bool = self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM tags WHERE name_key = ?1 AND id IS NOT ?2)",
            (fold(name), except),
            |r| r.get(0),
        )?;
        if taken {
            Err(CoreError::new("tag.duplicate").with("name", name))
        } else {
            Ok(())
        }
    }

    pub fn create(&self, input: TagInput) -> CoreResult<Tag> {
        let input = input.normalized()?;
        self.ensure_unique(&input.name, None)?;
        let id = Id::new();
        self.conn.execute(
            "INSERT INTO tags (id, name, name_key, color, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            (
                id,
                &input.name,
                fold(&input.name),
                &input.color,
                self.clock.now(),
            ),
        )?;
        self.get(id)
    }

    pub fn update(&self, id: Id, input: TagInput) -> CoreResult<Tag> {
        let input = input.normalized()?;
        self.get(id)?;
        self.ensure_unique(&input.name, Some(id))?;
        self.conn.execute(
            "UPDATE tags SET name = ?2, name_key = ?3, color = ?4, updated_at = ?5 WHERE id = ?1",
            (
                id,
                &input.name,
                fold(&input.name),
                &input.color,
                self.clock.now(),
            ),
        )?;
        self.get(id)
    }

    /// Deletes the tag and detaches it everywhere.
    pub fn delete(&self, id: Id) -> CoreResult<()> {
        match self.conn.execute("DELETE FROM tags WHERE id = ?1", [id])? {
            0 => Err(CoreError::not_found("tag", id)),
            _ => Ok(()),
        }
    }

    /// Attaching an already attached tag is a no-op.
    pub fn attach(&self, tag_id: Id, target: EntityRef) -> CoreResult<()> {
        ensure_taggable(target)?;
        self.get(tag_id)?;
        target.ensure_exists(self.conn)?;
        let (owner_type, owner_id) = target.parts();
        self.conn.execute(
            "INSERT OR IGNORE INTO taggings (tag_id, owner_type, owner_id) VALUES (?1, ?2, ?3)",
            (tag_id, owner_type, owner_id),
        )?;
        Ok(())
    }

    pub fn detach(&self, tag_id: Id, target: EntityRef) -> CoreResult<()> {
        let (owner_type, owner_id) = target.parts();
        self.conn.execute(
            "DELETE FROM taggings WHERE tag_id = ?1 AND owner_type = ?2 AND owner_id = ?3",
            (tag_id, owner_type, owner_id),
        )?;
        Ok(())
    }

    /// Tags of the target ordered by name.
    pub fn tags_of(&self, target: EntityRef) -> CoreResult<Vec<Tag>> {
        let (owner_type, owner_id) = target.parts();
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM tags
             JOIN taggings t ON t.tag_id = tags.id
             WHERE t.owner_type = ?1 AND t.owner_id = ?2
             ORDER BY tags.name_key"
        ))?;
        let rows = stmt.query_map((owner_type, owner_id), from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Makes `tag_ids` the exact set of the target's tags.
    pub fn set_tags(&self, target: EntityRef, tag_ids: &[Id]) -> CoreResult<Vec<Tag>> {
        ensure_taggable(target)?;
        for &id in tag_ids {
            self.get(id)?;
        }
        target.ensure_exists(self.conn)?;
        let (owner_type, owner_id) = target.parts();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM taggings WHERE owner_type = ?1 AND owner_id = ?2",
            (owner_type, owner_id),
        )?;
        for &tag_id in tag_ids {
            tx.execute(
                "INSERT OR IGNORE INTO taggings (tag_id, owner_type, owner_id)
                 VALUES (?1, ?2, ?3)",
                (tag_id, owner_type, owner_id),
            )?;
        }
        tx.commit()?;
        self.tags_of(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientFilter, ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use rstest::rstest;

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

        fn tags(&self) -> TagService<'_> {
            TagService::new(self.db.conn(), &self.clock)
        }

        fn clients(&self) -> ClientRepo<'_> {
            ClientRepo::new(self.db.conn(), &self.clock)
        }

        fn tag(&self, name: &str) -> Tag {
            self.tags().create(input(name, "#3b82f6")).unwrap()
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
            self.clients().create(input).unwrap().id
        }

        fn taggings(&self) -> i64 {
            self.db
                .conn()
                .query_row("SELECT count(*) FROM taggings", [], |r| r.get(0))
                .unwrap()
        }
    }

    fn input(name: &str, color: &str) -> TagInput {
        TagInput {
            name: name.into(),
            color: color.into(),
        }
    }

    fn names(tags: &[Tag]) -> Vec<&str> {
        tags.iter().map(|t| t.name.as_str()).collect()
    }

    #[rstest]
    #[case(input("  срочно ", "#AABBCC"), "срочно", "#aabbcc")]
    #[case(input("VIP", "#000000"), "VIP", "#000000")]
    fn valid_input_is_normalized(#[case] input: TagInput, #[case] name: &str, #[case] color: &str) {
        let n = input.normalized().unwrap();
        assert_eq!((n.name.as_str(), n.color.as_str()), (name, color));
    }

    #[rstest]
    #[case(input("  ", "#000000"), CoreError::validation("name", "required"))]
    #[case(
        input(&"я".repeat(MAX_NAME_LEN + 1), "#000000"),
        CoreError::validation("name", "maxLength").with("max", MAX_NAME_LEN)
    )]
    #[case(input("a", "red"), CoreError::validation("color", "color"))]
    #[case(input("a", "#12345"), CoreError::validation("color", "color"))]
    #[case(input("a", "#12345g"), CoreError::validation("color", "color"))]
    fn invalid_input_is_rejected(#[case] input: TagInput, #[case] err: CoreError) {
        assert_eq!(input.normalized().unwrap_err(), err);
    }

    #[test]
    fn create_and_list_sorted_by_name() {
        let f = Fixture::new();
        f.tag("Срочно");
        f.tag("ёлка");
        f.tag("архив");
        let created = f.tags().create(input("VIP", "#ff0000")).unwrap();
        assert_eq!(created.color, "#ff0000");
        assert_eq!(
            created.created_at,
            Timestamp::from_millis(1_700_000_000_000)
        );
        assert_eq!(
            names(&f.tags().list().unwrap()),
            ["VIP", "архив", "ёлка", "Срочно"]
        );
    }

    #[test]
    fn names_are_unique_case_insensitively() {
        let f = Fixture::new();
        f.tag("Срочно");
        let err = f.tags().create(input("СРОЧНО", "#000000")).unwrap_err();
        assert_eq!(err, CoreError::new("tag.duplicate").with("name", "СРОЧНО"));
        let err = f.tags().create(input("срочно", "#000000")).unwrap_err();
        assert_eq!(err.code, "tag.duplicate");
    }

    #[test]
    fn update_renames_and_recolors() {
        let f = Fixture::new();
        let tag = f.tag("vip");
        let updated = f.tags().update(tag.id, input("VIP", "#00ff00")).unwrap();
        assert_eq!(
            (updated.name.as_str(), updated.color.as_str()),
            ("VIP", "#00ff00")
        );
        assert_eq!(names(&f.tags().list().unwrap()), ["VIP"]);
    }

    #[test]
    fn update_rejects_duplicate_and_unknown_tag() {
        let f = Fixture::new();
        f.tag("A");
        let b = f.tag("B");
        assert_eq!(
            f.tags()
                .update(b.id, input("a", "#000000"))
                .unwrap_err()
                .code,
            "tag.duplicate"
        );
        let id = Id::new();
        assert_eq!(
            f.tags().update(id, input("C", "#000000")).unwrap_err(),
            CoreError::not_found("tag", id)
        );
    }

    #[test]
    fn attach_and_detach() {
        let f = Fixture::new();
        let (a, b) = (f.tag("A"), f.tag("B"));
        let target = EntityRef::Client(f.client("Ромашка"));
        f.tags().attach(b.id, target).unwrap();
        f.tags().attach(a.id, target).unwrap();
        f.tags().attach(a.id, target).unwrap();
        assert_eq!(names(&f.tags().tags_of(target).unwrap()), ["A", "B"]);

        f.tags().detach(a.id, target).unwrap();
        assert_eq!(names(&f.tags().tags_of(target).unwrap()), ["B"]);
        // Detaching a tag that is not attached is a no-op.
        f.tags().detach(a.id, target).unwrap();
    }

    #[test]
    fn attach_checks_tag_and_target() {
        let f = Fixture::new();
        let tag = f.tag("A");
        let client = f.client("Ромашка");
        let missing = Id::new();
        assert_eq!(
            f.tags()
                .attach(missing, EntityRef::Client(client))
                .unwrap_err(),
            CoreError::not_found("tag", missing)
        );
        assert_eq!(
            f.tags()
                .attach(tag.id, EntityRef::Client(missing))
                .unwrap_err(),
            CoreError::not_found("client", missing)
        );
    }

    #[test]
    fn contacts_cannot_be_tagged() {
        let f = Fixture::new();
        let tag = f.tag("A");
        let contact = EntityRef::Contact(Id::new());
        let err = CoreError::new("tag.unsupported_target").with("type", "contact");
        assert_eq!(f.tags().attach(tag.id, contact).unwrap_err(), err);
        assert_eq!(f.tags().set_tags(contact, &[tag.id]).unwrap_err(), err);
    }

    #[test]
    fn set_tags_replaces_the_set() {
        let f = Fixture::new();
        let (a, b, c) = (f.tag("A"), f.tag("B"), f.tag("C"));
        let target = EntityRef::Client(f.client("Ромашка"));
        f.tags().set_tags(target, &[a.id, b.id]).unwrap();
        let tags = f.tags().set_tags(target, &[c.id, b.id, c.id]).unwrap();
        assert_eq!(names(&tags), ["B", "C"]);

        let missing = Id::new();
        assert_eq!(
            f.tags().set_tags(target, &[a.id, missing]).unwrap_err(),
            CoreError::not_found("tag", missing)
        );
        assert_eq!(names(&f.tags().tags_of(target).unwrap()), ["B", "C"]);
    }

    #[test]
    fn deleting_tag_deletes_its_taggings() {
        let f = Fixture::new();
        let tag = f.tag("A");
        f.tags()
            .attach(tag.id, EntityRef::Client(f.client("Ромашка")))
            .unwrap();
        f.tags().delete(tag.id).unwrap();
        assert_eq!(f.taggings(), 0);
        assert!(f.tags().list().unwrap().is_empty());
        assert_eq!(f.tags().delete(tag.id).unwrap_err().code, "tag.not_found");
    }

    #[test]
    fn deleting_client_deletes_its_taggings() {
        let f = Fixture::new();
        let tag = f.tag("A");
        let client = f.client("Ромашка");
        f.tags().attach(tag.id, EntityRef::Client(client)).unwrap();
        f.clients().delete(client).unwrap();
        assert_eq!(f.taggings(), 0);
    }

    #[test]
    fn clients_are_filtered_by_all_given_tags() {
        let f = Fixture::new();
        let (vip, urgent) = (f.tag("VIP"), f.tag("Срочно"));
        let both = f.client("Оба");
        let only_vip = f.client("Только VIP");
        f.client("Без тегов");
        f.tags()
            .set_tags(EntityRef::Client(both), &[vip.id, urgent.id])
            .unwrap();
        f.tags()
            .set_tags(EntityRef::Client(only_vip), &[vip.id])
            .unwrap();

        let with = |tag_ids: Vec<Id>| {
            let filter = ClientFilter {
                tag_ids,
                ..ClientFilter::default()
            };
            f.clients()
                .list(&filter)
                .unwrap()
                .into_iter()
                .map(|c| c.display_name)
                .collect::<Vec<_>>()
        };
        assert_eq!(with(vec![vip.id]), ["Оба", "Только VIP"]);
        assert_eq!(with(vec![vip.id, urgent.id]), ["Оба"]);
        assert_eq!(with(vec![]).len(), 3);
    }
}
