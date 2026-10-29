//! User-defined key/value fields on clients (later contacts and projects),
//! typed for validation and display, ordered by the user.

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::contact_methods::{ContactMethodKind, normalize_value};
use crate::db::text_enum;
use crate::domain::{Clock, EntityRef, Id};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Url,
    Email,
    Phone,
    /// `YYYY-MM-DD`.
    Date,
    /// Decimal with `.` as separator.
    Number,
}

text_enum!(FieldType {
    Text => "text",
    Url => "url",
    Email => "email",
    Phone => "phone",
    Date => "date",
    Number => "number",
});

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldInput {
    pub key: String,
    /// May be empty: the field exists but is not filled in yet.
    pub value: String,
    pub field_type: FieldType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CustomField {
    pub id: Id,
    pub key: String,
    pub value: String,
    pub field_type: FieldType,
}

fn is_leap(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

/// Strict `YYYY-MM-DD` with a real calendar day.
fn is_date(value: &str) -> bool {
    let b = value.as_bytes();
    let shape = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && [0, 1, 2, 3, 5, 6, 8, 9]
            .iter()
            .all(|&i| b[i].is_ascii_digit());
    if !shape {
        return false;
    }
    let num = |range: std::ops::Range<usize>| value[range].parse::<u32>().unwrap_or(0);
    let (year, month, day) = (num(0..4), num(5..7), num(8..10));
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

/// `-12`, `3.5` or `3,5`; returned with `.` as the decimal separator.
fn number(value: &str) -> Option<String> {
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    let (int, frac) = match unsigned.split_once(['.', ',']) {
        Some((int, frac)) => (int, Some(frac)),
        None => (unsigned, None),
    };
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    (digits(int) && frac.is_none_or(digits)).then(|| value.replace(',', "."))
}

impl CustomFieldInput {
    pub fn normalized(self) -> CoreResult<Self> {
        let key = self.key.trim().to_owned();
        if key.is_empty() {
            return Err(CoreError::validation("key", "required"));
        }
        let value = self.value.trim();
        let value = if value.is_empty() {
            String::new()
        } else {
            let as_contact = |kind| normalize_value(kind, value).ok();
            let (normalized, rule) = match self.field_type {
                FieldType::Text => (Some(value.to_owned()), ""),
                FieldType::Email => (as_contact(ContactMethodKind::Email), "email"),
                FieldType::Url => (as_contact(ContactMethodKind::Website), "url"),
                FieldType::Phone => (as_contact(ContactMethodKind::Phone), "phone"),
                FieldType::Date => (is_date(value).then(|| value.to_owned()), "date"),
                FieldType::Number => (number(value), "number"),
            };
            normalized.ok_or_else(|| CoreError::validation("value", rule))?
        };
        Ok(Self { key, value, ..self })
    }
}

const COLUMNS: &str = "id, key, value, field_type";

fn from_row(row: &Row<'_>) -> rusqlite::Result<CustomField> {
    Ok(CustomField {
        id: row.get(0)?,
        key: row.get(1)?,
        value: row.get(2)?,
        field_type: row.get(3)?,
    })
}

pub struct CustomFieldRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> CustomFieldRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    /// Fields of the owner in user-defined order.
    pub fn list(&self, owner: EntityRef) -> CoreResult<Vec<CustomField>> {
        let (owner_type, owner_id) = owner.parts();
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM custom_fields
             WHERE owner_type = ?1 AND owner_id = ?2
             ORDER BY position, id"
        ))?;
        let rows = stmt.query_map((owner_type, owner_id), from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    fn get(&self, id: Id) -> CoreResult<CustomField> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM custom_fields WHERE id = ?1"),
                [id],
                from_row,
            )
            .optional()?
            .ok_or_else(|| CoreError::not_found("custom_field", id))
    }

    /// Adds a field at the end.
    pub fn create(&self, owner: EntityRef, input: CustomFieldInput) -> CoreResult<CustomField> {
        let input = input.normalized()?;
        owner.ensure_exists(self.conn)?;
        let (owner_type, owner_id) = owner.parts();
        let id = Id::new();
        self.conn.execute(
            "INSERT INTO custom_fields
                 (id, owner_type, owner_id, key, value, field_type, position, created_at, updated_at)
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, coalesce(max(position) + 1, 0), ?7, ?7
             FROM custom_fields WHERE owner_type = ?2 AND owner_id = ?3",
            (
                id,
                owner_type,
                owner_id,
                &input.key,
                &input.value,
                input.field_type,
                self.clock.now(),
            ),
        )?;
        self.get(id)
    }

    pub fn update(&self, id: Id, input: CustomFieldInput) -> CoreResult<CustomField> {
        let input = input.normalized()?;
        let changed = self.conn.execute(
            "UPDATE custom_fields SET key = ?2, value = ?3, field_type = ?4, updated_at = ?5
             WHERE id = ?1",
            (
                id,
                &input.key,
                &input.value,
                input.field_type,
                self.clock.now(),
            ),
        )?;
        if changed == 0 {
            return Err(CoreError::not_found("custom_field", id));
        }
        self.get(id)
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        match self
            .conn
            .execute("DELETE FROM custom_fields WHERE id = ?1", [id])?
        {
            0 => Err(CoreError::not_found("custom_field", id)),
            _ => Ok(()),
        }
    }

    /// Sets the order; `ids` must be exactly the owner's fields.
    pub fn reorder(&self, owner: EntityRef, ids: &[Id]) -> CoreResult<Vec<CustomField>> {
        let mut current: Vec<Id> = self.list(owner)?.into_iter().map(|f| f.id).collect();
        let mut requested = ids.to_vec();
        current.sort();
        requested.sort();
        if current != requested {
            return Err(CoreError::validation("ids", "order"));
        }
        let tx = self.conn.unchecked_transaction()?;
        for (position, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE custom_fields SET position = ?2 WHERE id = ?1",
                (id, position as i64),
            )?;
        }
        tx.commit()?;
        self.list(owner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::{FakeClock, Timestamp};
    use FieldType::*;
    use rstest::rstest;

    fn field(key: &str, value: &str, field_type: FieldType) -> CustomFieldInput {
        CustomFieldInput {
            key: key.into(),
            value: value.into(),
            field_type,
        }
    }

    #[rstest]
    #[case(Text, "  любой текст ", "любой текст")]
    #[case(Text, "", "")]
    #[case(Email, "Ivan@Example.COM", "Ivan@example.com")]
    #[case(Url, "example.com", "https://example.com/")]
    #[case(Phone, "+7 912 345-67-89", "+7 912 345-67-89")]
    #[case(Date, "2024-02-29", "2024-02-29")]
    #[case(Date, "2000-02-29", "2000-02-29")]
    #[case(Date, "2025-12-31", "2025-12-31")]
    #[case(Number, "42", "42")]
    #[case(Number, "-3,5", "-3.5")]
    #[case(Number, " 0.25 ", "0.25")]
    #[case(Email, "  ", "")]
    #[case(Date, "", "")]
    fn values_are_validated_by_type(
        #[case] field_type: FieldType,
        #[case] value: &str,
        #[case] expected: &str,
    ) {
        let n = field("Ключ", value, field_type).normalized().unwrap();
        assert_eq!(n.value, expected);
    }

    #[rstest]
    #[case(Email, "ivan", "email")]
    #[case(Url, "not a url", "url")]
    #[case(Phone, "123", "phone")]
    #[case(Date, "2023-02-29", "date")]
    #[case(Date, "1900-02-29", "date")]
    #[case(Date, "2024-13-01", "date")]
    #[case(Date, "2024-04-31", "date")]
    #[case(Date, "2024-00-10", "date")]
    #[case(Date, "24-01-01", "date")]
    #[case(Date, "01.02.2024", "date")]
    #[case(Number, "abc", "number")]
    #[case(Number, "1.2.3", "number")]
    #[case(Number, "1e5", "number")]
    #[case(Number, "-", "number")]
    #[case(Number, ".5", "number")]
    fn invalid_values_are_rejected(
        #[case] field_type: FieldType,
        #[case] value: &str,
        #[case] rule: &str,
    ) {
        assert_eq!(
            field("Ключ", value, field_type).normalized().unwrap_err(),
            CoreError::validation("value", rule)
        );
    }

    #[test]
    fn key_is_required_and_trimmed() {
        assert_eq!(
            field("  ", "x", Text).normalized().unwrap_err(),
            CoreError::validation("key", "required")
        );
        assert_eq!(field(" ИНН ", "", Text).normalized().unwrap().key, "ИНН");
    }

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

        fn repo(&self) -> CustomFieldRepo<'_> {
            CustomFieldRepo::new(self.db.conn(), &self.clock)
        }
    }

    fn keys(fields: &[CustomField]) -> Vec<&str> {
        fields.iter().map(|f| f.key.as_str()).collect()
    }

    #[test]
    fn create_appends_in_order() {
        let f = Fixture::new();
        let owner = f.client();
        let a = f.repo().create(owner, field("A", "1", Number)).unwrap();
        f.repo().create(owner, field("B", "", Text)).unwrap();
        f.repo().create(owner, field("C", "", Date)).unwrap();
        assert_eq!(a.value, "1");
        assert_eq!(a.field_type, Number);
        assert_eq!(keys(&f.repo().list(owner).unwrap()), ["A", "B", "C"]);
    }

    #[test]
    fn create_checks_owner_and_value() {
        let f = Fixture::new();
        let missing = Id::new();
        assert_eq!(
            f.repo()
                .create(EntityRef::Client(missing), field("A", "", Text))
                .unwrap_err(),
            CoreError::not_found("client", missing)
        );
        let owner = f.client();
        assert!(f.repo().create(owner, field("A", "x", Email)).is_err());
        assert!(f.repo().list(owner).unwrap().is_empty());
    }

    #[test]
    fn update_changes_key_value_and_type_keeping_position() {
        let f = Fixture::new();
        let owner = f.client();
        let a = f.repo().create(owner, field("A", "", Text)).unwrap();
        f.repo().create(owner, field("B", "", Text)).unwrap();
        let updated = f
            .repo()
            .update(a.id, field("Сайт", "example.com", Url))
            .unwrap();
        assert_eq!(updated.value, "https://example.com/");
        assert_eq!(keys(&f.repo().list(owner).unwrap()), ["Сайт", "B"]);

        assert!(f.repo().update(a.id, field("Сайт", "x", Date)).is_err());
        let missing = Id::new();
        assert_eq!(
            f.repo().update(missing, field("A", "", Text)).unwrap_err(),
            CoreError::not_found("custom_field", missing)
        );
    }

    #[test]
    fn delete_removes_field() {
        let f = Fixture::new();
        let owner = f.client();
        let a = f.repo().create(owner, field("A", "", Text)).unwrap();
        f.repo().delete(a.id).unwrap();
        assert!(f.repo().list(owner).unwrap().is_empty());
        assert_eq!(
            f.repo().delete(a.id).unwrap_err().code,
            "custom_field.not_found"
        );
    }

    #[test]
    fn reorder_persists_positions() {
        let f = Fixture::new();
        let owner = f.client();
        let ids: Vec<Id> = ["A", "B", "C"]
            .iter()
            .map(|k| f.repo().create(owner, field(k, "", Text)).unwrap().id)
            .collect();
        let reordered = f.repo().reorder(owner, &[ids[2], ids[0], ids[1]]).unwrap();
        assert_eq!(keys(&reordered), ["C", "A", "B"]);
        assert_eq!(keys(&f.repo().list(owner).unwrap()), ["C", "A", "B"]);

        // A field added after reordering goes to the end.
        f.repo().create(owner, field("D", "", Text)).unwrap();
        assert_eq!(keys(&f.repo().list(owner).unwrap()), ["C", "A", "B", "D"]);
    }

    #[test]
    fn reorder_requires_exactly_the_owners_fields() {
        let f = Fixture::new();
        let owner = f.client();
        let other = f.client();
        let a = f.repo().create(owner, field("A", "", Text)).unwrap().id;
        let b = f.repo().create(owner, field("B", "", Text)).unwrap().id;
        let foreign = f.repo().create(other, field("X", "", Text)).unwrap().id;
        let invalid = CoreError::validation("ids", "order");
        for ids in [
            vec![a],
            vec![a, b, b],
            vec![a, foreign],
            vec![a, b, foreign],
        ] {
            assert_eq!(f.repo().reorder(owner, &ids).unwrap_err(), invalid);
        }
        assert_eq!(keys(&f.repo().list(owner).unwrap()), ["A", "B"]);
    }

    #[test]
    fn deleting_client_deletes_its_fields() {
        let f = Fixture::new();
        let owner @ EntityRef::Client(id) = f.client();
        f.repo().create(owner, field("A", "", Text)).unwrap();
        ClientRepo::new(f.db.conn(), &f.clock).delete(id).unwrap();
        let left: i64 =
            f.db.conn()
                .query_row("SELECT count(*) FROM custom_fields", [], |r| r.get(0))
                .unwrap();
        assert_eq!(left, 0);
    }
}
