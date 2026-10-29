//! Projects of a client with a billing type, money and dates (FR-PR).

use rusqlite::{Connection, OptionalExtension, Row, named_params};
use serde::{Deserialize, Serialize};

use crate::clients::ClientRepo;
use crate::db::text_enum;
use crate::domain::date::is_iso_date;
use crate::domain::text::{clean, contains_pattern, fold};
use crate::domain::{Clock, Currency, Id, Timestamp};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Lead,
    Active,
    Paused,
    Completed,
    Cancelled,
}

text_enum!(ProjectStatus {
    Lead => "lead",
    Active => "active",
    Paused => "paused",
    Completed => "completed",
    Cancelled => "cancelled",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum BillingType {
    /// A fixed price: `budget_minor`.
    Fixed,
    /// Paid per hour: `rate_minor`.
    Hourly,
}

text_enum!(BillingType { Fixed => "fixed", Hourly => "hourly" });

/// Editable fields. Currency and rate left empty are inherited from the
/// client (then the app default currency) when saving.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInput {
    pub client_id: Id,
    pub name: String,
    pub description_md: String,
    pub status: ProjectStatus,
    pub billing_type: BillingType,
    pub currency: Option<Currency>,
    #[specta(type = Option<i32>)]
    pub rate_minor: Option<i64>,
    #[specta(type = Option<i32>)]
    pub budget_minor: Option<i64>,
    /// `YYYY-MM-DD`.
    pub start_date: Option<String>,
    /// `YYYY-MM-DD`, not before `start_date`.
    pub deadline: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: Id,
    pub client_id: Id,
    pub client_name: String,
    pub name: String,
    pub description_md: String,
    pub status: ProjectStatus,
    pub billing_type: BillingType,
    pub currency: Currency,
    #[specta(type = Option<i32>)]
    pub rate_minor: Option<i64>,
    #[specta(type = Option<i32>)]
    pub budget_minor: Option<i64>,
    pub start_date: Option<String>,
    pub deadline: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub archived_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFilter {
    pub query: Option<String>,
    pub status: Option<ProjectStatus>,
    pub client_id: Option<Id>,
    /// `false` lists active projects, `true` only archived ones.
    pub archived: bool,
}

const SELECT: &str = "SELECT p.id, p.client_id, c.display_name AS client_name, p.name,
    p.description_md, p.status, p.billing_type, p.currency, p.rate_minor, p.budget_minor,
    p.start_date, p.deadline, p.created_at, p.updated_at, p.archived_at
    FROM projects p JOIN clients c ON c.id = p.client_id";

fn from_row(row: &Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get("id")?,
        client_id: row.get("client_id")?,
        client_name: row.get("client_name")?,
        name: row.get("name")?,
        description_md: row.get("description_md")?,
        status: row.get("status")?,
        billing_type: row.get("billing_type")?,
        currency: row.get("currency")?,
        rate_minor: row.get("rate_minor")?,
        budget_minor: row.get("budget_minor")?,
        start_date: row.get("start_date")?,
        deadline: row.get("deadline")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        archived_at: row.get("archived_at")?,
    })
}

fn not_found(id: Id) -> CoreError {
    CoreError::not_found("project", id)
}

/// Input with inherited values filled in and the unused amount dropped.
struct Resolved {
    input: ProjectInput,
    currency: Currency,
}

fn date(value: Option<String>, field: &str) -> CoreResult<Option<String>> {
    match clean(value) {
        Some(d) if !is_iso_date(&d) => Err(CoreError::validation(field, "date")),
        other => Ok(other),
    }
}

fn amount(value: Option<i64>, field: &str) -> CoreResult<Option<i64>> {
    match value {
        None => Err(CoreError::validation(field, "required")),
        Some(v) if v < 0 => Err(CoreError::validation(field, "min")),
        some => Ok(some),
    }
}

pub struct ProjectRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> ProjectRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    fn resolve(&self, input: ProjectInput) -> CoreResult<Resolved> {
        let name = input.name.trim().to_owned();
        if name.is_empty() {
            return Err(CoreError::validation("name", "required"));
        }
        let client = ClientRepo::new(self.conn, self.clock).get(input.client_id)?;
        let (rate_minor, budget_minor) = match input.billing_type {
            BillingType::Fixed => (None, amount(input.budget_minor, "budgetMinor")?),
            BillingType::Hourly => (
                amount(
                    input.rate_minor.or(client.input.default_rate_minor),
                    "rateMinor",
                )?,
                None,
            ),
        };
        let start_date = date(input.start_date, "startDate")?;
        let deadline = date(input.deadline, "deadline")?;
        if let (Some(start), Some(end)) = (&start_date, &deadline)
            && end < start
        {
            return Err(CoreError::validation("deadline", "before_start"));
        }
        let currency = match input.currency.or(client.input.default_currency) {
            Some(c) => c,
            None => crate::settings::load_from(self.conn)?.default_currency,
        };
        Ok(Resolved {
            currency,
            input: ProjectInput {
                name,
                rate_minor,
                budget_minor,
                start_date,
                deadline,
                currency: Some(currency),
                ..input
            },
        })
    }

    pub fn create(&self, input: ProjectInput) -> CoreResult<Project> {
        let Resolved { input: p, currency } = self.resolve(input)?;
        let id = Id::new();
        self.conn.execute(
            "INSERT INTO projects (id, client_id, name, sort_key, description_md, status,
                 billing_type, currency, rate_minor, budget_minor, start_date, deadline,
                 created_at, updated_at)
             VALUES (:id, :client, :name, :sort_key, :description, :status, :billing,
                 :currency, :rate, :budget, :start, :deadline, :now, :now)",
            named_params! {
                ":id": id,
                ":client": p.client_id,
                ":sort_key": fold(&p.name),
                ":name": p.name,
                ":description": p.description_md,
                ":status": p.status,
                ":billing": p.billing_type,
                ":currency": currency,
                ":rate": p.rate_minor,
                ":budget": p.budget_minor,
                ":start": p.start_date,
                ":deadline": p.deadline,
                ":now": self.clock.now(),
            },
        )?;
        self.get(id)
    }

    pub fn get(&self, id: Id) -> CoreResult<Project> {
        self.conn
            .query_row(&format!("{SELECT} WHERE p.id = ?1"), [id], from_row)
            .optional()?
            .ok_or_else(|| not_found(id))
    }

    pub fn update(&self, id: Id, input: ProjectInput) -> CoreResult<Project> {
        let Resolved { input: p, currency } = self.resolve(input)?;
        let changed = self.conn.execute(
            "UPDATE projects SET client_id = :client, name = :name, sort_key = :sort_key,
                 description_md = :description, status = :status, billing_type = :billing,
                 currency = :currency, rate_minor = :rate, budget_minor = :budget,
                 start_date = :start, deadline = :deadline, updated_at = :now
             WHERE id = :id",
            named_params! {
                ":id": id,
                ":client": p.client_id,
                ":sort_key": fold(&p.name),
                ":name": p.name,
                ":description": p.description_md,
                ":status": p.status,
                ":billing": p.billing_type,
                ":currency": currency,
                ":rate": p.rate_minor,
                ":budget": p.budget_minor,
                ":start": p.start_date,
                ":deadline": p.deadline,
                ":now": self.clock.now(),
            },
        )?;
        if changed == 0 {
            return Err(not_found(id));
        }
        self.get(id)
    }

    pub fn list(&self, filter: &ProjectFilter) -> CoreResult<Vec<Project>> {
        let query = filter
            .query
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
            .map(contains_pattern);
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT}
             WHERE (p.archived_at IS NOT NULL) = :archived
               AND (:status IS NULL OR p.status = :status)
               AND (:client IS NULL OR p.client_id = :client)
               AND (:query IS NULL OR p.sort_key LIKE :query ESCAPE '\\')
             ORDER BY p.sort_key, p.id"
        ))?;
        let rows = stmt.query_map(
            named_params! {
                ":archived": filter.archived,
                ":status": filter.status,
                ":client": filter.client_id,
                ":query": query,
            },
            from_row,
        )?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn archive(&self, id: Id) -> CoreResult<Project> {
        self.conn.execute(
            "UPDATE projects SET archived_at = ?2 WHERE id = ?1 AND archived_at IS NULL",
            (id, self.clock.now()),
        )?;
        self.get(id)
    }

    pub fn restore(&self, id: Id) -> CoreResult<Project> {
        self.conn
            .execute("UPDATE projects SET archived_at = NULL WHERE id = ?1", [id])?;
        self.get(id)
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        match self
            .conn
            .execute("DELETE FROM projects WHERE id = ?1", [id])?
        {
            0 => Err(not_found(id)),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::settings::{self, Settings};

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

        fn repo(&self) -> ProjectRepo<'_> {
            ProjectRepo::new(self.db.conn(), &self.clock)
        }

        fn client(&self, name: &str, currency: Option<&str>, rate: Option<i64>) -> Id {
            let input = ClientInput {
                kind: ClientKind::Company,
                last_name: None,
                first_name: None,
                middle_name: None,
                company_name: Some(name.into()),
                legal_name: None,
                status: ClientStatus::Active,
                default_currency: currency.map(|c| c.parse().unwrap()),
                default_rate_minor: rate,
            };
            ClientRepo::new(self.db.conn(), &self.clock)
                .create(input)
                .unwrap()
                .id
        }
    }

    fn input(client_id: Id, name: &str) -> ProjectInput {
        ProjectInput {
            client_id,
            name: name.into(),
            description_md: String::new(),
            status: ProjectStatus::Active,
            billing_type: BillingType::Fixed,
            currency: Some("RUB".parse().unwrap()),
            rate_minor: None,
            budget_minor: Some(10_000_000),
            start_date: None,
            deadline: None,
        }
    }

    fn rub() -> Currency {
        "RUB".parse().unwrap()
    }

    #[test]
    fn create_and_get() {
        let f = Fixture::new();
        let client = f.client("Ромашка", None, None);
        let data = ProjectInput {
            name: "  Сайт  ".into(),
            description_md: "# Лендинг".into(),
            start_date: Some("2024-03-01".into()),
            deadline: Some("2024-04-30".into()),
            ..input(client, "")
        };
        let p = f.repo().create(data).unwrap();
        assert_eq!(p.name, "Сайт");
        assert_eq!(p.client_name, "Ромашка");
        assert_eq!(p.currency, rub());
        assert_eq!(p.budget_minor, Some(10_000_000));
        assert_eq!(p.deadline.as_deref(), Some("2024-04-30"));
        assert_eq!(p.created_at, Timestamp::from_millis(T0));
        assert_eq!(f.repo().get(p.id).unwrap(), p);
        assert_eq!(
            f.repo().get(Id::new()).unwrap_err().code,
            "project.not_found"
        );
    }

    #[test]
    fn project_needs_an_existing_client() {
        let f = Fixture::new();
        let missing = Id::new();
        assert_eq!(
            f.repo().create(input(missing, "X")).unwrap_err(),
            CoreError::not_found("client", missing)
        );
    }

    #[test]
    fn name_is_required() {
        let f = Fixture::new();
        let client = f.client("Ромашка", None, None);
        assert_eq!(
            f.repo().create(input(client, "  ")).unwrap_err(),
            CoreError::validation("name", "required")
        );
    }

    #[test]
    fn currency_and_rate_are_inherited_from_the_client() {
        let f = Fixture::new();
        let client = f.client("Ромашка", Some("EUR"), Some(5_000));
        let p = f
            .repo()
            .create(ProjectInput {
                billing_type: BillingType::Hourly,
                currency: None,
                rate_minor: None,
                budget_minor: None,
                ..input(client, "Поддержка")
            })
            .unwrap();
        assert_eq!(p.currency.code(), "EUR");
        assert_eq!(p.rate_minor, Some(5_000));

        // Explicit values win over the client's.
        let p = f
            .repo()
            .create(ProjectInput {
                billing_type: BillingType::Hourly,
                currency: Some(rub()),
                rate_minor: Some(300_000),
                budget_minor: None,
                ..input(client, "Аудит")
            })
            .unwrap();
        assert_eq!((p.currency, p.rate_minor), (rub(), Some(300_000)));
    }

    #[test]
    fn currency_falls_back_to_the_app_default() {
        let mut f = Fixture::new();
        settings::save(
            &mut f.db,
            &Settings {
                default_currency: "GBP".parse().unwrap(),
                ..Settings::default()
            },
        )
        .unwrap();
        let client = f.client("Ромашка", None, None);
        let p = f
            .repo()
            .create(ProjectInput {
                currency: None,
                ..input(client, "Сайт")
            })
            .unwrap();
        assert_eq!(p.currency.code(), "GBP");
    }

    #[test]
    fn fixed_needs_a_budget_and_hourly_a_rate() {
        let f = Fixture::new();
        let client = f.client("Ромашка", None, None);
        assert_eq!(
            f.repo()
                .create(ProjectInput {
                    budget_minor: None,
                    ..input(client, "Сайт")
                })
                .unwrap_err(),
            CoreError::validation("budgetMinor", "required")
        );
        assert_eq!(
            f.repo()
                .create(ProjectInput {
                    billing_type: BillingType::Hourly,
                    ..input(client, "Сайт")
                })
                .unwrap_err(),
            CoreError::validation("rateMinor", "required")
        );
        assert_eq!(
            f.repo()
                .create(ProjectInput {
                    budget_minor: Some(-1),
                    ..input(client, "Сайт")
                })
                .unwrap_err(),
            CoreError::validation("budgetMinor", "min")
        );
    }

    #[test]
    fn the_other_billing_amount_is_dropped() {
        let f = Fixture::new();
        let client = f.client("Ромашка", None, Some(5_000));
        let p = f
            .repo()
            .create(ProjectInput {
                rate_minor: Some(1),
                ..input(client, "Сайт")
            })
            .unwrap();
        assert_eq!((p.rate_minor, p.budget_minor), (None, Some(10_000_000)));
    }

    #[test]
    fn dates_are_validated() {
        let f = Fixture::new();
        let client = f.client("Ромашка", None, None);
        let with = |start: Option<&str>, deadline: Option<&str>| {
            f.repo().create(ProjectInput {
                start_date: start.map(Into::into),
                deadline: deadline.map(Into::into),
                ..input(client, "Сайт")
            })
        };
        assert_eq!(
            with(Some("2024-05-01"), Some("2024-04-30")).unwrap_err(),
            CoreError::validation("deadline", "before_start")
        );
        assert!(with(Some("2024-05-01"), Some("2024-05-01")).is_ok());
        assert!(with(None, Some("2024-05-01")).is_ok());
        assert_eq!(
            with(Some("2024-02-30"), None).unwrap_err(),
            CoreError::validation("startDate", "date")
        );
        assert_eq!(
            with(None, Some("soon")).unwrap_err(),
            CoreError::validation("deadline", "date")
        );
    }

    #[test]
    fn update_changes_fields() {
        let f = Fixture::new();
        let client = f.client("Ромашка", None, None);
        let other = f.client("Василёк", None, None);
        let p = f.repo().create(input(client, "Сайт")).unwrap();
        f.clock.advance_millis(1_000);
        let updated = f
            .repo()
            .update(
                p.id,
                ProjectInput {
                    status: ProjectStatus::Completed,
                    ..input(other, "Сайт 2.0")
                },
            )
            .unwrap();
        assert_eq!(updated.name, "Сайт 2.0");
        assert_eq!(updated.client_name, "Василёк");
        assert_eq!(updated.status, ProjectStatus::Completed);
        assert_eq!(updated.updated_at, Timestamp::from_millis(T0 + 1_000));
        assert_eq!(
            f.repo()
                .update(Id::new(), input(client, "X"))
                .unwrap_err()
                .code,
            "project.not_found"
        );
    }

    fn names(projects: &[Project]) -> Vec<&str> {
        projects.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn list_filters_and_sorts() {
        let f = Fixture::new();
        let a = f.client("А", None, None);
        let b = f.client("Б", None, None);
        f.repo().create(input(a, "Яндекс.Директ")).unwrap();
        f.repo().create(input(b, "ёлка")).unwrap();
        f.repo()
            .create(ProjectInput {
                status: ProjectStatus::Lead,
                ..input(a, "Аудит")
            })
            .unwrap();
        let old = f.repo().create(input(a, "Старый")).unwrap();
        f.repo().archive(old.id).unwrap();

        let list = |filter: ProjectFilter| names(&f.repo().list(&filter).unwrap()).join(",");
        assert_eq!(list(ProjectFilter::default()), "Аудит,ёлка,Яндекс.Директ");
        assert_eq!(
            list(ProjectFilter {
                client_id: Some(a),
                ..Default::default()
            }),
            "Аудит,Яндекс.Директ"
        );
        assert_eq!(
            list(ProjectFilter {
                status: Some(ProjectStatus::Lead),
                ..Default::default()
            }),
            "Аудит"
        );
        assert_eq!(
            list(ProjectFilter {
                query: Some("ЕЛК".into()),
                ..Default::default()
            }),
            "ёлка"
        );
        assert_eq!(
            list(ProjectFilter {
                archived: true,
                ..Default::default()
            }),
            "Старый"
        );
    }

    #[test]
    fn archive_restore_and_delete() {
        let f = Fixture::new();
        let client = f.client("Ромашка", None, None);
        let p = f.repo().create(input(client, "Сайт")).unwrap();
        f.clock.advance_millis(5);
        let archived = f.repo().archive(p.id).unwrap();
        assert_eq!(archived.archived_at, Some(Timestamp::from_millis(T0 + 5)));
        assert_eq!(f.repo().restore(p.id).unwrap().archived_at, None);
        f.repo().delete(p.id).unwrap();
        assert_eq!(f.repo().get(p.id).unwrap_err().code, "project.not_found");
        assert_eq!(f.repo().delete(p.id).unwrap_err().code, "project.not_found");
        assert_eq!(
            f.repo().archive(p.id).unwrap_err().code,
            "project.not_found"
        );
    }

    #[test]
    fn deleting_the_client_deletes_its_projects() {
        let f = Fixture::new();
        let client = f.client("Ромашка", None, None);
        f.repo().create(input(client, "Сайт")).unwrap();
        let clients = ClientRepo::new(f.db.conn(), &f.clock);
        assert_eq!(clients.delete_impact(client).unwrap().projects, 1);
        clients.delete(client).unwrap();
        assert!(f.repo().list(&ProjectFilter::default()).unwrap().is_empty());
    }
}
