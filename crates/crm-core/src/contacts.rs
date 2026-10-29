//! Contact persons of company clients. At most one per company is primary.

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::clients::{ClientKind, ClientRepo};
use crate::domain::text::{clean, fold, full_name};
use crate::domain::{Clock, Id, Timestamp};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContactInput {
    pub last_name: Option<String>,
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    /// Job title or role in the project.
    pub position: Option<String>,
    pub note: Option<String>,
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: Id,
    pub client_id: Id,
    #[serde(flatten)]
    pub input: ContactInput,
    pub display_name: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl ContactInput {
    pub fn normalized(self) -> CoreResult<Self> {
        let n = Self {
            last_name: clean(self.last_name),
            first_name: clean(self.first_name),
            middle_name: clean(self.middle_name),
            position: clean(self.position),
            note: clean(self.note),
            is_primary: self.is_primary,
        };
        if n.last_name.is_none() && n.first_name.is_none() {
            return Err(CoreError::validation("firstName", "required"));
        }
        Ok(n)
    }

    pub fn display_name(&self) -> String {
        full_name(&self.last_name, &self.first_name, &self.middle_name)
    }
}

const COLUMNS: &str = "id, client_id, last_name, first_name, middle_name, position, note, \
    is_primary, display_name, created_at, updated_at";

fn from_row(row: &Row<'_>) -> rusqlite::Result<Contact> {
    Ok(Contact {
        id: row.get("id")?,
        client_id: row.get("client_id")?,
        input: ContactInput {
            last_name: row.get("last_name")?,
            first_name: row.get("first_name")?,
            middle_name: row.get("middle_name")?,
            position: row.get("position")?,
            note: row.get("note")?,
            is_primary: row.get("is_primary")?,
        },
        display_name: row.get("display_name")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn not_found(id: Id) -> CoreError {
    CoreError::not_found("contact", id)
}

pub struct ContactRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> ContactRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    /// Contacts of a client: the primary one first, then by name.
    pub fn list(&self, client_id: Id) -> CoreResult<Vec<Contact>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM contacts WHERE client_id = ?1
             ORDER BY is_primary DESC, sort_key, id"
        ))?;
        let rows = stmt.query_map([client_id], from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn get(&self, id: Id) -> CoreResult<Contact> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM contacts WHERE id = ?1"),
                [id],
                from_row,
            )
            .optional()?
            .ok_or_else(|| not_found(id))
    }

    /// Clears the primary flag of the client's other contacts.
    fn clear_primary(conn: &Connection, client_id: Id, except: Id) -> CoreResult<()> {
        conn.execute(
            "UPDATE contacts SET is_primary = 0 WHERE client_id = ?1 AND id != ?2",
            (client_id, except),
        )?;
        Ok(())
    }

    /// Only company clients have contact persons.
    pub fn create(&self, client_id: Id, input: ContactInput) -> CoreResult<Contact> {
        let input = input.normalized()?;
        let client = ClientRepo::new(self.conn, self.clock).get(client_id)?;
        if client.input.kind != ClientKind::Company {
            return Err(CoreError::new("contact.client_not_company"));
        }
        let id = Id::new();
        let name = input.display_name();
        let tx = self.conn.unchecked_transaction()?;
        if input.is_primary {
            Self::clear_primary(&tx, client_id, id)?;
        }
        tx.execute(
            "INSERT INTO contacts (id, client_id, last_name, first_name, middle_name, position,
                 note, is_primary, display_name, sort_key, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
            (
                id,
                client_id,
                &input.last_name,
                &input.first_name,
                &input.middle_name,
                &input.position,
                &input.note,
                input.is_primary,
                &name,
                fold(&name),
                self.clock.now(),
            ),
        )?;
        tx.commit()?;
        self.get(id)
    }

    pub fn update(&self, id: Id, input: ContactInput) -> CoreResult<Contact> {
        let input = input.normalized()?;
        let current = self.get(id)?;
        let name = input.display_name();
        let tx = self.conn.unchecked_transaction()?;
        if input.is_primary {
            Self::clear_primary(&tx, current.client_id, id)?;
        }
        tx.execute(
            "UPDATE contacts SET last_name = ?2, first_name = ?3, middle_name = ?4,
                 position = ?5, note = ?6, is_primary = ?7, display_name = ?8, sort_key = ?9,
                 updated_at = ?10
             WHERE id = ?1",
            (
                id,
                &input.last_name,
                &input.first_name,
                &input.middle_name,
                &input.position,
                &input.note,
                input.is_primary,
                &name,
                fold(&name),
                self.clock.now(),
            ),
        )?;
        tx.commit()?;
        self.get(id)
    }

    /// Makes the contact primary and clears the flag on the previous one.
    pub fn set_primary(&self, id: Id) -> CoreResult<Contact> {
        let current = self.get(id)?;
        let tx = self.conn.unchecked_transaction()?;
        Self::clear_primary(&tx, current.client_id, id)?;
        tx.execute("UPDATE contacts SET is_primary = 1 WHERE id = ?1", [id])?;
        tx.commit()?;
        self.get(id)
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        match self
            .conn
            .execute("DELETE FROM contacts WHERE id = ?1", [id])?
        {
            0 => Err(not_found(id)),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientStatus};
    use crate::contact_methods::{ContactMethodInput, ContactMethodKind, ContactMethodRepo};
    use crate::custom_fields::{CustomFieldInput, CustomFieldRepo, FieldType};
    use crate::db::Db;
    use crate::domain::{EntityRef, FakeClock};
    use crate::notes::{NoteInput, NoteRepo};

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

        fn clients(&self) -> ClientRepo<'_> {
            ClientRepo::new(self.db.conn(), &self.clock)
        }

        fn repo(&self) -> ContactRepo<'_> {
            ContactRepo::new(self.db.conn(), &self.clock)
        }

        fn client(&self, kind: ClientKind) -> Id {
            let input = ClientInput {
                kind,
                last_name: Some("Петров".into()),
                first_name: None,
                middle_name: None,
                company_name: Some("Ромашка".into()),
                legal_name: None,
                status: ClientStatus::Active,
                default_currency: None,
                default_rate_minor: None,
            };
            self.clients().create(input).unwrap().id
        }

        fn company(&self) -> Id {
            self.client(ClientKind::Company)
        }

        fn contact(&self, client: Id, last: &str, primary: bool) -> Contact {
            let input = ContactInput {
                is_primary: primary,
                ..person(last, "Иван")
            };
            self.repo().create(client, input).unwrap()
        }

        fn count(&self, table: &str) -> i64 {
            self.db
                .conn()
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        }
    }

    fn person(last: &str, first: &str) -> ContactInput {
        let opt = |s: &str| (!s.is_empty()).then(|| s.to_owned());
        ContactInput {
            last_name: opt(last),
            first_name: opt(first),
            middle_name: None,
            position: None,
            note: None,
            is_primary: false,
        }
    }

    fn primary_names(contacts: &[Contact]) -> Vec<(&str, bool)> {
        contacts
            .iter()
            .map(|c| {
                (
                    c.input.last_name.as_deref().unwrap_or(""),
                    c.input.is_primary,
                )
            })
            .collect()
    }

    #[test]
    fn name_is_required_and_fields_are_trimmed() {
        assert_eq!(
            person(" ", "").normalized().unwrap_err(),
            CoreError::validation("firstName", "required")
        );
        let n = ContactInput {
            middle_name: Some("  ".into()),
            position: Some(" CTO ".into()),
            note: Some("".into()),
            ..person(" Иванов ", "")
        }
        .normalized()
        .unwrap();
        assert_eq!(n.last_name.as_deref(), Some("Иванов"));
        assert_eq!(n.middle_name, None);
        assert_eq!(n.position.as_deref(), Some("CTO"));
        assert_eq!(n.note, None);
    }

    #[test]
    fn create_and_get() {
        let f = Fixture::new();
        let client = f.company();
        let input = ContactInput {
            middle_name: Some("Петрович".into()),
            position: Some("Менеджер проекта".into()),
            note: Some("Отвечает быстро".into()),
            ..person("Сидоров", "Иван")
        };
        let contact = f.repo().create(client, input).unwrap();
        assert_eq!(contact.client_id, client);
        assert_eq!(contact.display_name, "Сидоров Иван Петрович");
        assert_eq!(contact.created_at, Timestamp::from_millis(T0));
        assert_eq!(f.repo().get(contact.id).unwrap(), contact);
        assert_eq!(
            f.repo().get(Id::new()).unwrap_err().code,
            "contact.not_found"
        );
    }

    #[test]
    fn person_client_cannot_have_contacts() {
        let f = Fixture::new();
        let client = f.client(ClientKind::Person);
        assert_eq!(
            f.repo().create(client, person("Иванов", "")).unwrap_err(),
            CoreError::new("contact.client_not_company")
        );
        let missing = Id::new();
        assert_eq!(
            f.repo().create(missing, person("Иванов", "")).unwrap_err(),
            CoreError::not_found("client", missing)
        );
    }

    #[test]
    fn company_with_contacts_cannot_become_a_person() {
        let f = Fixture::new();
        let client = f.company();
        f.contact(client, "Иванов", false);
        let mut input = f.clients().get(client).unwrap().input;
        input.kind = ClientKind::Person;
        input.last_name = Some("Петров".into());
        assert_eq!(
            f.clients().update(client, input).unwrap_err(),
            CoreError::new("client.has_contacts")
        );
    }

    #[test]
    fn list_shows_primary_first_then_by_name() {
        let f = Fixture::new();
        let client = f.company();
        f.contact(client, "Яковлев", false);
        f.contact(client, "Борисов", false);
        f.contact(client, "Сидоров", true);
        f.contact(f.company(), "Чужой", false);
        assert_eq!(
            primary_names(&f.repo().list(client).unwrap()),
            [("Сидоров", true), ("Борисов", false), ("Яковлев", false)]
        );
    }

    #[test]
    fn new_primary_contact_clears_the_previous_one() {
        let f = Fixture::new();
        let client = f.company();
        let a = f.contact(client, "А", true);
        let b = f.contact(client, "Б", true);
        assert_eq!(
            primary_names(&f.repo().list(client).unwrap()),
            [("Б", true), ("А", false)]
        );

        f.repo().set_primary(a.id).unwrap();
        assert_eq!(
            primary_names(&f.repo().list(client).unwrap()),
            [("А", true), ("Б", false)]
        );

        let input = ContactInput {
            is_primary: true,
            ..b.input
        };
        f.repo().update(b.id, input).unwrap();
        assert!(!f.repo().get(a.id).unwrap().input.is_primary);

        // Primary contacts of other companies are untouched.
        let other = f.contact(f.company(), "В", true);
        assert!(f.repo().get(other.id).unwrap().input.is_primary);
        assert_eq!(
            f.repo().set_primary(Id::new()).unwrap_err().code,
            "contact.not_found"
        );
    }

    #[test]
    fn update_changes_fields_and_updated_at() {
        let f = Fixture::new();
        let client = f.company();
        let c = f.contact(client, "Иванов", false);
        f.clock.advance_millis(1_000);
        let updated = f.repo().update(c.id, person("Петров", "Пётр")).unwrap();
        assert_eq!(updated.display_name, "Петров Пётр");
        assert_eq!(updated.updated_at, Timestamp::from_millis(T0 + 1_000));
        assert_eq!(updated.created_at, c.created_at);
        assert!(f.repo().update(c.id, person("", "")).is_err());
        assert_eq!(
            f.repo()
                .update(Id::new(), person("X", ""))
                .unwrap_err()
                .code,
            "contact.not_found"
        );
    }

    #[test]
    fn delete_removes_contact_and_its_data() {
        let f = Fixture::new();
        let client = f.company();
        let c = f.contact(client, "Иванов", true);
        attach_everything(&f, c.id);
        f.repo().delete(c.id).unwrap();
        assert_eq!(f.repo().get(c.id).unwrap_err().code, "contact.not_found");
        assert_eq!(f.repo().delete(c.id).unwrap_err().code, "contact.not_found");
        for table in ["contacts", "contact_methods", "custom_fields", "notes"] {
            assert_eq!(f.count(table), 0, "{table}");
        }
    }

    #[test]
    fn deleting_client_cascades_to_contacts_and_their_data() {
        let f = Fixture::new();
        let client = f.company();
        let c = f.contact(client, "Иванов", true);
        attach_everything(&f, c.id);
        f.clients().delete(client).unwrap();
        for table in ["contacts", "contact_methods", "custom_fields", "notes"] {
            assert_eq!(f.count(table), 0, "{table}");
        }
    }

    fn attach_everything(f: &Fixture, contact: Id) {
        let owner = EntityRef::Contact(contact);
        ContactMethodRepo::new(f.db.conn(), &f.clock)
            .replace(
                owner,
                vec![ContactMethodInput {
                    kind: ContactMethodKind::Email,
                    value: "ivan@example.com".into(),
                    label: None,
                }],
            )
            .unwrap();
        CustomFieldRepo::new(f.db.conn(), &f.clock)
            .create(
                owner,
                CustomFieldInput {
                    key: "День рождения".into(),
                    value: "1990-05-01".into(),
                    field_type: FieldType::Date,
                },
            )
            .unwrap();
        NoteRepo::new(f.db.conn(), &f.clock)
            .create(
                owner,
                NoteInput {
                    body_md: "x".into(),
                    pinned: false,
                },
            )
            .unwrap();
    }

    #[test]
    fn delete_impact_counts_everything_removed_with_the_client() {
        use crate::clients::DeleteImpact;
        use crate::tags::{TagInput, TagService};

        let f = Fixture::new();
        let client = f.company();
        let other = f.company();
        for name in ["А", "Б"] {
            let c = f.contact(client, name, false);
            attach_everything(&f, c.id);
        }
        f.contact(other, "Чужой", false);
        let owner = EntityRef::Client(client);
        ContactMethodRepo::new(f.db.conn(), &f.clock)
            .replace(
                owner,
                vec![ContactMethodInput {
                    kind: ContactMethodKind::Phone,
                    value: "+7 912 345-67-89".into(),
                    label: None,
                }],
            )
            .unwrap();
        NoteRepo::new(f.db.conn(), &f.clock)
            .create(
                owner,
                NoteInput {
                    body_md: "x".into(),
                    pinned: false,
                },
            )
            .unwrap();
        let tags = TagService::new(f.db.conn(), &f.clock);
        let tag = tags
            .create(TagInput {
                name: "VIP".into(),
                color: "#000000".into(),
            })
            .unwrap();
        tags.set_tags(owner, &[tag.id]).unwrap();

        // Contacts' own methods, fields and notes are counted too.
        assert_eq!(
            f.clients().delete_impact(client).unwrap(),
            DeleteImpact {
                contacts: 2,
                projects: 0,
                tasks: 0,
                contact_methods: 3,
                custom_fields: 2,
                notes: 3,
            }
        );
        assert_eq!(
            f.clients().delete_impact(other).unwrap(),
            DeleteImpact {
                contacts: 1,
                ..DeleteImpact::default()
            }
        );
        let missing = Id::new();
        assert_eq!(
            f.clients().delete_impact(missing).unwrap_err(),
            CoreError::not_found("client", missing)
        );
    }

    #[test]
    fn serializes_flat_camel_case() {
        let f = Fixture::new();
        let c = f.contact(f.company(), "Иванов", true);
        let json = serde_json::to_value(&c).unwrap();
        assert_eq!(json["lastName"], "Иванов");
        assert_eq!(json["isPrimary"], true);
        assert_eq!(json["clientId"], c.client_id.to_string());
        assert_eq!(json["displayName"], "Иванов Иван");
    }
}
