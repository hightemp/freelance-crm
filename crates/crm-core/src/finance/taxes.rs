//! Taxes (FR-FN-10), an estimate rather than accounting: when enabled,
//! received payments are taxed by the regime in force on their date. A
//! regime is a list of rules "rate if the client is a person/company
//! and/or the payment is in a currency"; the first matching rule applies.
//! Presets come from the country (NPD and USN for Russia) or "custom".

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::clients::ClientKind;
use crate::domain::{Clock, Currency, Id, Money, Timestamp};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaxRule {
    /// Basis points: 600 = 6 %.
    #[specta(type = i32)]
    pub rate_bp: i64,
    /// Only payments from this kind of client; `None` for any.
    pub client_kind: Option<ClientKind>,
    /// Only payments in this currency; `None` for any.
    pub currency: Option<Currency>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaxRegimeInput {
    pub name: String,
    /// `npd`, `usn_income` or `custom`.
    pub preset: String,
    /// The regime applies to payments from this date (`YYYY-MM-DD`).
    pub valid_from: String,
    pub rules: Vec<TaxRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaxRegime {
    pub id: Id,
    #[serde(flatten)]
    pub input: TaxRegimeInput,
    pub created_at: Timestamp,
}

/// Received money and estimated tax in one currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaxLine {
    pub currency: Currency,
    #[specta(type = i32)]
    pub income_minor: i64,
    #[specta(type = i32)]
    pub tax_minor: i64,
    #[specta(type = i32)]
    pub net_minor: i64,
}

/// A preset regime to start from; rates are editable.
pub fn preset(key: &str, valid_from: &str) -> Option<TaxRegimeInput> {
    let rule = |rate_bp, client_kind| TaxRule {
        rate_bp,
        client_kind,
        currency: None,
    };
    let (name, rules) = match key {
        "npd" => (
            "НПД",
            vec![
                rule(400, Some(ClientKind::Person)),
                rule(600, Some(ClientKind::Company)),
            ],
        ),
        "usn_income" => ("УСН «Доходы»", vec![rule(600, None)]),
        "custom" => ("", vec![rule(0, None)]),
        _ => return None,
    };
    Some(TaxRegimeInput {
        name: name.into(),
        preset: key.into(),
        valid_from: valid_from.into(),
        rules,
    })
}

pub struct TaxRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> TaxRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    /// Oldest first.
    pub fn regimes(&self) -> CoreResult<Vec<TaxRegime>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, preset, valid_from, rules, created_at FROM tax_regimes
             ORDER BY valid_from",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(TaxRegime {
                id: r.get(0)?,
                input: TaxRegimeInput {
                    name: r.get(1)?,
                    preset: r.get(2)?,
                    valid_from: r.get(3)?,
                    rules: serde_json::from_str(&r.get::<_, String>(4)?).unwrap_or_default(),
                },
                created_at: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    fn get(&self, id: Id) -> CoreResult<TaxRegime> {
        self.regimes()?
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| CoreError::not_found("tax_regime", id))
    }

    pub fn save(&self, id: Option<Id>, input: TaxRegimeInput) -> CoreResult<TaxRegime> {
        if !crate::domain::date::is_iso_date(&input.valid_from) {
            return Err(CoreError::validation("validFrom", "date"));
        }
        if input.rules.is_empty() {
            return Err(CoreError::validation("rules", "required"));
        }
        for (i, rule) in input.rules.iter().enumerate() {
            if !(0..=10_000).contains(&rule.rate_bp) {
                return Err(CoreError::validation(&format!("rules.{i}.rateBp"), "range"));
            }
        }
        if let Some(id) = id {
            self.get(id)?;
        }
        let taken: bool = self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM tax_regimes WHERE valid_from = ?1 AND id IS NOT ?2)",
            (&input.valid_from, id),
            |r| r.get(0),
        )?;
        if taken {
            return Err(CoreError::new("tax.duplicate_date"));
        }
        let id = id.unwrap_or_default();
        self.conn.execute(
            "INSERT INTO tax_regimes (id, name, preset, valid_from, rules, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name, preset = excluded.preset,
                 valid_from = excluded.valid_from, rules = excluded.rules",
            rusqlite::params![
                id,
                input.name.trim(),
                input.preset,
                input.valid_from,
                serde_json::to_string(&input.rules).expect("rules serialize"),
                self.clock.now(),
            ],
        )?;
        self.get(id)
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        if self
            .conn
            .execute("DELETE FROM tax_regimes WHERE id = ?1", [id])?
            == 0
        {
            return Err(CoreError::not_found("tax_regime", id));
        }
        Ok(())
    }

    /// Tax on payments received in `[from, to]` (dates); `None` when taxes
    /// are switched off in the settings.
    pub fn estimate(&self, from: &str, to: &str) -> CoreResult<Option<Vec<TaxLine>>> {
        if !crate::settings::load_from(self.conn)?.taxes_enabled {
            return Ok(None);
        }
        let regimes = self.regimes()?;
        let mut stmt = self.conn.prepare(
            "SELECT pm.amount_minor, pm.currency, pm.paid_on, c.kind
             FROM payments pm JOIN clients c ON c.id = pm.client_id
             WHERE pm.paid_on BETWEEN ?1 AND ?2",
        )?;
        let payments = stmt
            .query_map((from, to), |r| {
                Ok((
                    Money::new(r.get(0)?, r.get(1)?),
                    r.get::<_, String>(2)?,
                    r.get::<_, ClientKind>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut lines: std::collections::BTreeMap<String, TaxLine> = Default::default();
        for (amount, paid_on, kind) in payments {
            // The regime in force on the payment's date.
            let regime = regimes.iter().rev().find(|r| r.input.valid_from <= paid_on);
            let rule = regime.and_then(|r| {
                r.input.rules.iter().find(|rule| {
                    rule.client_kind.is_none_or(|k| k == kind)
                        && rule.currency.is_none_or(|c| c == amount.currency)
                })
            });
            let tax = match rule {
                Some(rule) => {
                    amount
                        .percent_bp(rule.rate_bp)
                        .map_err(|_| CoreError::new("money.overflow"))?
                        .amount_minor
                }
                None => 0,
            };
            let line = lines
                .entry(amount.currency.code().to_owned())
                .or_insert(TaxLine {
                    currency: amount.currency,
                    income_minor: 0,
                    tax_minor: 0,
                    net_minor: 0,
                });
            line.income_minor += amount.amount_minor;
            line.tax_minor += tax;
            line.net_minor = line.income_minor - line.tax_minor;
        }
        Ok(Some(lines.into_values().collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};

    fn money(minor: i64, code: &str) -> Money {
        Money::new(minor, code.parse().unwrap())
    }

    struct Fixture {
        db: Db,
        clock: FakeClock,
        person: Id,
        company: Id,
    }

    impl Fixture {
        /// A person and a company, each with a project; taxes enabled.
        fn new() -> Self {
            let db = Db::open_in_memory().unwrap();
            let clock = FakeClock::at(Timestamp::from_millis(1_773_000_000_000));
            let project = |kind: ClientKind, name: &str| {
                let client = ClientRepo::new(db.conn(), &clock)
                    .create(ClientInput {
                        kind,
                        last_name: (kind == ClientKind::Person).then(|| name.to_owned()),
                        first_name: None,
                        middle_name: None,
                        company_name: (kind == ClientKind::Company).then(|| name.to_owned()),
                        legal_name: None,
                        status: ClientStatus::Active,
                        default_currency: None,
                        default_rate_minor: None,
                    })
                    .unwrap()
                    .id;
                ProjectRepo::new(db.conn(), &clock)
                    .create(ProjectInput {
                        client_id: client,
                        name: format!("Проект {name}"),
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
            let person = project(ClientKind::Person, "Иванов");
            let company = project(ClientKind::Company, "Ромашка");
            db.conn()
                .execute(
                    "INSERT INTO settings (key, value) VALUES ('taxesEnabled', 'true')",
                    [],
                )
                .unwrap();
            Self {
                db,
                clock,
                person,
                company,
            }
        }

        fn repo(&self) -> TaxRepo<'_> {
            TaxRepo::new(self.db.conn(), &self.clock)
        }

        fn pay(&self, project: Id, amount: Money, date: &str) {
            crate::finance::payments::PaymentRepo::new(self.db.conn(), &self.clock)
                .create(crate::finance::payments::PaymentInput {
                    invoice_id: None,
                    project_id: Some(project),
                    amount,
                    paid_on: date.into(),
                    method: String::new(),
                    note: String::new(),
                })
                .unwrap();
        }
    }

    fn line(code: &str, income: i64, tax: i64) -> TaxLine {
        TaxLine {
            currency: code.parse().unwrap(),
            income_minor: income,
            tax_minor: tax,
            net_minor: income - tax,
        }
    }

    #[test]
    fn npd_taxes_people_at_4_and_companies_at_6_percent() {
        let f = Fixture::new();
        f.repo()
            .save(None, preset("npd", "2026-01-01").unwrap())
            .unwrap();
        f.pay(f.person, money(100_000, "RUB"), "2026-03-02");
        f.pay(f.company, money(100_000, "RUB"), "2026-03-03");
        assert_eq!(
            f.repo().estimate("2026-03-01", "2026-03-31").unwrap(),
            Some(vec![line("RUB", 200_000, 4_000 + 6_000)])
        );
    }

    #[test]
    fn usn_rate_is_editable_and_rounding_is_per_payment() {
        let f = Fixture::new();
        let mut usn = preset("usn_income", "2026-01-01").unwrap();
        assert_eq!(
            usn.rules,
            [TaxRule {
                rate_bp: 600,
                client_kind: None,
                currency: None
            }]
        );
        usn.rules[0].rate_bp = 400;
        f.repo().save(None, usn).unwrap();
        // 4 % of 13 kopecks = 0.52 → 1 kopeck each: 2 in all, not 1.04 → 1.
        f.pay(f.person, money(13, "RUB"), "2026-03-02");
        f.pay(f.person, money(13, "RUB"), "2026-03-02");
        assert_eq!(
            f.repo().estimate("2026-03-01", "2026-03-31").unwrap(),
            Some(vec![line("RUB", 26, 2)])
        );
        f.pay(f.person, money(100_037, "RUB"), "2026-03-03");
        // 4 % of 1 000.37 = 40.0148 → 40.01.
        assert_eq!(
            f.repo().estimate("2026-03-03", "2026-03-03").unwrap(),
            Some(vec![line("RUB", 100_037, 4_001)])
        );
    }

    #[test]
    fn custom_rules_by_currency_and_one_line_per_currency() {
        let f = Fixture::new();
        f.repo()
            .save(
                None,
                TaxRegimeInput {
                    name: "Свой".into(),
                    preset: "custom".into(),
                    valid_from: "2026-01-01".into(),
                    rules: vec![
                        TaxRule {
                            rate_bp: 1000,
                            client_kind: None,
                            currency: Some("USD".parse().unwrap()),
                        },
                        TaxRule {
                            rate_bp: 1300,
                            client_kind: None,
                            currency: None,
                        },
                    ],
                },
            )
            .unwrap();
        f.pay(f.company, money(10_000, "USD"), "2026-03-02");
        f.pay(f.company, money(100_000, "RUB"), "2026-03-02");
        assert_eq!(
            f.repo().estimate("2026-03-01", "2026-03-31").unwrap(),
            Some(vec![
                line("RUB", 100_000, 13_000),
                line("USD", 10_000, 1_000)
            ])
        );
    }

    #[test]
    fn a_new_regime_applies_from_its_date_only() {
        let f = Fixture::new();
        f.repo()
            .save(None, preset("npd", "2026-01-01").unwrap())
            .unwrap();
        f.repo()
            .save(None, preset("usn_income", "2026-07-01").unwrap())
            .unwrap();
        f.pay(f.person, money(100_000, "RUB"), "2026-06-30");
        f.pay(f.person, money(100_000, "RUB"), "2026-07-01");
        assert_eq!(
            f.repo().estimate("2026-01-01", "2026-12-31").unwrap(),
            Some(vec![line("RUB", 200_000, 4_000 + 6_000)])
        );
        // Before any regime nothing is taxed.
        f.pay(f.person, money(50_000, "RUB"), "2025-12-31");
        assert_eq!(
            f.repo().estimate("2025-12-01", "2025-12-31").unwrap(),
            Some(vec![line("RUB", 50_000, 0)])
        );
    }

    #[test]
    fn switched_off_taxes_give_no_figures_at_all() {
        let f = Fixture::new();
        f.repo()
            .save(None, preset("npd", "2026-01-01").unwrap())
            .unwrap();
        f.pay(f.person, money(100_000, "RUB"), "2026-03-02");
        f.db.conn()
            .execute(
                "UPDATE settings SET value = 'false' WHERE key = 'taxesEnabled'",
                [],
            )
            .unwrap();
        assert_eq!(f.repo().estimate("2026-03-01", "2026-03-31").unwrap(), None);
    }

    #[test]
    fn regimes_are_validated_listed_and_deleted() {
        let f = Fixture::new();
        let mut bad = preset("usn_income", "2026-01-01").unwrap();
        bad.rules[0].rate_bp = 10_001;
        assert_eq!(
            f.repo().save(None, bad).unwrap_err(),
            CoreError::validation("rules.0.rateBp", "range")
        );
        assert_eq!(
            f.repo()
                .save(
                    None,
                    TaxRegimeInput {
                        rules: vec![],
                        ..preset("usn_income", "2026-01-01").unwrap()
                    }
                )
                .unwrap_err(),
            CoreError::validation("rules", "required")
        );
        assert_eq!(
            f.repo()
                .save(None, preset("usn_income", "2026-13-01").unwrap())
                .unwrap_err(),
            CoreError::validation("validFrom", "date")
        );
        assert_eq!(preset("unknown", "2026-01-01"), None);
        let later = f
            .repo()
            .save(None, preset("usn_income", "2026-07-01").unwrap())
            .unwrap();
        let earlier = f
            .repo()
            .save(None, preset("npd", "2026-01-01").unwrap())
            .unwrap();
        assert_eq!(
            f.repo()
                .regimes()
                .unwrap()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
            [earlier.id, later.id]
        );
        assert_eq!(
            f.repo()
                .save(None, preset("npd", "2026-01-01").unwrap())
                .unwrap_err(),
            CoreError::new("tax.duplicate_date")
        );
        let mut renamed = later.input.clone();
        renamed.name = "УСН 4%".into();
        renamed.rules[0].rate_bp = 400;
        let saved = f.repo().save(Some(later.id), renamed).unwrap();
        assert_eq!(
            (saved.input.name.as_str(), saved.input.rules[0].rate_bp),
            ("УСН 4%", 400)
        );
        f.repo().delete(later.id).unwrap();
        assert_eq!(f.repo().regimes().unwrap().len(), 1);
        assert_eq!(
            f.repo().delete(later.id).unwrap_err(),
            CoreError::not_found("tax_regime", later.id)
        );
    }
}
