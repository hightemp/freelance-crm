//! Dashboard aggregates (FR-FN-8) for a period of dates: income by month,
//! client and currency, expenses and profit, open invoices and, when taxes
//! are on, the tax estimate. Payment and expense dates are calendar dates
//! the user entered in their own time zone, so months are their prefixes;
//! only "overdue" needs the zone, for today's date. Money in different
//! currencies is never summed together.

use std::collections::BTreeMap;

use jiff::tz::TimeZone;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::invoices::{InvoiceRepo, InvoiceStatus};
use super::taxes::TaxRepo;
use crate::domain::{Clock, Currency, Id, Money};
use crate::{CoreError, CoreResult};

/// Inclusive dates, `YYYY-MM-DD`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    pub from: String,
    pub to: String,
}

/// One currency over the period; open amounts are as of today.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyTotals {
    pub currency: Currency,
    #[specta(type = i32)]
    pub income_minor: i64,
    #[specta(type = i32)]
    pub expenses_minor: i64,
    #[specta(type = i32)]
    pub profit_minor: i64,
    /// Still due on sent, partially paid and overdue invoices.
    #[specta(type = i32)]
    pub unpaid_minor: i64,
    /// The overdue part of `unpaid_minor`.
    #[specta(type = i32)]
    pub overdue_minor: i64,
    /// Only when taxes are on.
    #[specta(type = Option<i32>)]
    pub tax_minor: Option<i64>,
    /// Income less tax; only when taxes are on.
    #[specta(type = Option<i32>)]
    pub net_minor: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MonthTotals {
    /// `YYYY-MM`.
    pub month: String,
    pub currency: Currency,
    #[specta(type = i32)]
    pub income_minor: i64,
    #[specta(type = i32)]
    pub expenses_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ClientIncome {
    pub client_id: Id,
    pub client_name: String,
    pub currency: Currency,
    #[specta(type = i32)]
    pub income_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenInvoice {
    pub id: Id,
    pub number: String,
    pub client_id: Id,
    pub client_name: String,
    pub due_date: Option<String>,
    pub due: Money,
    pub overdue: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    /// The default currency first, then by code; never empty.
    pub currencies: Vec<CurrencyTotals>,
    /// Every month of the period for every currency, oldest first.
    pub months: Vec<MonthTotals>,
    /// Clients who paid in the period, by income descending.
    pub clients: Vec<ClientIncome>,
    /// Overdue first, then by due date.
    pub open_invoices: Vec<OpenInvoice>,
    pub taxes_enabled: bool,
}

impl CurrencyTotals {
    fn zero(currency: Currency, taxed: bool) -> Self {
        Self {
            currency,
            income_minor: 0,
            expenses_minor: 0,
            profit_minor: 0,
            unpaid_minor: 0,
            overdue_minor: 0,
            tax_minor: taxed.then_some(0),
            net_minor: taxed.then_some(0),
        }
    }
}

fn entry(
    totals: &mut BTreeMap<String, CurrencyTotals>,
    currency: Currency,
    taxed: bool,
) -> &mut CurrencyTotals {
    totals
        .entry(currency.code().to_owned())
        .or_insert_with(|| CurrencyTotals::zero(currency, taxed))
}

fn validate(period: &Period) -> CoreResult<()> {
    use crate::domain::date::is_iso_date;
    if !is_iso_date(&period.from) {
        return Err(CoreError::validation("from", "date"));
    }
    if !is_iso_date(&period.to) {
        return Err(CoreError::validation("to", "date"));
    }
    if period.to < period.from {
        return Err(CoreError::validation("to", "before_start"));
    }
    Ok(())
}

/// `YYYY-MM` of every month from `from` to `to`.
fn months_between(from: &str, to: &str) -> Vec<String> {
    let parse = |d: &str| -> (i32, u32) { (d[..4].parse().unwrap(), d[5..7].parse().unwrap()) };
    let (mut year, mut month) = parse(from);
    let end = parse(to);
    let mut out = Vec::new();
    while (year, month) <= end {
        out.push(format!("{year:04}-{month:02}"));
        (year, month) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
    }
    out
}

/// Invoices awaiting payment as of today in `tz`: overdue first, then by
/// due date (those without one last).
pub fn open_invoices(
    conn: &Connection,
    clock: &dyn Clock,
    tz: TimeZone,
) -> CoreResult<Vec<OpenInvoice>> {
    let invoices = InvoiceRepo::new(conn, clock).in_zone(tz).list(None, None)?;
    let mut open: Vec<OpenInvoice> = invoices
        .into_iter()
        .filter(|i| i.status.awaits_payment() && i.totals.due.amount_minor > 0)
        .map(|i| OpenInvoice {
            overdue: i.status == InvoiceStatus::Overdue,
            id: i.id,
            number: i.number,
            client_id: i.client_id,
            client_name: i.client_name,
            due_date: i.due_date,
            due: i.totals.due,
        })
        .collect();
    // Without a due date: last.
    open.sort_by(|a, b| {
        (!a.overdue, a.due_date.is_none(), &a.due_date, &a.number).cmp(&(
            !b.overdue,
            b.due_date.is_none(),
            &b.due_date,
            &b.number,
        ))
    });
    Ok(open)
}

pub fn dashboard(
    conn: &Connection,
    clock: &dyn Clock,
    tz: TimeZone,
    period: &Period,
) -> CoreResult<Dashboard> {
    validate(period)?;
    let settings = crate::settings::load_from(conn)?;
    let range = (period.from.as_str(), period.to.as_str());

    // (month, currency, client, client name, amount)
    let mut stmt = conn.prepare(
        "SELECT substr(pm.paid_on, 1, 7), pm.currency, pm.client_id, c.display_name,
                sum(pm.amount_minor)
         FROM payments pm JOIN clients c ON c.id = pm.client_id
         WHERE pm.paid_on BETWEEN ?1 AND ?2
         GROUP BY 1, 2, 3",
    )?;
    let income = stmt
        .query_map(range, |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Currency>(1)?,
                r.get::<_, Id>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut stmt = conn.prepare(
        "SELECT substr(spent_on, 1, 7), currency, sum(amount_minor) FROM expenses
         WHERE spent_on BETWEEN ?1 AND ?2 GROUP BY 1, 2",
    )?;
    let expenses = stmt
        .query_map(range, |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Currency>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let open_invoices = open_invoices(conn, clock, tz)?;

    let taxes = TaxRepo::new(conn, clock).estimate(&period.from, &period.to)?;

    // Currencies by code, the default one always present.
    let mut totals: BTreeMap<String, CurrencyTotals> = BTreeMap::new();
    let taxed = taxes.is_some();
    entry(&mut totals, settings.default_currency, taxed);
    for (_, currency, _, _, amount) in &income {
        entry(&mut totals, *currency, taxed).income_minor += amount;
    }
    for (_, currency, amount) in &expenses {
        entry(&mut totals, *currency, taxed).expenses_minor += amount;
    }
    for invoice in &open_invoices {
        let t = entry(&mut totals, invoice.due.currency, taxed);
        t.unpaid_minor += invoice.due.amount_minor;
        if invoice.overdue {
            t.overdue_minor += invoice.due.amount_minor;
        }
    }
    for line in taxes.iter().flatten() {
        let t = entry(&mut totals, line.currency, taxed);
        t.tax_minor = Some(line.tax_minor);
    }
    let mut currencies: Vec<CurrencyTotals> = totals
        .into_values()
        .map(|mut t| {
            t.profit_minor = t.income_minor - t.expenses_minor;
            t.net_minor = t.tax_minor.map(|tax| t.income_minor - tax);
            t
        })
        .collect();
    currencies.sort_by_key(|t| t.currency != settings.default_currency);

    let mut months = Vec::new();
    for month in months_between(&period.from, &period.to) {
        for t in &currencies {
            let currency = t.currency;
            months.push(MonthTotals {
                income_minor: income
                    .iter()
                    .filter(|(m, c, ..)| *m == month && *c == currency)
                    .map(|i| i.4)
                    .sum(),
                expenses_minor: expenses
                    .iter()
                    .filter(|(m, c, _)| *m == month && *c == currency)
                    .map(|e| e.2)
                    .sum(),
                month: month.clone(),
                currency,
            });
        }
    }

    let mut by_client: BTreeMap<(Id, String), ClientIncome> = BTreeMap::new();
    for (_, currency, client_id, client_name, amount) in income {
        by_client
            .entry((client_id, currency.code().to_owned()))
            .or_insert(ClientIncome {
                client_id,
                client_name,
                currency,
                income_minor: 0,
            })
            .income_minor += amount;
    }
    let mut clients: Vec<ClientIncome> = by_client.into_values().collect();
    clients.sort_by(|a, b| {
        b.income_minor
            .cmp(&a.income_minor)
            .then_with(|| a.client_name.cmp(&b.client_name))
    });

    Ok(Dashboard {
        currencies,
        months,
        clients,
        open_invoices,
        taxes_enabled: taxes.is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::{FakeClock, Timestamp};
    use crate::finance::expenses::{ExpenseInput, ExpenseRepo};
    use crate::finance::invoices::{Discount, InvoiceInput, InvoiceItemInput, StoredStatus};
    use crate::finance::payments::{PaymentInput, PaymentRepo};
    use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};

    fn money(minor: i64, code: &str) -> Money {
        Money::new(minor, code.parse().unwrap())
    }

    fn period(from: &str, to: &str) -> Period {
        Period {
            from: from.into(),
            to: to.into(),
        }
    }

    struct Fixture {
        db: Db,
        clock: FakeClock,
    }

    /// A client with a project.
    struct Party {
        client: Id,
        project: Id,
    }

    impl Fixture {
        /// 2026-03-10 22:00 UTC: already 2026-03-11 in Moscow.
        /// Roubles by default.
        fn new() -> Self {
            let db = Db::open_in_memory().unwrap();
            db.conn()
                .execute(
                    "INSERT INTO settings (key, value) VALUES ('defaultCurrency', '\"RUB\"')",
                    [],
                )
                .unwrap();
            Self {
                db,
                clock: FakeClock::at(Timestamp::from_millis(1_773_180_000_000)),
            }
        }

        fn party(&self, name: &str) -> Party {
            let client = ClientRepo::new(self.db.conn(), &self.clock)
                .create(ClientInput {
                    kind: ClientKind::Company,
                    last_name: None,
                    first_name: None,
                    middle_name: None,
                    company_name: Some(name.to_owned()),
                    legal_name: None,
                    status: ClientStatus::Active,
                    default_currency: None,
                    default_rate_minor: None,
                })
                .unwrap()
                .id;
            let project = ProjectRepo::new(self.db.conn(), &self.clock)
                .create(ProjectInput {
                    client_id: client,
                    name: format!("Проект {name}"),
                    description_md: String::new(),
                    status: ProjectStatus::Active,
                    billing_type: BillingType::Fixed,
                    currency: None,
                    rate_minor: None,
                    budget_minor: Some(1),
                    start_date: None,
                    deadline: None,
                })
                .unwrap()
                .id;
            Party { client, project }
        }

        fn pay(&self, party: &Party, amount: Money, date: &str) {
            PaymentRepo::new(self.db.conn(), &self.clock)
                .create(PaymentInput {
                    invoice_id: None,
                    project_id: Some(party.project),
                    amount,
                    paid_on: date.into(),
                    method: String::new(),
                    note: String::new(),
                })
                .unwrap();
        }

        fn pay_invoice(&self, invoice: Id, amount: Money, date: &str) {
            PaymentRepo::new(self.db.conn(), &self.clock)
                .create(PaymentInput {
                    invoice_id: Some(invoice),
                    project_id: None,
                    amount,
                    paid_on: date.into(),
                    method: String::new(),
                    note: String::new(),
                })
                .unwrap();
        }

        fn spend(&self, amount: Money, date: &str) {
            ExpenseRepo::new(self.db.conn(), &self.clock)
                .create(ExpenseInput {
                    project_id: None,
                    category: "software".into(),
                    amount,
                    spent_on: date.into(),
                    description: String::new(),
                })
                .unwrap();
        }

        fn invoice(
            &self,
            party: &Party,
            number: &str,
            total: Money,
            status: StoredStatus,
            due: Option<&str>,
        ) -> Id {
            InvoiceRepo::new(self.db.conn(), &self.clock)
                .create(InvoiceInput {
                    number: number.into(),
                    client_id: party.client,
                    project_id: Some(party.project),
                    currency: total.currency,
                    issue_date: "2025-12-01".into(),
                    due_date: due.map(Into::into),
                    status,
                    items: vec![InvoiceItemInput {
                        description: "Работа".into(),
                        quantity_milli: 1000,
                        unit: String::new(),
                        price: total,
                    }],
                    discount: Discount::None,
                    tax_bp: 0,
                    notes_md: String::new(),
                    language: "ru".into(),
                    show_signature: None,
                })
                .unwrap()
                .id
        }

        fn dashboard(&self, period: &Period) -> CoreResult<Dashboard> {
            dashboard(
                self.db.conn(),
                &self.clock,
                TimeZone::get("Europe/Moscow").unwrap(),
                period,
            )
        }
    }

    #[test]
    fn an_empty_period_gives_zeros_in_the_default_currency() {
        let f = Fixture::new();
        let d = f.dashboard(&period("2026-01-15", "2026-03-01")).unwrap();
        assert_eq!(
            d.currencies,
            [CurrencyTotals::zero("RUB".parse().unwrap(), false)]
        );
        assert_eq!(
            d.months
                .iter()
                .map(|m| (m.month.as_str(), m.income_minor, m.expenses_minor))
                .collect::<Vec<_>>(),
            [("2026-01", 0, 0), ("2026-02", 0, 0), ("2026-03", 0, 0)]
        );
        assert!(d.clients.is_empty());
        assert!(d.open_invoices.is_empty());
        assert!(!d.taxes_enabled);
    }

    #[test]
    fn income_by_month_client_and_currency_with_expenses_and_profit() {
        let f = Fixture::new();
        let a = f.party("Альфа");
        let b = f.party("Бета");
        f.pay(&a, money(10000, "RUB"), "2026-01-31");
        f.pay(&a, money(5000, "RUB"), "2026-03-01");
        f.pay(&b, money(30000, "RUB"), "2026-03-15");
        f.pay(&b, money(2000, "USD"), "2026-02-10");
        // Outside the period.
        f.pay(&a, money(99900, "RUB"), "2025-12-31");
        f.pay(&a, money(99900, "RUB"), "2026-04-01");
        f.spend(money(3000, "RUB"), "2026-03-02");
        f.spend(money(500, "USD"), "2026-01-05");
        f.spend(money(99900, "RUB"), "2026-04-01");

        let d = f.dashboard(&period("2026-01-01", "2026-03-31")).unwrap();
        let totals: Vec<_> = d
            .currencies
            .iter()
            .map(|t| {
                (
                    t.currency.code(),
                    t.income_minor,
                    t.expenses_minor,
                    t.profit_minor,
                )
            })
            .collect();
        assert_eq!(
            totals,
            [("RUB", 45000, 3000, 42000), ("USD", 2000, 500, 1500)]
        );
        let months: Vec<_> = d
            .months
            .iter()
            .map(|m| {
                (
                    m.month.as_str(),
                    m.currency.code(),
                    m.income_minor,
                    m.expenses_minor,
                )
            })
            .collect();
        assert_eq!(
            months,
            [
                ("2026-01", "RUB", 10000, 0),
                ("2026-01", "USD", 0, 500),
                ("2026-02", "RUB", 0, 0),
                ("2026-02", "USD", 2000, 0),
                ("2026-03", "RUB", 35000, 3000),
                ("2026-03", "USD", 0, 0),
            ]
        );
        let clients: Vec<_> = d
            .clients
            .iter()
            .map(|c| (c.client_name.as_str(), c.currency.code(), c.income_minor))
            .collect();
        assert_eq!(
            clients,
            [
                ("Бета", "RUB", 30000),
                ("Альфа", "RUB", 15000),
                ("Бета", "USD", 2000)
            ]
        );
        assert_eq!(d.clients[0].client_id, b.client);
    }

    #[test]
    fn open_invoices_unpaid_and_overdue_in_the_users_time_zone() {
        let f = Fixture::new();
        let a = f.party("Альфа");
        // Due yesterday in Moscow, today in UTC.
        let overdue = f.invoice(
            &a,
            "1",
            money(10000, "RUB"),
            StoredStatus::Sent,
            Some("2026-03-10"),
        );
        let later = f.invoice(
            &a,
            "2",
            money(20000, "RUB"),
            StoredStatus::Sent,
            Some("2026-04-01"),
        );
        let partial = f.invoice(&a, "3", money(30000, "RUB"), StoredStatus::Sent, None);
        f.pay_invoice(partial, money(10000, "RUB"), "2026-03-01");
        let paid = f.invoice(
            &a,
            "4",
            money(5000, "RUB"),
            StoredStatus::Sent,
            Some("2026-03-01"),
        );
        f.pay_invoice(paid, money(5000, "RUB"), "2026-03-01");
        f.invoice(
            &a,
            "5",
            money(7000, "RUB"),
            StoredStatus::Draft,
            Some("2026-01-01"),
        );
        f.invoice(
            &a,
            "6",
            money(8000, "RUB"),
            StoredStatus::Cancelled,
            Some("2026-01-01"),
        );
        let usd = f.invoice(
            &a,
            "7",
            money(1000, "USD"),
            StoredStatus::Sent,
            Some("2026-01-01"),
        );

        // Open amounts do not depend on the period.
        let d = f.dashboard(&period("2025-01-01", "2025-01-31")).unwrap();
        let open: Vec<_> = d
            .open_invoices
            .iter()
            .map(|i| (i.id, i.due, i.overdue))
            .collect();
        assert_eq!(
            open,
            [
                (usd, money(1000, "USD"), true),
                (overdue, money(10000, "RUB"), true),
                (later, money(20000, "RUB"), false),
                (partial, money(20000, "RUB"), false),
            ]
        );
        let totals: Vec<_> = d
            .currencies
            .iter()
            .map(|t| (t.currency.code(), t.unpaid_minor, t.overdue_minor))
            .collect();
        assert_eq!(totals, [("RUB", 50000, 10000), ("USD", 1000, 1000)]);
        // In UTC it is still 2026-03-10: not overdue yet.
        let utc = dashboard(
            f.db.conn(),
            &f.clock,
            TimeZone::UTC,
            &period("2025-01-01", "2025-01-31"),
        )
        .unwrap();
        assert!(
            !utc.open_invoices
                .iter()
                .any(|i| i.id == overdue && i.overdue)
        );
    }

    #[test]
    fn with_taxes_on_tax_and_net_income_for_the_period() {
        let f = Fixture::new();
        let a = f.party("Альфа");
        f.db.conn()
            .execute(
                "INSERT INTO settings (key, value) VALUES ('taxesEnabled', 'true')",
                [],
            )
            .unwrap();
        TaxRepo::new(f.db.conn(), &f.clock)
            .save(
                None,
                crate::finance::taxes::preset("usn_income", "2026-01-01").unwrap(),
            )
            .unwrap();
        f.pay(&a, money(100000, "RUB"), "2026-03-02");
        f.pay(&a, money(1000, "USD"), "2026-03-03");

        let d = f.dashboard(&period("2026-03-01", "2026-03-31")).unwrap();
        assert!(d.taxes_enabled);
        let taxes: Vec<_> = d
            .currencies
            .iter()
            .map(|t| (t.currency.code(), t.tax_minor, t.net_minor))
            .collect();
        assert_eq!(
            taxes,
            [
                ("RUB", Some(6000), Some(94000)),
                ("USD", Some(60), Some(940))
            ]
        );
        // An empty period with taxes on: zero tax, not none.
        let empty = f.dashboard(&period("2026-05-01", "2026-05-31")).unwrap();
        assert_eq!(
            empty.currencies,
            [CurrencyTotals::zero("RUB".parse().unwrap(), true)]
        );
    }

    #[test]
    fn rejects_bad_periods() {
        let f = Fixture::new();
        assert_eq!(
            f.dashboard(&period("2026-13-01", "2026-12-31"))
                .unwrap_err(),
            CoreError::validation("from", "date")
        );
        assert_eq!(
            f.dashboard(&period("2026-01-01", "")).unwrap_err(),
            CoreError::validation("to", "date")
        );
        assert_eq!(
            f.dashboard(&period("2026-02-01", "2026-01-31"))
                .unwrap_err(),
            CoreError::validation("to", "before_start")
        );
    }

    #[test]
    fn months_cross_years() {
        assert_eq!(
            months_between("2025-11-30", "2026-02-01"),
            ["2025-11", "2025-12", "2026-01", "2026-02"]
        );
    }
}
