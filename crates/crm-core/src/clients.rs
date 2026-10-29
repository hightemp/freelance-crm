//! Clients: private persons or companies the freelancer works with.

use rusqlite::{Connection, OptionalExtension, Row, named_params};
use serde::{Deserialize, Serialize};

use crate::db::text_enum;
use crate::domain::text::{clean, contains_pattern, fold, full_name};
use crate::domain::{Clock, Currency, Id, Timestamp};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ClientKind {
    Person,
    Company,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ClientStatus {
    Lead,
    Active,
    Paused,
    Former,
}

text_enum!(ClientKind { Person => "person", Company => "company" });
text_enum!(ClientStatus {
    Lead => "lead",
    Active => "active",
    Paused => "paused",
    Former => "former",
});

/// Editable fields of a client, used for both create and update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ClientInput {
    pub kind: ClientKind,
    pub last_name: Option<String>,
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    /// Short name of a company, shown in lists.
    pub company_name: Option<String>,
    /// Full legal name of a company, used in documents.
    pub legal_name: Option<String>,
    pub status: ClientStatus,
    /// Inherited by new projects; `None` uses the app default currency.
    pub default_currency: Option<Currency>,
    /// Hourly rate in minor units of `default_currency`.
    #[specta(type = Option<i32>)]
    pub default_rate_minor: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    pub id: Id,
    #[serde(flatten)]
    pub input: ClientInput,
    pub display_name: String,
    /// Attached tags, ordered by tag name.
    pub tag_ids: Vec<Id>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub archived_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ClientFilter {
    /// Case-insensitive substring of the display name.
    pub query: Option<String>,
    pub status: Option<ClientStatus>,
    /// `false` lists active clients, `true` only archived ones.
    pub archived: bool,
    /// Only clients having all of these tags.
    pub tag_ids: Vec<Id>,
}

/// What else is deleted together with a client, shown before deleting.
/// Grows with later modules (projects, tasks, events, files, secrets).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DeleteImpact {
    pub contacts: u32,
    pub projects: u32,
    /// Of the client and of its contacts.
    pub contact_methods: u32,
    pub custom_fields: u32,
    pub notes: u32,
}

impl ClientInput {
    /// Trims text fields, turns blank ones into `None` and checks required
    /// fields for the client kind.
    pub fn normalized(self) -> CoreResult<Self> {
        let is_person = self.kind == ClientKind::Person;
        let keep = |value: Option<String>, applies: bool| clean(value.filter(|_| applies));
        let n = Self {
            last_name: keep(self.last_name, is_person),
            first_name: keep(self.first_name, is_person),
            middle_name: keep(self.middle_name, is_person),
            company_name: keep(self.company_name, !is_person),
            legal_name: keep(self.legal_name, !is_person),
            ..self
        };
        if is_person && n.last_name.is_none() && n.first_name.is_none() {
            return Err(CoreError::validation("firstName", "required"));
        }
        if !is_person && n.company_name.is_none() {
            return Err(CoreError::validation("companyName", "required"));
        }
        if n.default_rate_minor.is_some_and(|rate| rate < 0) {
            return Err(CoreError::validation("defaultRateMinor", "min"));
        }
        Ok(n)
    }

    /// "Last First Middle" for a person, the short name for a company.
    pub fn display_name(&self) -> String {
        match self.kind {
            ClientKind::Person => full_name(&self.last_name, &self.first_name, &self.middle_name),
            ClientKind::Company => self.company_name.clone().unwrap_or_default(),
        }
    }
}

const COLUMNS: &str = "id, kind, last_name, first_name, middle_name, company_name, legal_name, \
    status, default_currency, default_rate_minor, display_name, created_at, updated_at, archived_at, \
    (SELECT json_group_array(t.tag_id ORDER BY g.name_key)
     FROM taggings t JOIN tags g ON g.id = t.tag_id
     WHERE t.owner_type = 'client' AND t.owner_id = clients.id) AS tag_ids";

/// Parses the `json_group_array` of tag ids.
fn tag_ids(row: &Row<'_>) -> rusqlite::Result<Vec<Id>> {
    let json: String = row.get("tag_ids")?;
    serde_json::from_str(&json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<Client> {
    Ok(Client {
        id: row.get("id")?,
        input: ClientInput {
            kind: row.get("kind")?,
            last_name: row.get("last_name")?,
            first_name: row.get("first_name")?,
            middle_name: row.get("middle_name")?,
            company_name: row.get("company_name")?,
            legal_name: row.get("legal_name")?,
            status: row.get("status")?,
            default_currency: row.get("default_currency")?,
            default_rate_minor: row.get("default_rate_minor")?,
        },
        display_name: row.get("display_name")?,
        tag_ids: tag_ids(row)?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        archived_at: row.get("archived_at")?,
    })
}

fn not_found(id: Id) -> CoreError {
    CoreError::not_found("client", id)
}

pub struct ClientRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> ClientRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    pub fn create(&self, input: ClientInput) -> CoreResult<Client> {
        let input = input.normalized()?;
        let id = Id::new();
        let now = self.clock.now();
        let name = input.display_name();
        self.conn.execute(
            "INSERT INTO clients (id, kind, last_name, first_name, middle_name, company_name,
                 legal_name, status, default_currency, default_rate_minor, display_name,
                 sort_key, created_at, updated_at)
             VALUES (:id, :kind, :last_name, :first_name, :middle_name, :company_name,
                 :legal_name, :status, :currency, :rate, :name, :sort_key, :now, :now)",
            named_params! {
                ":id": id,
                ":kind": input.kind,
                ":last_name": input.last_name,
                ":first_name": input.first_name,
                ":middle_name": input.middle_name,
                ":company_name": input.company_name,
                ":legal_name": input.legal_name,
                ":status": input.status,
                ":currency": input.default_currency,
                ":rate": input.default_rate_minor,
                ":sort_key": fold(&name),
                ":name": name,
                ":now": now,
            },
        )?;
        self.get(id)
    }

    pub fn get(&self, id: Id) -> CoreResult<Client> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM clients WHERE id = ?1"),
                [id],
                from_row,
            )
            .optional()?
            .ok_or_else(|| not_found(id))
    }

    pub fn update(&self, id: Id, input: ClientInput) -> CoreResult<Client> {
        let input = input.normalized()?;
        if input.kind == ClientKind::Person {
            let has_contacts: bool = self.conn.query_row(
                "SELECT EXISTS (SELECT 1 FROM contacts WHERE client_id = ?1)",
                [id],
                |r| r.get(0),
            )?;
            if has_contacts {
                return Err(CoreError::new("client.has_contacts"));
            }
        }
        let name = input.display_name();
        let changed = self.conn.execute(
            "UPDATE clients SET kind = :kind, last_name = :last_name, first_name = :first_name,
                 middle_name = :middle_name, company_name = :company_name,
                 legal_name = :legal_name, status = :status, default_currency = :currency,
                 default_rate_minor = :rate, display_name = :name, sort_key = :sort_key,
                 updated_at = :now
             WHERE id = :id",
            named_params! {
                ":id": id,
                ":kind": input.kind,
                ":last_name": input.last_name,
                ":first_name": input.first_name,
                ":middle_name": input.middle_name,
                ":company_name": input.company_name,
                ":legal_name": input.legal_name,
                ":status": input.status,
                ":currency": input.default_currency,
                ":rate": input.default_rate_minor,
                ":sort_key": fold(&name),
                ":name": name,
                ":now": self.clock.now(),
            },
        )?;
        if changed == 0 {
            return Err(not_found(id));
        }
        self.get(id)
    }

    pub fn list(&self, filter: &ClientFilter) -> CoreResult<Vec<Client>> {
        let query = filter
            .query
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
            .map(contains_pattern);
        let mut tag_ids = filter.tag_ids.clone();
        tag_ids.sort();
        tag_ids.dedup();
        let tags = serde_json::to_string(&tag_ids).expect("ids serialize");
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM clients
             WHERE (archived_at IS NOT NULL) = :archived
               AND (:status IS NULL OR status = :status)
               AND (:query IS NULL OR sort_key LIKE :query ESCAPE '\\')
               AND (SELECT count(*) FROM taggings t
                    WHERE t.owner_type = 'client' AND t.owner_id = clients.id
                      AND t.tag_id IN (SELECT value FROM json_each(:tags)))
                   = json_array_length(:tags)
             ORDER BY sort_key, id"
        ))?;
        let rows = stmt.query_map(
            named_params! {
                ":archived": filter.archived,
                ":status": filter.status,
                ":query": query,
                ":tags": tags,
            },
            from_row,
        )?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn archive(&self, id: Id) -> CoreResult<Client> {
        self.conn.execute(
            "UPDATE clients SET archived_at = ?2 WHERE id = ?1 AND archived_at IS NULL",
            (id, self.clock.now()),
        )?;
        self.get(id)
    }

    pub fn restore(&self, id: Id) -> CoreResult<Client> {
        self.conn
            .execute("UPDATE clients SET archived_at = NULL WHERE id = ?1", [id])?;
        self.get(id)
    }

    pub fn delete_impact(&self, id: Id) -> CoreResult<DeleteImpact> {
        self.get(id)?;
        let impact = self.conn.query_row(
            "WITH owners (type, id) AS (
                 SELECT 'client', ?1
                 UNION ALL SELECT 'contact', id FROM contacts WHERE client_id = ?1
             )
             SELECT
                 (SELECT count(*) FROM contacts WHERE client_id = ?1),
                 (SELECT count(*) FROM contact_methods x
                  JOIN owners o ON x.owner_type = o.type AND x.owner_id = o.id),
                 (SELECT count(*) FROM custom_fields x
                  JOIN owners o ON x.owner_type = o.type AND x.owner_id = o.id),
                 (SELECT count(*) FROM notes x
                  JOIN owners o ON x.owner_type = o.type AND x.owner_id = o.id),
                 (SELECT count(*) FROM projects WHERE client_id = ?1)",
            [id],
            |r| {
                Ok(DeleteImpact {
                    contacts: r.get(0)?,
                    projects: r.get(4)?,
                    contact_methods: r.get(1)?,
                    custom_fields: r.get(2)?,
                    notes: r.get(3)?,
                })
            },
        )?;
        Ok(impact)
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        match self
            .conn
            .execute("DELETE FROM clients WHERE id = ?1", [id])?
        {
            0 => Err(not_found(id)),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::domain::FakeClock;
    use rstest::rstest;

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

        fn repo(&self) -> ClientRepo<'_> {
            ClientRepo::new(self.db.conn(), &self.clock)
        }
    }

    fn person(last: &str, first: &str, middle: &str) -> ClientInput {
        let opt = |s: &str| (!s.is_empty()).then(|| s.to_owned());
        ClientInput {
            kind: ClientKind::Person,
            last_name: opt(last),
            first_name: opt(first),
            middle_name: opt(middle),
            company_name: None,
            legal_name: None,
            status: ClientStatus::Active,
            default_currency: None,
            default_rate_minor: None,
        }
    }

    fn company(name: &str) -> ClientInput {
        ClientInput {
            kind: ClientKind::Company,
            company_name: Some(name.to_owned()),
            ..person("", "", "")
        }
    }

    #[test]
    fn person_without_last_and_first_name_is_invalid() {
        let err = person("", "  ", "Иванович").normalized().unwrap_err();
        assert_eq!(err, CoreError::validation("firstName", "required"));
    }

    #[test]
    fn person_needs_only_one_of_last_or_first_name() {
        assert!(person("Иванов", "", "").normalized().is_ok());
        assert!(person("", "Иван", "").normalized().is_ok());
    }

    #[test]
    fn company_without_name_is_invalid() {
        let err = company("   ").normalized().unwrap_err();
        assert_eq!(err, CoreError::validation("companyName", "required"));
    }

    #[test]
    fn negative_rate_is_invalid() {
        let input = ClientInput {
            default_rate_minor: Some(-1),
            ..company("Acme")
        };
        assert_eq!(
            input.normalized().unwrap_err(),
            CoreError::validation("defaultRateMinor", "min")
        );
    }

    #[test]
    fn normalization_trims_and_drops_blank_fields() {
        let input = ClientInput {
            legal_name: Some("  ".into()),
            ..company("  Ромашка ")
        };
        let n = input.normalized().unwrap();
        assert_eq!(n.company_name.as_deref(), Some("Ромашка"));
        assert_eq!(n.legal_name, None);
    }

    #[test]
    fn normalization_drops_fields_of_the_other_kind() {
        let input = ClientInput {
            last_name: Some("Иванов".into()),
            ..company("Ромашка")
        };
        assert_eq!(input.normalized().unwrap().last_name, None);

        let input = ClientInput {
            company_name: Some("Ромашка".into()),
            ..person("Иванов", "Иван", "")
        };
        assert_eq!(input.normalized().unwrap().company_name, None);
    }

    #[rstest]
    #[case(person("Иванов", "Иван", "Иванович"), "Иванов Иван Иванович")]
    #[case(person("Иванов", "Иван", ""), "Иванов Иван")]
    #[case(person("", "Иван", ""), "Иван")]
    #[case(ClientInput { legal_name: Some("ООО «Ромашка»".into()), ..company("Ромашка") }, "Ромашка")]
    fn display_name(#[case] input: ClientInput, #[case] expected: &str) {
        assert_eq!(input.normalized().unwrap().display_name(), expected);
    }

    #[test]
    fn create_and_get() {
        let f = Fixture::new();
        let input = ClientInput {
            default_currency: Some("RUB".parse().unwrap()),
            default_rate_minor: Some(250_000),
            ..person(" Иванов ", "Иван", "")
        };
        let created = f.repo().create(input).unwrap();
        assert_eq!(created.display_name, "Иванов Иван");
        assert_eq!(created.input.last_name.as_deref(), Some("Иванов"));
        assert_eq!(created.created_at, Timestamp::from_millis(T0));
        assert_eq!(created.updated_at, Timestamp::from_millis(T0));
        assert_eq!(created.archived_at, None);
        assert_eq!(f.repo().get(created.id).unwrap(), created);
    }

    #[test]
    fn create_rejects_invalid_input() {
        let f = Fixture::new();
        assert!(f.repo().create(company("")).is_err());
        assert!(f.repo().list(&ClientFilter::default()).unwrap().is_empty());
    }

    #[test]
    fn get_unknown_client_is_not_found() {
        let f = Fixture::new();
        let id = Id::new();
        let err = f.repo().get(id).unwrap_err();
        assert_eq!(
            err,
            CoreError::new("client.not_found").with("id", id.to_string())
        );
    }

    #[test]
    fn update_changes_fields_and_updated_at() {
        let f = Fixture::new();
        let c = f.repo().create(company("Ромашка")).unwrap();
        f.clock.advance_millis(1_000);
        let input = ClientInput {
            status: ClientStatus::Paused,
            ..company("Василёк")
        };
        let updated = f.repo().update(c.id, input).unwrap();
        assert_eq!(updated.display_name, "Василёк");
        assert_eq!(updated.input.status, ClientStatus::Paused);
        assert_eq!(updated.created_at, c.created_at);
        assert_eq!(updated.updated_at, Timestamp::from_millis(T0 + 1_000));
        assert_eq!(f.repo().get(c.id).unwrap(), updated);
    }

    #[test]
    fn update_validates_and_reports_missing_client() {
        let f = Fixture::new();
        let c = f.repo().create(company("Ромашка")).unwrap();
        assert!(f.repo().update(c.id, company("")).is_err());
        assert_eq!(f.repo().get(c.id).unwrap().display_name, "Ромашка");
        let err = f.repo().update(Id::new(), company("X")).unwrap_err();
        assert_eq!(err.code, "client.not_found");
    }

    fn names(clients: &[Client]) -> Vec<&str> {
        clients.iter().map(|c| c.display_name.as_str()).collect()
    }

    #[test]
    fn list_sorts_by_name_with_cyrillic() {
        let f = Fixture::new();
        for name in ["Яковлев", "ёлкин", "борисов", "Абрамов", "Ежов", "acme"]
        {
            f.repo().create(company(name)).unwrap();
        }
        let all = f.repo().list(&ClientFilter::default()).unwrap();
        assert_eq!(
            names(&all),
            ["acme", "Абрамов", "борисов", "Ежов", "ёлкин", "Яковлев"]
        );
    }

    #[test]
    fn list_filters_by_status_and_archived() {
        let f = Fixture::new();
        let repo = f.repo();
        repo.create(company("Active")).unwrap();
        repo.create(ClientInput {
            status: ClientStatus::Lead,
            ..company("Lead")
        })
        .unwrap();
        let gone = repo.create(company("Gone")).unwrap();
        repo.archive(gone.id).unwrap();

        let active = repo.list(&ClientFilter::default()).unwrap();
        assert_eq!(names(&active), ["Active", "Lead"]);

        let leads = repo
            .list(&ClientFilter {
                status: Some(ClientStatus::Lead),
                ..ClientFilter::default()
            })
            .unwrap();
        assert_eq!(names(&leads), ["Lead"]);

        let archived = repo
            .list(&ClientFilter {
                archived: true,
                ..ClientFilter::default()
            })
            .unwrap();
        assert_eq!(names(&archived), ["Gone"]);
    }

    #[test]
    fn list_searches_case_insensitively_in_cyrillic() {
        let f = Fixture::new();
        f.repo().create(company("ООО Ромашка")).unwrap();
        f.repo().create(company("Василёк")).unwrap();
        let search = |q: &str| {
            let filter = ClientFilter {
                query: Some(q.into()),
                ..ClientFilter::default()
            };
            names(&f.repo().list(&filter).unwrap())
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        assert_eq!(search("ромаш"), ["ООО Ромашка"]);
        assert_eq!(search("ВАСИЛЕК"), ["Василёк"]);
        assert_eq!(search("  "), ["Василёк", "ООО Ромашка"]);
        // LIKE wildcards are matched literally.
        assert!(search("%").is_empty());
    }

    #[test]
    fn archive_and_restore() {
        let f = Fixture::new();
        let c = f.repo().create(company("Ромашка")).unwrap();
        f.clock.advance_millis(5);
        let archived = f.repo().archive(c.id).unwrap();
        assert_eq!(archived.archived_at, Some(Timestamp::from_millis(T0 + 5)));

        // Archiving again keeps the original time.
        f.clock.advance_millis(5);
        assert_eq!(f.repo().archive(c.id).unwrap(), archived);

        let restored = f.repo().restore(c.id).unwrap();
        assert_eq!(restored.archived_at, None);
        assert_eq!(
            f.repo().archive(Id::new()).unwrap_err().code,
            "client.not_found"
        );
        assert_eq!(
            f.repo().restore(Id::new()).unwrap_err().code,
            "client.not_found"
        );
    }

    #[test]
    fn delete_removes_client() {
        let f = Fixture::new();
        let c = f.repo().create(company("Ромашка")).unwrap();
        f.repo().delete(c.id).unwrap();
        assert_eq!(f.repo().get(c.id).unwrap_err().code, "client.not_found");
        assert_eq!(f.repo().delete(c.id).unwrap_err().code, "client.not_found");
    }

    #[test]
    fn serializes_flat_camel_case() {
        let f = Fixture::new();
        let c = f.repo().create(company("Acme")).unwrap();
        let json = serde_json::to_value(&c).unwrap();
        assert_eq!(json["kind"], "company");
        assert_eq!(json["companyName"], "Acme");
        assert_eq!(json["displayName"], "Acme");
        assert_eq!(json["status"], "active");
        assert!(json["archivedAt"].is_null());
    }
}
