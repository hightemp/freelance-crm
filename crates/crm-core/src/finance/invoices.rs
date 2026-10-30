//! Invoices (FR-FN-1, 2, 3): items priced in minor units of the invoice's
//! currency, a discount (percent or fixed) and a tax percentage, rounded
//! half-even. `draft`, `sent` and `cancelled` are stored; `paid`,
//! `partially_paid` and `overdue` follow from payments and the due date.

use jiff::tz::TimeZone;
use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::domain::{Clock, Currency, Id, Money, Timestamp};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Sent,
    PartiallyPaid,
    Paid,
    Overdue,
    Cancelled,
}

impl InvoiceStatus {
    /// Sent and not paid in full yet.
    pub fn awaits_payment(self) -> bool {
        matches!(self, Self::Sent | Self::PartiallyPaid | Self::Overdue)
    }
}

/// What the user sets; the rest of [`InvoiceStatus`] is computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum StoredStatus {
    Draft,
    Sent,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum Discount {
    None,
    /// Basis points: 1000 = 10 %.
    Percent(#[specta(type = i32)] i64),
    /// Minor units of the invoice's currency.
    Fixed(#[specta(type = i32)] i64),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceItemInput {
    pub description: String,
    /// Thousandths: 1.5 h is 1500.
    #[specta(type = i32)]
    pub quantity_milli: i64,
    pub unit: String,
    pub price: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceItem {
    pub id: Id,
    pub description: String,
    #[specta(type = i32)]
    pub quantity_milli: i64,
    pub unit: String,
    pub price: Money,
    /// Price × quantity, rounded.
    pub amount: Money,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceInput {
    /// Empty on creation: the next number from the template in settings.
    pub number: String,
    pub client_id: Id,
    pub project_id: Option<Id>,
    pub currency: Currency,
    /// `YYYY-MM-DD`.
    pub issue_date: String,
    pub due_date: Option<String>,
    pub status: StoredStatus,
    pub items: Vec<InvoiceItemInput>,
    pub discount: Discount,
    /// Basis points: 2000 = 20 %.
    #[specta(type = i32)]
    pub tax_bp: i64,
    pub notes_md: String,
    /// Language of the document: `en` or `ru`.
    pub language: String,
    /// Signature and stamp on the document; `None` follows the settings.
    #[serde(default)]
    pub show_signature: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub subtotal: Money,
    pub discount: Money,
    pub tax: Money,
    pub total: Money,
    pub paid: Money,
    /// Still to pay; negative when overpaid.
    pub due: Money,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    pub id: Id,
    pub number: String,
    pub client_id: Id,
    pub client_name: String,
    pub project_id: Option<Id>,
    pub project_name: Option<String>,
    pub currency: Currency,
    pub issue_date: String,
    pub due_date: Option<String>,
    pub stored_status: StoredStatus,
    pub status: InvoiceStatus,
    pub items: Vec<InvoiceItem>,
    pub discount: Discount,
    #[specta(type = i32)]
    pub tax_bp: i64,
    pub notes_md: String,
    pub language: String,
    pub show_signature: Option<bool>,
    pub totals: Totals,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

fn overflow(_: crate::domain::MoneyError) -> CoreError {
    CoreError::new("money.overflow")
}

/// Item amounts, subtotal, discount, tax and total (before payments).
pub fn compute(
    currency: Currency,
    items: &[InvoiceItemInput],
    discount: Discount,
    tax_bp: i64,
) -> CoreResult<(Vec<Money>, Money, Money, Money, Money)> {
    let mut lines = Vec::with_capacity(items.len());
    let mut subtotal = Money::zero(currency);
    for (index, item) in items.iter().enumerate() {
        if item.price.currency != currency {
            return Err(CoreError::new("invoice.currency_mismatch").with("index", index));
        }
        let line = item
            .price
            .times_milli(item.quantity_milli)
            .map_err(overflow)?;
        subtotal = subtotal.checked_add(line).map_err(overflow)?;
        lines.push(line);
    }
    let off = match discount {
        Discount::None => Money::zero(currency),
        Discount::Percent(bp) => subtotal.percent_bp(bp).map_err(overflow)?,
        Discount::Fixed(minor) => Money::new(minor.min(subtotal.amount_minor), currency),
    };
    let taxable = subtotal.checked_sub(off).map_err(overflow)?;
    let tax = taxable.percent_bp(tax_bp).map_err(overflow)?;
    let total = taxable.checked_add(tax).map_err(overflow)?;
    Ok((lines, subtotal, off, tax, total))
}

pub struct InvoiceRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
    /// Where "today" is for the overdue status.
    tz: TimeZone,
}

impl<'a> InvoiceRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self {
            conn,
            clock,
            tz: TimeZone::UTC,
        }
    }

    pub fn in_zone(self, tz: TimeZone) -> Self {
        Self { tz, ..self }
    }

    pub(crate) fn conn(&self) -> &'a Connection {
        self.conn
    }

    fn validate(&self, input: InvoiceInput, id: Option<Id>) -> CoreResult<InvoiceInput> {
        use crate::domain::date::is_iso_date;
        let number = input.number.trim().to_owned();
        // A new invoice without a number gets one when stored.
        if number.is_empty() && id.is_some() {
            return Err(CoreError::validation("number", "required"));
        }
        if !is_iso_date(&input.issue_date) {
            return Err(CoreError::validation("issueDate", "date"));
        }
        let due_date = crate::domain::text::clean(input.due_date);
        if let Some(due) = &due_date {
            if !is_iso_date(due) {
                return Err(CoreError::validation("dueDate", "date"));
            }
            if *due < input.issue_date {
                return Err(CoreError::validation("dueDate", "before_start"));
            }
        }
        for (i, item) in input.items.iter().enumerate() {
            if item.quantity_milli <= 0 {
                return Err(CoreError::validation(&format!("items.{i}.quantity"), "min"));
            }
            if item.price.amount_minor < 0 {
                return Err(CoreError::validation(&format!("items.{i}.price"), "min"));
            }
        }
        if !(0..=10_000).contains(&input.tax_bp) {
            return Err(CoreError::validation("taxBp", "range"));
        }
        match input.discount {
            Discount::Percent(bp) if !(0..=10_000).contains(&bp) => {
                return Err(CoreError::validation("discount", "range"));
            }
            Discount::Fixed(minor) if minor < 0 => {
                return Err(CoreError::validation("discount", "range"));
            }
            _ => {}
        }
        if !["en", "ru"].contains(&input.language.as_str()) {
            return Err(CoreError::validation("language", "option"));
        }
        compute(input.currency, &input.items, input.discount, input.tax_bp)?;
        crate::db::ensure_exists(self.conn, "clients", "client", input.client_id)?;
        if let Some(project) = input.project_id {
            let owner: Option<Id> = self
                .conn
                .query_row(
                    "SELECT client_id FROM projects WHERE id = ?1",
                    [project],
                    |r| r.get(0),
                )
                .optional()?;
            match owner {
                None => return Err(CoreError::not_found("project", project)),
                Some(owner) if owner != input.client_id => {
                    return Err(CoreError::new("invoice.project_mismatch"));
                }
                _ => {}
            }
        }
        let taken: bool = self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM invoices WHERE number = ?1 AND id IS NOT ?2)",
            (&number, id),
            |r| r.get(0),
        )?;
        if taken {
            return Err(CoreError::new("invoice.number_taken"));
        }
        Ok(InvoiceInput {
            number,
            due_date,
            ..input
        })
    }

    pub fn create(&self, input: InvoiceInput) -> CoreResult<Invoice> {
        let tx = self.conn.unchecked_transaction()?;
        let id = self.insert(&tx, input)?;
        tx.commit()?;
        self.get(id)
    }

    /// Validates and stores a new invoice inside the caller's transaction.
    pub(crate) fn insert(&self, tx: &Connection, input: InvoiceInput) -> CoreResult<Id> {
        let mut input = self.validate(input, None)?;
        let id = Id::new();
        if input.number.is_empty() {
            let template = crate::settings::load_from(tx)?.invoice_number_template;
            input.number = super::numbering::next(tx, &template, &input.issue_date)?;
        }
        let (kind, value) = discount_parts(input.discount);
        tx.execute(
            "INSERT INTO invoices (id, number, client_id, project_id, currency, issue_date, due_date,
                 status, discount_kind, discount_value, tax_bp, notes_md, language, created_at,
                 updated_at, show_signature)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14, ?15)",
            rusqlite::params![
                id,
                input.number,
                input.client_id,
                input.project_id,
                input.currency,
                input.issue_date,
                input.due_date,
                input.status,
                kind,
                value,
                input.tax_bp,
                input.notes_md,
                input.language,
                self.clock.now(),
                input.show_signature,
            ],
        )?;
        self.write_items(tx, id, &input.items)?;
        if input.status == StoredStatus::Sent {
            self.take_snapshot(tx, id)?;
        }
        Ok(id)
    }

    /// Only invoices without payments that are not cancelled can change.
    pub fn update(&self, id: Id, input: InvoiceInput) -> CoreResult<Invoice> {
        let before = self.editable(id)?;
        let input = self.validate(input, Some(id))?;
        let tx = self.conn.unchecked_transaction()?;
        let (kind, value) = discount_parts(input.discount);
        tx.execute(
            "UPDATE invoices SET number = ?2, client_id = ?3, project_id = ?4, currency = ?5,
                 issue_date = ?6, due_date = ?7, status = ?8, discount_kind = ?9,
                 discount_value = ?10, tax_bp = ?11, notes_md = ?12, language = ?13,
                 updated_at = ?14, show_signature = ?15
             WHERE id = ?1",
            rusqlite::params![
                id,
                input.number,
                input.client_id,
                input.project_id,
                input.currency,
                input.issue_date,
                input.due_date,
                input.status,
                kind,
                value,
                input.tax_bp,
                input.notes_md,
                input.language,
                self.clock.now(),
                input.show_signature,
            ],
        )?;
        // Requisites are frozen when the invoice is issued.
        match input.status {
            StoredStatus::Sent if before.stored_status != StoredStatus::Sent => {
                self.take_snapshot(&tx, id)?;
            }
            StoredStatus::Draft => {
                tx.execute("UPDATE invoices SET snapshot = NULL WHERE id = ?1", [id])?;
            }
            _ => {}
        }
        if input.status == StoredStatus::Cancelled {
            // A cancelled invoice gives its time back for billing.
            tx.execute(
                "UPDATE time_entries SET invoice_id = NULL WHERE invoice_id = ?1",
                [id],
            )?;
        }
        tx.execute("DELETE FROM invoice_items WHERE invoice_id = ?1", [id])?;
        self.write_items(&tx, id, &input.items)?;
        tx.commit()?;
        self.get(id)
    }

    fn write_items(&self, conn: &Connection, id: Id, items: &[InvoiceItemInput]) -> CoreResult<()> {
        for (position, item) in items.iter().enumerate() {
            conn.execute(
                "INSERT INTO invoice_items (id, invoice_id, position, description, quantity_milli,
                     unit, price_minor)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    Id::new(),
                    id,
                    position as i64,
                    item.description.trim(),
                    item.quantity_milli,
                    item.unit.trim(),
                    item.price.amount_minor,
                ],
            )?;
        }
        Ok(())
    }

    /// Fails with `invoice.locked` once paid in part or cancelled.
    fn editable(&self, id: Id) -> CoreResult<Invoice> {
        let invoice = self.get(id)?;
        if invoice.stored_status == StoredStatus::Cancelled || invoice.totals.paid.amount_minor > 0
        {
            return Err(CoreError::new("invoice.locked"));
        }
        Ok(invoice)
    }

    pub fn get(&self, id: Id) -> CoreResult<Invoice> {
        let row = self
            .conn
            .query_row(&format!("{SELECT} WHERE i.id = ?1"), [id], from_row)
            .optional()?
            .ok_or_else(|| CoreError::not_found("invoice", id))?;
        self.complete(row)
    }

    /// Adds items, totals and the computed status to a stored row.
    pub(crate) fn complete(&self, row: Row0) -> CoreResult<Invoice> {
        let mut stmt = self.conn.prepare(
            "SELECT id, description, quantity_milli, unit, price_minor FROM invoice_items
             WHERE invoice_id = ?1 ORDER BY position",
        )?;
        let inputs = stmt
            .query_map([row.id], |r| {
                Ok((
                    r.get::<_, Id>(0)?,
                    InvoiceItemInput {
                        description: r.get(1)?,
                        quantity_milli: r.get(2)?,
                        unit: r.get(3)?,
                        price: Money::new(r.get(4)?, row.currency),
                    },
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let plain: Vec<InvoiceItemInput> = inputs.iter().map(|(_, i)| i.clone()).collect();
        let (lines, subtotal, discount, tax, total) =
            compute(row.currency, &plain, row.discount, row.tax_bp)?;
        let paid_minor: i64 = self.conn.query_row(
            "SELECT coalesce(sum(amount_minor), 0) FROM payments WHERE invoice_id = ?1",
            [row.id],
            |r| r.get(0),
        )?;
        let paid = Money::new(paid_minor, row.currency);
        let due = total.checked_sub(paid).map_err(overflow)?;
        let today = jiff::Timestamp::from_millisecond(self.clock.now().as_millis())
            .map_err(|_| CoreError::new("internal.clock"))?
            .to_zoned(self.tz.clone())
            .date()
            .to_string();
        let status = match row.status {
            StoredStatus::Draft => InvoiceStatus::Draft,
            StoredStatus::Cancelled => InvoiceStatus::Cancelled,
            StoredStatus::Sent if total.amount_minor > 0 && paid_minor >= total.amount_minor => {
                InvoiceStatus::Paid
            }
            StoredStatus::Sent if paid_minor > 0 => InvoiceStatus::PartiallyPaid,
            StoredStatus::Sent if row.due_date.as_ref().is_some_and(|d| *d < today) => {
                InvoiceStatus::Overdue
            }
            StoredStatus::Sent => InvoiceStatus::Sent,
        };
        Ok(Invoice {
            id: row.id,
            number: row.number,
            client_id: row.client_id,
            client_name: row.client_name,
            project_id: row.project_id,
            project_name: row.project_name,
            currency: row.currency,
            issue_date: row.issue_date,
            due_date: row.due_date,
            stored_status: row.status,
            status,
            items: inputs
                .into_iter()
                .zip(lines)
                .map(|((id, i), amount)| InvoiceItem {
                    id,
                    description: i.description,
                    quantity_milli: i.quantity_milli,
                    unit: i.unit,
                    price: i.price,
                    amount,
                })
                .collect(),
            discount: row.discount,
            tax_bp: row.tax_bp,
            notes_md: row.notes_md,
            language: row.language,
            show_signature: row.show_signature,
            totals: Totals {
                subtotal,
                discount,
                tax,
                total,
                paid,
                due,
            },
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// A client's or project's invoices (all if neither), newest first.
    pub fn list(&self, client_id: Option<Id>, project_id: Option<Id>) -> CoreResult<Vec<Invoice>> {
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT} WHERE (?1 IS NULL OR i.client_id = ?1) AND (?2 IS NULL OR i.project_id = ?2)
             ORDER BY i.issue_date DESC, i.number DESC"
        ))?;
        let rows = stmt
            .query_map((client_id, project_id), from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter().map(|r| self.complete(r)).collect()
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        self.editable(id)?;
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE time_entries SET invoice_id = NULL WHERE invoice_id = ?1",
            [id],
        )?;
        tx.execute("DELETE FROM invoices WHERE id = ?1", [id])?;
        tx.commit()?;
        Ok(())
    }
}

/// Requisites of both parties as they were when the invoice was issued.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub seller: Option<crate::requisites::Requisites>,
    pub buyer: Option<crate::requisites::Requisites>,
    /// The client's legal name, else its display name.
    pub buyer_name: String,
}

impl InvoiceRepo<'_> {
    /// The parties' requisites now: from the snapshot of an issued
    /// invoice, live for drafts.
    pub fn snapshot(&self, id: Id) -> CoreResult<Snapshot> {
        let stored: Option<String> = self
            .conn
            .query_row("SELECT snapshot FROM invoices WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()?
            .ok_or_else(|| CoreError::not_found("invoice", id))?;
        match stored.and_then(|json| serde_json::from_str(&json).ok()) {
            Some(snapshot) => Ok(snapshot),
            None => self.live_snapshot(self.conn, id),
        }
    }

    fn live_snapshot(&self, conn: &Connection, id: Id) -> CoreResult<Snapshot> {
        use crate::requisites::{RequisitesOwner, RequisitesService};
        let (client, legal, display): (Id, Option<String>, String) = conn.query_row(
            "SELECT c.id, c.legal_name, c.display_name FROM invoices i
             JOIN clients c ON c.id = i.client_id WHERE i.id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let requisites = RequisitesService::new(conn, self.clock);
        Ok(Snapshot {
            seller: requisites.get(RequisitesOwner::Profile)?,
            buyer: requisites.get(RequisitesOwner::Client(client))?,
            buyer_name: legal.filter(|l| !l.trim().is_empty()).unwrap_or(display),
        })
    }

    fn take_snapshot(&self, conn: &Connection, id: Id) -> CoreResult<()> {
        let snapshot = self.live_snapshot(conn, id)?;
        conn.execute(
            "UPDATE invoices SET snapshot = ?2 WHERE id = ?1",
            (
                id,
                serde_json::to_string(&snapshot).expect("snapshot serializes"),
            ),
        )?;
        Ok(())
    }
}

fn discount_parts(discount: Discount) -> (&'static str, i64) {
    match discount {
        Discount::None => ("none", 0),
        Discount::Percent(bp) => ("percent", bp),
        Discount::Fixed(minor) => ("fixed", minor),
    }
}

crate::db::text_enum!(StoredStatus { Draft => "draft", Sent => "sent", Cancelled => "cancelled" });

/// An invoice as stored, before items and totals.
pub(crate) struct Row0 {
    id: Id,
    number: String,
    client_id: Id,
    client_name: String,
    project_id: Option<Id>,
    project_name: Option<String>,
    currency: Currency,
    issue_date: String,
    due_date: Option<String>,
    status: StoredStatus,
    discount: Discount,
    tax_bp: i64,
    notes_md: String,
    language: String,
    show_signature: Option<bool>,
    created_at: Timestamp,
    updated_at: Timestamp,
}

pub(crate) const SELECT: &str = "SELECT i.id, i.number, i.client_id, c.display_name AS client_name,
        i.project_id, p.name AS project_name, i.currency, i.issue_date, i.due_date, i.status,
        i.discount_kind, i.discount_value, i.tax_bp, i.notes_md, i.language, i.show_signature,
        i.created_at, i.updated_at
     FROM invoices i
     JOIN clients c ON c.id = i.client_id
     LEFT JOIN projects p ON p.id = i.project_id";

pub(crate) fn from_row(r: &Row) -> rusqlite::Result<Row0> {
    let value: i64 = r.get("discount_value")?;
    let discount = match r.get::<_, String>("discount_kind")?.as_str() {
        "percent" => Discount::Percent(value),
        "fixed" => Discount::Fixed(value),
        _ => Discount::None,
    };
    Ok(Row0 {
        id: r.get("id")?,
        number: r.get("number")?,
        client_id: r.get("client_id")?,
        client_name: r.get("client_name")?,
        project_id: r.get("project_id")?,
        project_name: r.get("project_name")?,
        currency: r.get("currency")?,
        issue_date: r.get("issue_date")?,
        due_date: r.get("due_date")?,
        status: r.get("status")?,
        discount,
        tax_bp: r.get("tax_bp")?,
        notes_md: r.get("notes_md")?,
        language: r.get("language")?,
        show_signature: r.get("show_signature")?,
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

    fn money(minor: i64, code: &str) -> Money {
        Money::new(minor, code.parse().unwrap())
    }

    fn item(quantity_milli: i64, price: Money) -> InvoiceItemInput {
        InvoiceItemInput {
            description: "Работа".into(),
            quantity_milli,
            unit: "ч".into(),
            price,
        }
    }

    fn totals(
        currency: &str,
        items: &[InvoiceItemInput],
        discount: Discount,
        tax_bp: i64,
    ) -> Vec<i64> {
        let (lines, subtotal, discount, tax, total) =
            compute(currency.parse().unwrap(), items, discount, tax_bp).unwrap();
        let mut out: Vec<i64> = lines.iter().map(|m| m.amount_minor).collect();
        out.extend([
            subtotal.amount_minor,
            discount.amount_minor,
            tax.amount_minor,
            total.amount_minor,
        ]);
        out
    }

    #[test]
    fn sums_items_discount_and_tax() {
        let rub = |m| money(m, "RUB");
        // 2 × 1 500.00 + 0.5 × 3 000.00 = 4 500.00; −10 % = 4 050.00; +20 % tax.
        assert_eq!(
            totals(
                "RUB",
                &[item(2000, rub(150_000)), item(500, rub(300_000))],
                Discount::Percent(1000),
                2000
            ),
            [300_000, 150_000, 450_000, 45_000, 81_000, 486_000]
        );
        assert_eq!(
            totals(
                "RUB",
                &[item(1000, rub(100_000))],
                Discount::Fixed(25_000),
                0
            ),
            [100_000, 100_000, 25_000, 0, 75_000]
        );
        // A discount larger than the subtotal stops at zero.
        assert_eq!(
            totals("RUB", &[item(1000, rub(100))], Discount::Fixed(500), 1000),
            [100, 100, 100, 0, 0]
        );
        assert_eq!(totals("RUB", &[], Discount::None, 2000), [0, 0, 0, 0]);
    }

    #[test]
    fn rounds_half_even_in_minor_units() {
        let rub = |m| money(m, "RUB");
        // 0.05 × 0.5 = 0.025 → 0.02; 0.03 × 0.5 = 0.015 → 0.02.
        assert_eq!(totals("RUB", &[item(500, rub(5))], Discount::None, 0)[0], 2);
        assert_eq!(totals("RUB", &[item(500, rub(3))], Discount::None, 0)[0], 2);
        // Tax of 12.5 % on 1.00 = 0.125 → 0.12.
        assert_eq!(
            totals("RUB", &[item(1000, rub(100))], Discount::None, 1250)[3],
            12
        );
    }

    #[test]
    fn currencies_with_zero_and_three_decimals() {
        // ¥1 234 × 3 with 10 % tax: 3 702 + 370.2 → 370.
        assert_eq!(
            totals(
                "JPY",
                &[item(3000, money(1234, "JPY"))],
                Discount::None,
                1000
            ),
            [3702, 3702, 0, 370, 4072]
        );
        // 1.2345 KWD is not representable; 1.234 KWD × 1.5 = 1.851.
        assert_eq!(
            totals("KWD", &[item(1500, money(1234, "KWD"))], Discount::None, 0)[0],
            1851
        );
    }

    #[test]
    fn items_must_be_in_the_invoices_currency() {
        let err = compute(
            "RUB".parse().unwrap(),
            &[item(1000, money(100, "RUB")), item(1000, money(100, "USD"))],
            Discount::None,
            0,
        )
        .unwrap_err();
        assert_eq!(
            err,
            CoreError::new("invoice.currency_mismatch").with("index", 1)
        );
    }

    struct Fixture {
        db: Db,
        clock: FakeClock,
        client: Id,
    }

    impl Fixture {
        fn new() -> Self {
            let db = Db::open_in_memory().unwrap();
            // 2026-03-10 22:00 UTC is already the 11th in Moscow.
            let clock = FakeClock::at(Timestamp::from_millis(
                "2026-03-10T22:00:00Z"
                    .parse::<jiff::Timestamp>()
                    .unwrap()
                    .as_millisecond(),
            ));
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
            Self { db, clock, client }
        }

        fn repo(&self) -> InvoiceRepo<'_> {
            InvoiceRepo::new(self.db.conn(), &self.clock)
        }

        fn input(&self, number: &str, status: StoredStatus, due: Option<&str>) -> InvoiceInput {
            InvoiceInput {
                number: number.into(),
                client_id: self.client,
                project_id: None,
                currency: "RUB".parse().unwrap(),
                issue_date: "2026-03-01".into(),
                due_date: due.map(Into::into),
                status,
                items: vec![item(1000, money(100_000, "RUB"))],
                discount: Discount::None,
                tax_bp: 0,
                notes_md: String::new(),
                language: "ru".into(),
                show_signature: None,
            }
        }

        fn pay(&self, invoice: Id, amount: i64) {
            self.db
                .conn()
                .execute(
                    "INSERT INTO payments (id, invoice_id, client_id, amount_minor, currency, paid_on,
                         method, note, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, 'RUB', '2026-03-05', '', '', 0, 0)",
                    (Id::new(), invoice, self.client, amount),
                )
                .unwrap();
        }
    }

    #[test]
    fn stores_items_and_totals() {
        let f = Fixture::new();
        let mut input = f.input(" INV-1 ", StoredStatus::Draft, None);
        input.items.push(item(2500, money(1_000, "RUB")));
        input.tax_bp = 2000;
        let inv = f.repo().create(input).unwrap();
        assert_eq!(inv.number, "INV-1");
        assert_eq!(inv.client_name, "Ромашка");
        assert_eq!(inv.items.len(), 2);
        assert_eq!(inv.items[1].amount, money(2_500, "RUB"));
        assert_eq!(inv.totals.total, money(123_000, "RUB"));
        assert_eq!(inv.totals.due, money(123_000, "RUB"));
        assert_eq!(inv.status, InvoiceStatus::Draft);
        assert_eq!(f.repo().get(inv.id).unwrap(), inv);
    }

    #[test]
    fn validates_invoices() {
        let f = Fixture::new();
        let check = |patch: &dyn Fn(&mut InvoiceInput), err: CoreError| {
            let mut input = f.input("INV-X", StoredStatus::Draft, None);
            patch(&mut input);
            assert_eq!(f.repo().create(input).unwrap_err(), err);
        };
        check(
            &|i| i.issue_date = "2026-02-30".into(),
            CoreError::validation("issueDate", "date"),
        );
        check(
            &|i| i.due_date = Some("2026-02-01".into()),
            CoreError::validation("dueDate", "before_start"),
        );
        check(
            &|i| i.items[0].quantity_milli = 0,
            CoreError::validation("items.0.quantity", "min"),
        );
        check(
            &|i| i.items[0].price = money(-1, "RUB"),
            CoreError::validation("items.0.price", "min"),
        );
        check(
            &|i| i.tax_bp = 10_001,
            CoreError::validation("taxBp", "range"),
        );
        check(
            &|i| i.discount = Discount::Percent(-5),
            CoreError::validation("discount", "range"),
        );
        check(
            &|i| i.language = "de".into(),
            CoreError::validation("language", "option"),
        );
        f.repo()
            .create(f.input("INV-1", StoredStatus::Draft, None))
            .unwrap();
        check(
            &|i| i.number = "INV-1".into(),
            CoreError::new("invoice.number_taken"),
        );
    }

    #[test]
    fn status_follows_payments_and_the_due_date() {
        let f = Fixture::new();
        let moscow = TimeZone::get("Europe/Moscow").unwrap();
        let status = |id| f.repo().in_zone(moscow.clone()).get(id).unwrap().status;
        // Due on the 10th: overdue in Moscow (already the 11th), not in UTC.
        let sent = f
            .repo()
            .create(f.input("A", StoredStatus::Sent, Some("2026-03-10")))
            .unwrap();
        assert_eq!(status(sent.id), InvoiceStatus::Overdue);
        assert_eq!(f.repo().get(sent.id).unwrap().status, InvoiceStatus::Sent);
        f.pay(sent.id, 40_000);
        assert_eq!(status(sent.id), InvoiceStatus::PartiallyPaid);
        assert_eq!(
            f.repo().get(sent.id).unwrap().totals.due,
            money(60_000, "RUB")
        );
        f.pay(sent.id, 60_000);
        assert_eq!(status(sent.id), InvoiceStatus::Paid);
        // Drafts are never overdue; cancelled stays cancelled.
        let draft = f
            .repo()
            .create(f.input("B", StoredStatus::Draft, Some("2026-03-01")))
            .unwrap();
        assert_eq!(status(draft.id), InvoiceStatus::Draft);
        let cancelled = f
            .repo()
            .create(f.input("C", StoredStatus::Cancelled, Some("2026-03-01")))
            .unwrap();
        assert_eq!(status(cancelled.id), InvoiceStatus::Cancelled);
    }

    #[test]
    fn paid_and_cancelled_invoices_are_locked() {
        let f = Fixture::new();
        let inv = f
            .repo()
            .create(f.input("A", StoredStatus::Sent, None))
            .unwrap();
        f.pay(inv.id, 1_000);
        let locked = CoreError::new("invoice.locked");
        assert_eq!(
            f.repo()
                .update(inv.id, f.input("A", StoredStatus::Sent, None))
                .unwrap_err(),
            locked
        );
        assert_eq!(f.repo().delete(inv.id).unwrap_err(), locked);
        let draft = f
            .repo()
            .create(f.input("B", StoredStatus::Draft, None))
            .unwrap();
        let sent = f
            .repo()
            .update(draft.id, f.input("B", StoredStatus::Sent, None))
            .unwrap();
        assert_eq!(sent.status, InvoiceStatus::Sent);
        f.repo().delete(draft.id).unwrap();
        assert_eq!(
            f.repo().get(draft.id).unwrap_err(),
            CoreError::not_found("invoice", draft.id)
        );
    }

    #[test]
    fn checks_dates_discounts_projects_and_overflow() {
        use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};
        let f = Fixture::new();
        let mut bad = f.input("X", StoredStatus::Draft, Some("2026-13-01"));
        assert_eq!(
            f.repo().create(bad.clone()).unwrap_err(),
            CoreError::validation("dueDate", "date")
        );
        bad.due_date = None;
        bad.discount = Discount::Fixed(-1);
        assert_eq!(
            f.repo().create(bad).unwrap_err(),
            CoreError::validation("discount", "range")
        );

        let project = |client: Id| {
            ProjectRepo::new(f.db.conn(), &f.clock)
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
                .id
        };
        let other = ClientRepo::new(f.db.conn(), &f.clock)
            .create(ClientInput {
                kind: ClientKind::Person,
                last_name: Some("Иванов".into()),
                first_name: None,
                middle_name: None,
                company_name: None,
                legal_name: None,
                status: ClientStatus::Active,
                default_currency: None,
                default_rate_minor: None,
            })
            .unwrap()
            .id;
        let mut input = f.input("P", StoredStatus::Draft, None);
        input.project_id = Some(project(other));
        assert_eq!(
            f.repo().create(input.clone()).unwrap_err(),
            CoreError::new("invoice.project_mismatch")
        );
        let missing = Id::new();
        input.project_id = Some(missing);
        assert_eq!(
            f.repo().create(input.clone()).unwrap_err(),
            CoreError::not_found("project", missing)
        );
        input.project_id = Some(project(f.client));
        input.discount = Discount::Percent(1500);
        let saved = f.repo().create(input).unwrap();
        assert_eq!(saved.project_name.as_deref(), Some("Сайт"));
        assert_eq!(saved.discount, Discount::Percent(1500));
        assert_eq!(saved.totals.discount, money(15_000, "RUB"));
        let mut fixed = f.input("P", StoredStatus::Draft, None);
        fixed.project_id = saved.project_id;
        fixed.discount = Discount::Fixed(5_000);
        let saved = f.repo().update(saved.id, fixed).unwrap();
        assert_eq!(saved.discount, Discount::Fixed(5_000));
        assert_eq!(saved.totals.total, money(95_000, "RUB"));

        let mut huge = f.input("H", StoredStatus::Draft, None);
        huge.items[0].price = money(i64::MAX, "RUB");
        huge.items[0].quantity_milli = 2000;
        assert_eq!(
            f.repo().create(huge).unwrap_err(),
            CoreError::new("money.overflow")
        );
    }

    #[test]
    fn numbers_new_invoices_from_the_template() {
        let f = Fixture::new();
        let first = f
            .repo()
            .create(f.input("", StoredStatus::Draft, None))
            .unwrap();
        let second = f
            .repo()
            .create(f.input("  ", StoredStatus::Draft, None))
            .unwrap();
        assert_eq!(
            (first.number.as_str(), second.number.as_str()),
            ("INV-2026-001", "INV-2026-002")
        );
        assert_eq!(
            f.repo()
                .update(first.id, f.input("", StoredStatus::Draft, None))
                .unwrap_err(),
            CoreError::validation("number", "required")
        );
    }

    #[test]
    fn lists_newest_first() {
        let f = Fixture::new();
        let mut older = f.input("A-1", StoredStatus::Sent, None);
        older.issue_date = "2026-02-01".into();
        f.repo().create(older).unwrap();
        f.repo()
            .create(f.input("A-2", StoredStatus::Draft, None))
            .unwrap();
        let numbers: Vec<String> = f
            .repo()
            .list(Some(f.client), None)
            .unwrap()
            .into_iter()
            .map(|i| i.number)
            .collect();
        assert_eq!(numbers, ["A-2", "A-1"]);
        assert!(f.repo().list(Some(Id::new()), None).unwrap().is_empty());
    }

    #[test]
    fn clients_with_invoices_cannot_be_deleted() {
        let f = Fixture::new();
        f.repo()
            .create(f.input("A", StoredStatus::Draft, None))
            .unwrap();
        let clients = ClientRepo::new(f.db.conn(), &f.clock);
        assert_eq!(clients.delete_impact(f.client).unwrap().invoices, 1);
        assert_eq!(
            clients.delete(f.client).unwrap_err(),
            CoreError::new("client.has_invoices")
        );
    }
}
