//! Invoices from tracked time (FR-FN-4, FR-TT-5): billable entries not yet
//! billed become items at their hourly rate, grouped by task or as one
//! line, and are marked with the invoice. Cancelling or deleting the
//! invoice frees them again.

use serde::{Deserialize, Serialize};

use super::invoices::{
    Discount, Invoice, InvoiceInput, InvoiceItemInput, InvoiceRepo, StoredStatus,
};
use crate::domain::{Id, Money, Timestamp};
use crate::time_report::{TimeFilter, rated};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Grouping {
    /// One item per task (and rate); time without a task per project.
    ByTask,
    SingleLine,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FromTimeInput {
    pub client_id: Id,
    pub project_id: Option<Id>,
    /// Entries starting in `[from, to)`.
    pub from: Timestamp,
    pub to: Timestamp,
    pub grouping: Grouping,
    /// Description of the single line.
    pub description: String,
    /// Empty for the next number from the template.
    pub number: String,
    pub issue_date: String,
    pub due_date: Option<String>,
    #[specta(type = i32)]
    pub tax_bp: i64,
    pub language: String,
}

impl InvoiceRepo<'_> {
    /// Bills the client's unbilled time; durations are rounded up to
    /// `rounding_minutes` per entry.
    pub fn from_time(&self, input: FromTimeInput, rounding_minutes: u32) -> CoreResult<Invoice> {
        let entries = rated(
            self.conn(),
            TimeFilter {
                from: Some(input.from),
                to: Some(input.to),
                client_id: Some(input.client_id),
                project_id: input.project_id,
                billable: Some(true),
            },
            true,
        )?;
        if entries.is_empty() {
            return Err(CoreError::new("invoice.no_time"));
        }
        let rates: Vec<Money> = entries
            .iter()
            .map(|r| r.rate.ok_or_else(|| CoreError::new("invoice.no_rate")))
            .collect::<CoreResult<_>>()?;
        let currency = rates[0].currency;
        if rates.iter().any(|r| r.currency != currency) {
            return Err(CoreError::new("invoice.mixed_currencies"));
        }
        let unit = if input.language == "ru" { "ч" } else { "h" };

        // (description, rate) → seconds, in order of description.
        let mut groups: Vec<(String, Money, i64)> = Vec::new();
        for (r, rate) in entries.iter().zip(&rates) {
            let description = match input.grouping {
                Grouping::SingleLine => input.description.trim().to_owned(),
                Grouping::ByTask => {
                    let e = &r.entry;
                    match (&e.task_title, &e.project_name, &e.client_name) {
                        (Some(task), Some(project), _) => format!("{project} — {task}"),
                        (Some(task), None, _) => task.clone(),
                        (None, Some(project), _) => project.clone(),
                        (None, None, client) => client.clone().unwrap_or_default(),
                    }
                }
            };
            let seconds = r.seconds(rounding_minutes);
            match groups
                .iter_mut()
                .find(|g| g.0 == description && g.1 == *rate)
            {
                Some(group) => group.2 += seconds,
                None => groups.push((description, *rate, seconds)),
            }
        }
        if input.grouping == Grouping::SingleLine && groups.len() > 1 {
            return Err(CoreError::new("invoice.mixed_rates"));
        }
        groups.sort_by(|a, b| {
            (crate::domain::text::fold(&a.0), a.1.amount_minor)
                .cmp(&(crate::domain::text::fold(&b.0), b.1.amount_minor))
        });
        let items = groups
            .into_iter()
            .map(|(description, price, seconds)| InvoiceItemInput {
                description,
                // Hours in thousandths, rounded half up.
                quantity_milli: (seconds * 1000 + 1800) / 3600,
                unit: unit.into(),
                price,
            })
            .collect();

        let tx = self.conn().unchecked_transaction()?;
        let id = self.insert(
            &tx,
            InvoiceInput {
                number: input.number,
                client_id: input.client_id,
                project_id: input.project_id,
                currency,
                issue_date: input.issue_date,
                due_date: input.due_date,
                status: StoredStatus::Draft,
                items,
                discount: Discount::None,
                tax_bp: input.tax_bp,
                notes_md: String::new(),
                language: input.language,
            },
        )?;
        for r in &entries {
            tx.execute(
                "UPDATE time_entries SET invoice_id = ?2 WHERE id = ?1",
                (r.entry.id, id),
            )?;
        }
        tx.commit()?;
        self.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};
    use crate::tasks::{Priority, TaskInput, TaskRepo};
    use crate::time_entries::{TimeEntryInput, TimeRepo, TimeTarget};

    fn utc(s: &str) -> Timestamp {
        Timestamp::from_millis(s.parse::<jiff::Timestamp>().unwrap().as_millisecond())
    }

    struct Fixture {
        db: Db,
        clock: FakeClock,
        client: Id,
        site: Id,
        design: Id,
        markup: Id,
    }

    impl Fixture {
        /// Ромашка (2 000 ₽/h) with project "Сайт" at 3 000 ₽/h and its tasks
        /// "Дизайн" and "Вёрстка".
        fn new() -> Self {
            let db = Db::open_in_memory().unwrap();
            let clock = FakeClock::at(utc("2026-04-01T00:00:00Z"));
            let client = ClientRepo::new(db.conn(), &clock)
                .create(ClientInput {
                    kind: ClientKind::Company,
                    last_name: None,
                    first_name: None,
                    middle_name: None,
                    company_name: Some("Ромашка".into()),
                    legal_name: None,
                    status: ClientStatus::Active,
                    default_currency: Some("RUB".parse().unwrap()),
                    default_rate_minor: Some(200_000),
                })
                .unwrap()
                .id;
            let site = ProjectRepo::new(db.conn(), &clock)
                .create(ProjectInput {
                    client_id: client,
                    name: "Сайт".into(),
                    description_md: String::new(),
                    status: ProjectStatus::Active,
                    billing_type: BillingType::Hourly,
                    currency: Some("RUB".parse().unwrap()),
                    rate_minor: Some(300_000),
                    budget_minor: None,
                    start_date: None,
                    deadline: None,
                })
                .unwrap()
                .id;
            let task = |title: &str| {
                TaskRepo::new(db.conn(), &clock)
                    .create(TaskInput {
                        title: title.into(),
                        description_md: String::new(),
                        status_id: None,
                        priority: Priority::Normal,
                        project_id: Some(site),
                        client_id: None,
                        due_date: None,
                        due_time: None,
                        estimate_minutes: None,
                    })
                    .unwrap()
                    .id
            };
            let (design, markup) = (task("Дизайн"), task("Вёрстка"));
            Self {
                db,
                clock,
                client,
                site,
                design,
                markup,
            }
        }

        fn log(&self, target: TimeTarget, from: &str, minutes: i64, billable: bool) -> Id {
            TimeRepo::new(self.db.conn(), &self.clock)
                .create(TimeEntryInput {
                    target,
                    description: String::new(),
                    started_at: utc(from),
                    ended_at: Timestamp::from_millis(utc(from).as_millis() + minutes * 60_000),
                    billable,
                })
                .unwrap()
                .entry
                .id
        }

        fn task(&self, id: Id) -> TimeTarget {
            TimeTarget {
                task_id: Some(id),
                ..Default::default()
            }
        }

        fn input(&self, grouping: Grouping) -> FromTimeInput {
            FromTimeInput {
                client_id: self.client,
                project_id: None,
                from: utc("2026-03-01T00:00:00Z"),
                to: utc("2026-04-01T00:00:00Z"),
                grouping,
                description: "Разработка сайта, март 2026".into(),
                number: String::new(),
                issue_date: "2026-04-01".into(),
                due_date: Some("2026-04-15".into()),
                tax_bp: 0,
                language: "ru".into(),
            }
        }

        fn repo(&self) -> InvoiceRepo<'_> {
            InvoiceRepo::new(self.db.conn(), &self.clock)
        }

        fn invoice_of(&self, entry: Id) -> Option<Id> {
            self.db
                .conn()
                .query_row(
                    "SELECT invoice_id FROM time_entries WHERE id = ?1",
                    [entry],
                    |r| r.get(0),
                )
                .unwrap()
        }
    }

    fn rub(minor: i64) -> Money {
        Money::new(minor, "RUB".parse().unwrap())
    }

    #[test]
    fn bills_billable_unbilled_time_by_task() {
        let f = Fixture::new();
        let a = f.log(f.task(f.design), "2026-03-02T10:00:00Z", 90, true);
        let b = f.log(f.task(f.design), "2026-03-03T10:00:00Z", 30, true);
        let c = f.log(f.task(f.markup), "2026-03-04T10:00:00Z", 45, true);
        let free = f.log(f.task(f.markup), "2026-03-05T10:00:00Z", 60, false);
        let april = f.log(f.task(f.markup), "2026-04-02T10:00:00Z", 60, true);
        let inv = f.repo().from_time(f.input(Grouping::ByTask), 0).unwrap();
        assert_eq!(inv.number, "INV-2026-001");
        assert_eq!(
            inv.items
                .iter()
                .map(|i| (i.description.as_str(), i.quantity_milli, i.price, i.amount))
                .collect::<Vec<_>>(),
            [
                ("Сайт — Вёрстка", 750, rub(300_000), rub(225_000)),
                ("Сайт — Дизайн", 2000, rub(300_000), rub(600_000)),
            ]
        );
        assert_eq!(inv.items[0].unit, "ч");
        assert_eq!(inv.totals.total, rub(825_000));
        for entry in [a, b, c] {
            assert_eq!(f.invoice_of(entry), Some(inv.id));
        }
        assert_eq!((f.invoice_of(free), f.invoice_of(april)), (None, None));
        // Nothing is left to bill.
        assert_eq!(
            f.repo()
                .from_time(f.input(Grouping::ByTask), 0)
                .unwrap_err(),
            CoreError::new("invoice.no_time")
        );
    }

    #[test]
    fn bills_as_one_line_with_rounding() {
        let f = Fixture::new();
        f.log(f.task(f.design), "2026-03-02T10:00:00Z", 50, true);
        f.log(f.task(f.markup), "2026-03-03T10:00:00Z", 7, true);
        let mut input = f.input(Grouping::SingleLine);
        input.language = "en".into();
        let inv = f.repo().from_time(input, 15).unwrap();
        // 60 + 15 minutes.
        assert_eq!(inv.items.len(), 1);
        assert_eq!(
            (
                inv.items[0].description.as_str(),
                inv.items[0].quantity_milli,
                inv.items[0].unit.as_str()
            ),
            ("Разработка сайта, март 2026", 1250, "h")
        );
        assert_eq!(inv.totals.total, rub(375_000));
    }

    #[test]
    fn time_without_a_task_and_different_rates() {
        let f = Fixture::new();
        // On the project (3 000 ₽/h) and on the client itself (2 000 ₽/h).
        f.log(
            TimeTarget {
                project_id: Some(f.site),
                ..Default::default()
            },
            "2026-03-02T10:00:00Z",
            60,
            true,
        );
        f.log(
            TimeTarget {
                client_id: Some(f.client),
                ..Default::default()
            },
            "2026-03-03T10:00:00Z",
            60,
            true,
        );
        let inv = f.repo().from_time(f.input(Grouping::ByTask), 0).unwrap();
        assert_eq!(
            inv.items
                .iter()
                .map(|i| (i.description.as_str(), i.price))
                .collect::<Vec<_>>(),
            [("Ромашка", rub(200_000)), ("Сайт", rub(300_000))]
        );
        f.repo()
            .update(
                inv.id,
                InvoiceInput {
                    status: StoredStatus::Cancelled,
                    ..inv_input(&inv)
                },
            )
            .unwrap();
        // One price cannot cover two rates.
        assert_eq!(
            f.repo()
                .from_time(f.input(Grouping::SingleLine), 0)
                .unwrap_err(),
            CoreError::new("invoice.mixed_rates")
        );
        // Only the project's time.
        let mut only_site = f.input(Grouping::SingleLine);
        only_site.project_id = Some(f.site);
        assert_eq!(
            f.repo().from_time(only_site, 0).unwrap().totals.total,
            rub(300_000)
        );
    }

    #[test]
    fn cancelling_or_deleting_frees_the_time() {
        let f = Fixture::new();
        let entry = f.log(f.task(f.design), "2026-03-02T10:00:00Z", 60, true);
        let inv = f.repo().from_time(f.input(Grouping::ByTask), 0).unwrap();
        f.repo()
            .update(
                inv.id,
                InvoiceInput {
                    status: StoredStatus::Cancelled,
                    ..inv_input(&inv)
                },
            )
            .unwrap();
        assert_eq!(f.invoice_of(entry), None);
        let again = f.repo().from_time(f.input(Grouping::ByTask), 0).unwrap();
        assert_eq!(f.invoice_of(entry), Some(again.id));
        f.repo().delete(again.id).unwrap();
        assert_eq!(f.invoice_of(entry), None);
        let _ = Discount::None;
    }

    #[test]
    fn needs_a_rate_in_one_currency() {
        let f = Fixture::new();
        f.db.conn()
            .execute(
                "UPDATE clients SET default_rate_minor = NULL WHERE id = ?1",
                [f.client],
            )
            .unwrap();
        f.log(
            TimeTarget {
                client_id: Some(f.client),
                ..Default::default()
            },
            "2026-03-02T10:00:00Z",
            60,
            true,
        );
        assert_eq!(
            f.repo()
                .from_time(f.input(Grouping::ByTask), 0)
                .unwrap_err(),
            CoreError::new("invoice.no_rate")
        );
        f.db.conn()
            .execute("UPDATE clients SET default_rate_minor = 5000, default_currency = 'USD' WHERE id = ?1", [f.client])
            .unwrap();
        f.log(f.task(f.design), "2026-03-03T10:00:00Z", 60, true);
        assert_eq!(
            f.repo()
                .from_time(f.input(Grouping::ByTask), 0)
                .unwrap_err(),
            CoreError::new("invoice.mixed_currencies")
        );
    }

    /// The input that would save `inv` unchanged.
    fn inv_input(inv: &Invoice) -> InvoiceInput {
        InvoiceInput {
            number: inv.number.clone(),
            client_id: inv.client_id,
            project_id: inv.project_id,
            currency: inv.currency,
            issue_date: inv.issue_date.clone(),
            due_date: inv.due_date.clone(),
            status: inv.stored_status,
            items: inv
                .items
                .iter()
                .map(|i| super::super::invoices::InvoiceItemInput {
                    description: i.description.clone(),
                    quantity_milli: i.quantity_milli,
                    unit: i.unit.clone(),
                    price: i.price,
                })
                .collect(),
            discount: inv.discount,
            tax_bp: inv.tax_bp,
            notes_md: inv.notes_md.clone(),
            language: inv.language.clone(),
        }
    }
}
