//! Payments (FR-FN-6): to an invoice (partial payments allowed) or to a
//! project without one. Invoice statuses follow from them.

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use super::invoices::{InvoiceRepo, StoredStatus};
use crate::domain::{Clock, Id, Money, Timestamp};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInput {
    /// The invoice paid; its currency, client and project apply.
    pub invoice_id: Option<Id>,
    /// Without an invoice: the project paid for.
    pub project_id: Option<Id>,
    pub amount: Money,
    /// `YYYY-MM-DD`.
    pub paid_on: String,
    pub method: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    pub id: Id,
    pub invoice_id: Option<Id>,
    pub invoice_number: Option<String>,
    pub project_id: Option<Id>,
    pub project_name: Option<String>,
    pub client_id: Id,
    pub client_name: String,
    pub amount: Money,
    pub paid_on: String,
    pub method: String,
    pub note: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// A saved payment; `overpaid` warns that the invoice got more than due.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedPayment {
    pub payment: Payment,
    pub overpaid: Option<Money>,
}

pub struct PaymentRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> PaymentRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    /// Checks the input and works out the client and project paid for.
    fn resolve(&self, input: PaymentInput) -> CoreResult<(PaymentInput, Id)> {
        if input.amount.amount_minor <= 0 {
            return Err(CoreError::validation("amount", "min"));
        }
        if !crate::domain::date::is_iso_date(&input.paid_on) {
            return Err(CoreError::validation("paidOn", "date"));
        }
        let (client, project) = match input.invoice_id {
            Some(invoice) => {
                let inv = InvoiceRepo::new(self.conn, self.clock).get(invoice)?;
                if inv.stored_status != StoredStatus::Sent {
                    return Err(CoreError::new("payment.invoice_not_sent"));
                }
                if inv.currency != input.amount.currency {
                    return Err(CoreError::new("payment.currency_mismatch"));
                }
                (inv.client_id, inv.project_id)
            }
            None => {
                let project = input
                    .project_id
                    .ok_or_else(|| CoreError::validation("projectId", "required"))?;
                let client: Id = self
                    .conn
                    .query_row(
                        "SELECT client_id FROM projects WHERE id = ?1",
                        [project],
                        |r| r.get(0),
                    )
                    .optional()?
                    .ok_or_else(|| CoreError::not_found("project", project))?;
                (client, Some(project))
            }
        };
        Ok((
            PaymentInput {
                project_id: project,
                method: input.method.trim().to_owned(),
                note: input.note.trim().to_owned(),
                ..input
            },
            client,
        ))
    }

    /// The payment with a warning if its invoice is now overpaid.
    fn saved(&self, id: Id) -> CoreResult<SavedPayment> {
        let payment = self.get(id)?;
        let overpaid = match payment.invoice_id {
            Some(invoice) => {
                let due = InvoiceRepo::new(self.conn, self.clock)
                    .get(invoice)?
                    .totals
                    .due;
                (due.amount_minor < 0).then(|| Money::new(-due.amount_minor, due.currency))
            }
            None => None,
        };
        Ok(SavedPayment { payment, overpaid })
    }

    pub fn create(&self, input: PaymentInput) -> CoreResult<SavedPayment> {
        let (p, client) = self.resolve(input)?;
        let id = Id::new();
        self.conn.execute(
            "INSERT INTO payments (id, invoice_id, project_id, client_id, amount_minor, currency,
                 paid_on, method, note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            rusqlite::params![
                id,
                p.invoice_id,
                p.project_id,
                client,
                p.amount.amount_minor,
                p.amount.currency,
                p.paid_on,
                p.method,
                p.note,
                self.clock.now(),
            ],
        )?;
        self.saved(id)
    }

    pub fn update(&self, id: Id, input: PaymentInput) -> CoreResult<SavedPayment> {
        self.get(id)?;
        let (p, client) = self.resolve(input)?;
        self.conn.execute(
            "UPDATE payments SET invoice_id = ?2, project_id = ?3, client_id = ?4,
                 amount_minor = ?5, currency = ?6, paid_on = ?7, method = ?8, note = ?9,
                 updated_at = ?10
             WHERE id = ?1",
            rusqlite::params![
                id,
                p.invoice_id,
                p.project_id,
                client,
                p.amount.amount_minor,
                p.amount.currency,
                p.paid_on,
                p.method,
                p.note,
                self.clock.now(),
            ],
        )?;
        self.saved(id)
    }

    pub fn get(&self, id: Id) -> CoreResult<Payment> {
        self.conn
            .query_row(&format!("{SELECT} WHERE pm.id = ?1"), [id], from_row)
            .optional()?
            .ok_or_else(|| CoreError::not_found("payment", id))
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        if self
            .conn
            .execute("DELETE FROM payments WHERE id = ?1", [id])?
            == 0
        {
            return Err(CoreError::not_found("payment", id));
        }
        Ok(())
    }

    /// An invoice's payments, oldest first.
    pub fn for_invoice(&self, invoice_id: Id) -> CoreResult<Vec<Payment>> {
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT} WHERE pm.invoice_id = ?1 ORDER BY pm.paid_on, pm.created_at"
        ))?;
        let rows = stmt.query_map([invoice_id], from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// What a project received (with and without invoices), per currency.
    pub fn project_income(&self, project_id: Id) -> CoreResult<Vec<Money>> {
        let mut stmt = self.conn.prepare(
            "SELECT currency, sum(amount_minor) FROM payments WHERE project_id = ?1
             GROUP BY currency ORDER BY currency",
        )?;
        let rows = stmt.query_map([project_id], |r| Ok(Money::new(r.get(1)?, r.get(0)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

const SELECT: &str = "SELECT pm.id, pm.invoice_id, i.number AS invoice_number, pm.project_id,
        p.name AS project_name, pm.client_id, c.display_name AS client_name, pm.amount_minor,
        pm.currency, pm.paid_on, pm.method, pm.note, pm.created_at, pm.updated_at
     FROM payments pm
     JOIN clients c ON c.id = pm.client_id
     LEFT JOIN invoices i ON i.id = pm.invoice_id
     LEFT JOIN projects p ON p.id = pm.project_id";

fn from_row(r: &Row) -> rusqlite::Result<Payment> {
    Ok(Payment {
        id: r.get("id")?,
        invoice_id: r.get("invoice_id")?,
        invoice_number: r.get("invoice_number")?,
        project_id: r.get("project_id")?,
        project_name: r.get("project_name")?,
        client_id: r.get("client_id")?,
        client_name: r.get("client_name")?,
        amount: Money::new(r.get("amount_minor")?, r.get("currency")?),
        paid_on: r.get("paid_on")?,
        method: r.get("method")?,
        note: r.get("note")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::finance::invoices::{Discount, InvoiceInput, InvoiceItemInput, InvoiceStatus};
    use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};

    fn money(minor: i64, code: &str) -> Money {
        Money::new(minor, code.parse().unwrap())
    }

    struct Fixture {
        db: Db,
        clock: FakeClock,
        project: Id,
        invoice: Id,
    }

    impl Fixture {
        /// A sent invoice of 1 000.00 ₽ for project "Сайт".
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
            let invoice = InvoiceRepo::new(db.conn(), &clock)
                .create(InvoiceInput {
                    number: String::new(),
                    client_id: client,
                    project_id: Some(project),
                    currency: "RUB".parse().unwrap(),
                    issue_date: "2026-03-01".into(),
                    due_date: None,
                    status: StoredStatus::Sent,
                    items: vec![InvoiceItemInput {
                        description: "Сайт".into(),
                        quantity_milli: 1000,
                        unit: String::new(),
                        price: money(100_000, "RUB"),
                    }],
                    discount: Discount::None,
                    tax_bp: 0,
                    notes_md: String::new(),
                    language: "ru".into(),
                    show_signature: None,
                })
                .unwrap()
                .id;
            Self {
                db,
                clock,
                project,
                invoice,
            }
        }

        fn repo(&self) -> PaymentRepo<'_> {
            PaymentRepo::new(self.db.conn(), &self.clock)
        }

        fn status(&self) -> InvoiceStatus {
            InvoiceRepo::new(self.db.conn(), &self.clock)
                .get(self.invoice)
                .unwrap()
                .status
        }

        fn to_invoice(&self, minor: i64) -> PaymentInput {
            PaymentInput {
                invoice_id: Some(self.invoice),
                project_id: None,
                amount: money(minor, "RUB"),
                paid_on: "2026-03-05".into(),
                method: " Перевод ".into(),
                note: String::new(),
            }
        }
    }

    #[test]
    fn partial_and_full_payments() {
        let f = Fixture::new();
        let first = f.repo().create(f.to_invoice(40_000)).unwrap();
        assert_eq!(first.overpaid, None);
        let p = &first.payment;
        assert_eq!(
            (p.invoice_number.as_deref(), p.project_name.as_deref()),
            (Some("INV-2026-001"), Some("Сайт"))
        );
        assert_eq!(
            (p.client_name.as_str(), p.method.as_str()),
            ("Ромашка", "Перевод")
        );
        assert_eq!(f.status(), InvoiceStatus::PartiallyPaid);
        f.repo().create(f.to_invoice(60_000)).unwrap();
        assert_eq!(f.status(), InvoiceStatus::Paid);
        assert_eq!(f.repo().for_invoice(f.invoice).unwrap().len(), 2);
    }

    #[test]
    fn overpayment_is_a_warning() {
        let f = Fixture::new();
        f.repo().create(f.to_invoice(90_000)).unwrap();
        let saved = f.repo().create(f.to_invoice(30_000)).unwrap();
        assert_eq!(saved.overpaid, Some(money(20_000, "RUB")));
        assert_eq!(f.status(), InvoiceStatus::Paid);
    }

    #[test]
    fn deleting_or_editing_recomputes_the_status() {
        let f = Fixture::new();
        let full = f.repo().create(f.to_invoice(100_000)).unwrap().payment;
        assert_eq!(f.status(), InvoiceStatus::Paid);
        f.repo().update(full.id, f.to_invoice(10_000)).unwrap();
        assert_eq!(f.status(), InvoiceStatus::PartiallyPaid);
        f.repo().delete(full.id).unwrap();
        assert_eq!(f.status(), InvoiceStatus::Sent);
        assert_eq!(
            f.repo().get(full.id).unwrap_err(),
            CoreError::not_found("payment", full.id)
        );
    }

    #[test]
    fn payments_without_an_invoice_count_for_the_project() {
        let f = Fixture::new();
        f.repo().create(f.to_invoice(100_000)).unwrap();
        let direct = f
            .repo()
            .create(PaymentInput {
                invoice_id: None,
                project_id: Some(f.project),
                amount: money(5_000, "USD"),
                paid_on: "2026-03-07".into(),
                method: String::new(),
                note: "Аванс".into(),
            })
            .unwrap()
            .payment;
        assert_eq!(direct.invoice_id, None);
        assert_eq!(direct.client_name, "Ромашка");
        assert_eq!(
            f.repo().project_income(f.project).unwrap(),
            [money(100_000, "RUB"), money(5_000, "USD")]
        );
    }

    #[test]
    fn validates_payments() {
        let f = Fixture::new();
        let check = |input: PaymentInput, err: CoreError| {
            assert_eq!(f.repo().create(input).unwrap_err(), err)
        };
        check(
            PaymentInput {
                amount: money(0, "RUB"),
                ..f.to_invoice(0)
            },
            CoreError::validation("amount", "min"),
        );
        check(
            PaymentInput {
                paid_on: "вчера".into(),
                ..f.to_invoice(1)
            },
            CoreError::validation("paidOn", "date"),
        );
        check(
            PaymentInput {
                amount: money(1, "USD"),
                ..f.to_invoice(1)
            },
            CoreError::new("payment.currency_mismatch"),
        );
        check(
            PaymentInput {
                invoice_id: None,
                ..f.to_invoice(1)
            },
            CoreError::validation("projectId", "required"),
        );
        // Drafts and cancelled invoices cannot be paid.
        f.db.conn()
            .execute("UPDATE invoices SET status = 'draft'", [])
            .unwrap();
        check(f.to_invoice(1), CoreError::new("payment.invoice_not_sent"));
    }
}
