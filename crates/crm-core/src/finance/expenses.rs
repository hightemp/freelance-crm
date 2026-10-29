//! Expenses (FR-FN-7): amount, currency, date, category and description,
//! optionally for a project, with receipts attached as files.

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use super::payments::PaymentRepo;
use crate::domain::{Clock, Currency, Id, Money, Timestamp};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseInput {
    pub project_id: Option<Id>,
    pub category: String,
    pub amount: Money,
    /// `YYYY-MM-DD`.
    pub spent_on: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Expense {
    pub id: Id,
    pub project_id: Option<Id>,
    pub project_name: Option<String>,
    pub category: String,
    pub amount: Money,
    pub spent_on: String,
    pub description: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseFilter {
    pub project_id: Option<Id>,
    /// Inclusive dates.
    pub from: Option<String>,
    pub to: Option<String>,
    pub category: Option<String>,
}

/// A project's money in one currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Profit {
    pub currency: Currency,
    #[specta(type = i32)]
    pub income_minor: i64,
    #[specta(type = i32)]
    pub expenses_minor: i64,
    #[specta(type = i32)]
    pub profit_minor: i64,
}

pub struct ExpenseRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> ExpenseRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    fn validate(&self, input: ExpenseInput) -> CoreResult<ExpenseInput> {
        let category = input.category.trim().to_owned();
        if category.is_empty() {
            return Err(CoreError::validation("category", "required"));
        }
        if input.amount.amount_minor <= 0 {
            return Err(CoreError::validation("amount", "min"));
        }
        if !crate::domain::date::is_iso_date(&input.spent_on) {
            return Err(CoreError::validation("spentOn", "date"));
        }
        if let Some(project) = input.project_id {
            crate::db::ensure_exists(self.conn, "projects", "project", project)?;
        }
        Ok(ExpenseInput {
            category,
            description: input.description.trim().to_owned(),
            ..input
        })
    }

    pub fn create(&self, input: ExpenseInput) -> CoreResult<Expense> {
        let e = self.validate(input)?;
        let id = Id::new();
        self.conn.execute(
            "INSERT INTO expenses (id, project_id, category, category_key, amount_minor, currency,
                 spent_on, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            rusqlite::params![
                id,
                e.project_id,
                e.category,
                fold(&e.category),
                e.amount.amount_minor,
                e.amount.currency,
                e.spent_on,
                e.description,
                self.clock.now(),
            ],
        )?;
        self.get(id)
    }

    pub fn update(&self, id: Id, input: ExpenseInput) -> CoreResult<Expense> {
        self.get(id)?;
        let e = self.validate(input)?;
        self.conn.execute(
            "UPDATE expenses SET project_id = ?2, category = ?3, category_key = ?4,
                 amount_minor = ?5, currency = ?6, spent_on = ?7, description = ?8,
                 updated_at = ?9
             WHERE id = ?1",
            rusqlite::params![
                id,
                e.project_id,
                e.category,
                fold(&e.category),
                e.amount.amount_minor,
                e.amount.currency,
                e.spent_on,
                e.description,
                self.clock.now(),
            ],
        )?;
        self.get(id)
    }

    pub fn get(&self, id: Id) -> CoreResult<Expense> {
        self.conn
            .query_row(&format!("{SELECT} WHERE e.id = ?1"), [id], from_row)
            .optional()?
            .ok_or_else(|| CoreError::not_found("expense", id))
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        if self
            .conn
            .execute("DELETE FROM expenses WHERE id = ?1", [id])?
            == 0
        {
            return Err(CoreError::not_found("expense", id));
        }
        Ok(())
    }

    /// Newest first.
    pub fn list(&self, filter: &ExpenseFilter) -> CoreResult<Vec<Expense>> {
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT}
             WHERE (?1 IS NULL OR e.project_id = ?1)
               AND (?2 IS NULL OR e.spent_on >= ?2)
               AND (?3 IS NULL OR e.spent_on <= ?3)
               AND (?4 IS NULL OR e.category_key = ?4)
             ORDER BY e.spent_on DESC, e.created_at DESC"
        ))?;
        let rows = stmt.query_map(
            rusqlite::params![
                filter.project_id,
                filter.from,
                filter.to,
                filter.category.as_deref().map(fold)
            ],
            from_row,
        )?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Categories used so far, alphabetically, for suggestions.
    pub fn categories(&self) -> CoreResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.category FROM expenses e
             WHERE e.rowid = (SELECT min(x.rowid) FROM expenses x WHERE x.category_key = e.category_key)
             ORDER BY e.category_key",
        )?;
        let rows = stmt.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Payments received minus expenses, per currency.
    pub fn project_profit(&self, project_id: Id) -> CoreResult<Vec<Profit>> {
        let income = PaymentRepo::new(self.conn, self.clock).project_income(project_id)?;
        let mut stmt = self.conn.prepare(
            "SELECT currency, sum(amount_minor) FROM expenses WHERE project_id = ?1
             GROUP BY currency",
        )?;
        let spent = stmt
            .query_map([project_id], |r| Ok(Money::new(r.get(1)?, r.get(0)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        // Currency code → (currency, income, expenses).
        let mut by_code: std::collections::BTreeMap<String, (Currency, i64, i64)> =
            Default::default();
        for m in &income {
            by_code
                .entry(m.currency.code().to_owned())
                .or_insert((m.currency, 0, 0))
                .1 += m.amount_minor;
        }
        for m in &spent {
            by_code
                .entry(m.currency.code().to_owned())
                .or_insert((m.currency, 0, 0))
                .2 += m.amount_minor;
        }
        Ok(by_code
            .into_values()
            .map(|(currency, income_minor, expenses_minor)| Profit {
                currency,
                income_minor,
                expenses_minor,
                profit_minor: income_minor - expenses_minor,
            })
            .collect())
    }
}

use crate::domain::text::fold;

const SELECT: &str =
    "SELECT e.id, e.project_id, p.name AS project_name, e.category, e.amount_minor,
        e.currency, e.spent_on, e.description, e.created_at, e.updated_at
     FROM expenses e LEFT JOIN projects p ON p.id = e.project_id";

fn from_row(r: &Row) -> rusqlite::Result<Expense> {
    Ok(Expense {
        id: r.get("id")?,
        project_id: r.get("project_id")?,
        project_name: r.get("project_name")?,
        category: r.get("category")?,
        amount: Money::new(r.get("amount_minor")?, r.get("currency")?),
        spent_on: r.get("spent_on")?,
        description: r.get("description")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::{EntityRef, FakeClock};
    use crate::finance::payments::PaymentInput;
    use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};

    fn money(minor: i64, code: &str) -> Money {
        Money::new(minor, code.parse().unwrap())
    }

    struct Fixture {
        db: Db,
        clock: FakeClock,
        project: Id,
    }

    impl Fixture {
        fn new() -> Self {
            let db = Db::open_in_memory().unwrap();
            let clock = FakeClock::at(Timestamp::from_millis(1_773_000_000_000));
            let client = ClientRepo::new(db.conn(), &clock)
                .create(ClientInput {
                    kind: ClientKind::Company,
                    last_name: None,
                    first_name: None,
                    middle_name: None,
                    company_name: Some("Ромашка".into()),
                    legal_name: None,
                    status: ClientStatus::Active,
                    default_currency: None,
                    default_rate_minor: None,
                })
                .unwrap()
                .id;
            let project = ProjectRepo::new(db.conn(), &clock)
                .create(ProjectInput {
                    client_id: client,
                    name: "Сайт".into(),
                    description_md: String::new(),
                    status: ProjectStatus::Active,
                    billing_type: BillingType::Fixed,
                    currency: Some("RUB".parse().unwrap()),
                    rate_minor: None,
                    budget_minor: Some(1),
                    start_date: None,
                    deadline: None,
                })
                .unwrap()
                .id;
            Self { db, clock, project }
        }

        fn repo(&self) -> ExpenseRepo<'_> {
            ExpenseRepo::new(self.db.conn(), &self.clock)
        }

        fn input(&self, category: &str, amount: Money, date: &str) -> ExpenseInput {
            ExpenseInput {
                project_id: Some(self.project),
                category: category.into(),
                amount,
                spent_on: date.into(),
                description: "Покупка".into(),
            }
        }
    }

    #[test]
    fn crud() {
        let f = Fixture::new();
        let e = f
            .repo()
            .create(f.input(" Хостинг ", money(50_000, "RUB"), "2026-03-02"))
            .unwrap();
        assert_eq!(
            (e.category.as_str(), e.project_name.as_deref()),
            ("Хостинг", Some("Сайт"))
        );
        assert_eq!(f.repo().get(e.id).unwrap(), e);
        f.clock.advance_millis(1000);
        let mut changed = f.input("Хостинг", money(60_000, "RUB"), "2026-03-03");
        changed.project_id = None;
        let updated = f.repo().update(e.id, changed).unwrap();
        assert_eq!(
            (updated.amount, updated.project_id),
            (money(60_000, "RUB"), None)
        );
        assert!(updated.updated_at > e.updated_at);
        f.repo().delete(e.id).unwrap();
        assert_eq!(
            f.repo().get(e.id).unwrap_err(),
            CoreError::not_found("expense", e.id)
        );
    }

    #[test]
    fn validates() {
        let f = Fixture::new();
        let check = |input: ExpenseInput, err: CoreError| {
            assert_eq!(f.repo().create(input).unwrap_err(), err)
        };
        check(
            f.input(" ", money(1, "RUB"), "2026-03-02"),
            CoreError::validation("category", "required"),
        );
        check(
            f.input("Софт", money(0, "RUB"), "2026-03-02"),
            CoreError::validation("amount", "min"),
        );
        check(
            f.input("Софт", money(1, "RUB"), "2026-3-2"),
            CoreError::validation("spentOn", "date"),
        );
        let missing = Id::new();
        check(
            ExpenseInput {
                project_id: Some(missing),
                ..f.input("Софт", money(1, "RUB"), "2026-03-02")
            },
            CoreError::not_found("project", missing),
        );
    }

    #[test]
    fn lists_and_suggests_categories() {
        let f = Fixture::new();
        f.repo()
            .create(f.input("Софт", money(1_000, "USD"), "2026-03-01"))
            .unwrap();
        f.repo()
            .create(f.input("хостинг", money(50_000, "RUB"), "2026-03-10"))
            .unwrap();
        f.repo()
            .create(f.input("Хостинг", money(50_000, "RUB"), "2026-04-10"))
            .unwrap();
        // Case-insensitively distinct, keeping the first spelling.
        assert_eq!(f.repo().categories().unwrap(), ["Софт", "хостинг"]);
        let march = ExpenseFilter {
            from: Some("2026-03-01".into()),
            to: Some("2026-03-31".into()),
            ..Default::default()
        };
        assert_eq!(
            f.repo()
                .list(&march)
                .unwrap()
                .iter()
                .map(|e| e.spent_on.as_str())
                .collect::<Vec<_>>(),
            ["2026-03-10", "2026-03-01"]
        );
        let hosting = ExpenseFilter {
            category: Some("ХОСТИНГ".into()),
            ..Default::default()
        };
        assert_eq!(f.repo().list(&hosting).unwrap().len(), 2);
    }

    #[test]
    fn receipts_are_attachments() {
        let f = Fixture::new();
        let e = f
            .repo()
            .create(f.input("Софт", money(1_000, "USD"), "2026-03-01"))
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let vault = crate::vault::Vault::new(
            crate::vault::crypto::KdfParams {
                m_kib: 64,
                t: 1,
                p: 1,
            },
            std::sync::Arc::new(FakeClock::at(Timestamp::from_millis(0))),
        );
        let files =
            crate::files::attachments::Attachments::new(dir.path(), f.db.conn(), &f.clock, &vault);
        let owner = EntityRef::Expense(e.id);
        files.attach(owner, "чек.pdf", &b"%PDF"[..], false).unwrap();
        assert_eq!(files.list(owner).unwrap().len(), 1);
        f.repo().delete(e.id).unwrap();
        assert!(files.list(owner).unwrap().is_empty());
    }

    #[test]
    fn project_profit_is_income_minus_expenses_per_currency() {
        let f = Fixture::new();
        let payments = PaymentRepo::new(f.db.conn(), &f.clock);
        for amount in [money(300_000, "RUB"), money(10_000, "USD")] {
            payments
                .create(PaymentInput {
                    invoice_id: None,
                    project_id: Some(f.project),
                    amount,
                    paid_on: "2026-03-05".into(),
                    method: String::new(),
                    note: String::new(),
                })
                .unwrap();
        }
        f.repo()
            .create(f.input("Хостинг", money(50_000, "RUB"), "2026-03-02"))
            .unwrap();
        f.repo()
            .create(f.input("Софт", money(2_000, "EUR"), "2026-03-02"))
            .unwrap();
        let line = |c: &str, i, e| Profit {
            currency: c.parse().unwrap(),
            income_minor: i,
            expenses_minor: e,
            profit_minor: i - e,
        };
        assert_eq!(
            f.repo().project_profit(f.project).unwrap(),
            [
                line("EUR", 0, 2_000),
                line("RUB", 300_000, 50_000),
                line("USD", 10_000, 0)
            ]
        );
    }
}
