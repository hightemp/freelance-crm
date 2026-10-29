//! The data an invoice PDF is rendered from (FR-FN-5, FR-CO-3): template by
//! the freelancer's country, labels in the invoice's language, both
//! parties' requisites (frozen when issued), formatted amounts and, for
//! Russian invoices, the VAT note and the total in words.

use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::Serialize;

use super::invoices::{Discount, InvoiceRepo};
use crate::CoreResult;
use crate::domain::{Clock, Id};
use crate::profile_images::{ImageKind, ProfileImages};
use crate::requisites::Requisites;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Line {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Party {
    pub name: String,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DocItem {
    pub n: usize,
    pub description: String,
    pub quantity: String,
    pub unit: String,
    pub price: String,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InvoiceDocument {
    /// `international` or `ru_invoice`.
    pub template: &'static str,
    pub lang: String,
    pub labels: BTreeMap<&'static str, &'static str>,
    pub title: String,
    pub number: String,
    pub issue_date: String,
    pub due_date: Option<String>,
    pub seller: Party,
    pub buyer: Party,
    pub items: Vec<DocItem>,
    /// Shown next to totals: `руб.` for rubles in Russian, else the code.
    pub currency: String,
    pub subtotal: String,
    pub discount: Option<String>,
    pub tax: Option<Line>,
    pub total: String,
    pub vat_note: Option<String>,
    pub amount_in_words: Option<String>,
    pub notes: String,
    pub logo: bool,
    pub signature: bool,
    pub stamp: bool,
}

/// Builds the document of an invoice; `show_signature` is the setting
/// that the invoice may override.
pub fn document(
    conn: &Connection,
    clock: &dyn Clock,
    invoice_id: Id,
    show_signature: bool,
) -> CoreResult<InvoiceDocument> {
    let repo = InvoiceRepo::new(conn, clock);
    let invoice = repo.get(invoice_id)?;
    let snapshot = repo.snapshot(invoice_id)?;
    let ru = invoice.language == "ru";
    let labels = if ru { labels_ru() } else { labels_en() };
    let seller_country = snapshot.seller.as_ref().map(|r| r.country.as_str());
    let template = if seller_country == Some("RU") {
        "ru_invoice"
    } else {
        "international"
    };
    let units = invoice.currency.minor_units();
    let fmt = |m: crate::domain::Money| amount(m.amount_minor, units, ru);
    let issue_date = date(&invoice.issue_date, ru);
    let title = match (template, ru) {
        ("ru_invoice", true) => format!("Счёт на оплату № {} от {issue_date}", invoice.number),
        ("ru_invoice", false) => {
            format!("Invoice for payment No. {} of {issue_date}", invoice.number)
        }
        (_, true) => format!("Счёт № {}", invoice.number),
        (_, false) => format!("Invoice {}", invoice.number),
    };
    let is_rub = invoice.currency.code() == "RUB";
    let t = invoice.totals;
    let images = ProfileImages::new(conn, clock);
    let has = |kind| images.get(kind).map(|i| i.is_some());
    let wants_signature = invoice.show_signature.unwrap_or(show_signature);
    let seller_form = snapshot.seller.as_ref().map(|r| r.legal_form.as_str());
    let vat_note = match (template, invoice.tax_bp, seller_form) {
        ("ru_invoice", 0, Some("self_employed")) => Some(
            "Без НДС. Исполнитель применяет налог на профессиональный доход \
             (Федеральный закон от 27.11.2018 № 422-ФЗ)."
                .to_owned(),
        ),
        ("ru_invoice", 0, Some("ip" | "ooo")) => Some(
            "Без НДС на основании п. 2 ст. 346.11 НК РФ (упрощённая система налогообложения)."
                .to_owned(),
        ),
        _ => None,
    };
    Ok(InvoiceDocument {
        template,
        lang: invoice.language.clone(),
        title,
        number: invoice.number.clone(),
        due_date: invoice.due_date.as_deref().map(|d| date(d, ru)),
        issue_date,
        seller: party(
            snapshot.seller.as_ref(),
            snapshot
                .seller
                .as_ref()
                .and_then(|r| r.name.clone())
                .unwrap_or_default(),
            &invoice.language,
        ),
        buyer: party(
            snapshot.buyer.as_ref(),
            snapshot.buyer_name.clone(),
            &invoice.language,
        ),
        items: invoice
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| DocItem {
                n: i + 1,
                description: item.description.clone(),
                quantity: quantity(item.quantity_milli, ru),
                unit: item.unit.clone(),
                price: fmt(item.price),
                amount: fmt(item.amount),
            })
            .collect(),
        currency: if ru && is_rub {
            "руб.".into()
        } else {
            invoice.currency.code().to_owned()
        },
        subtotal: fmt(t.subtotal),
        discount: (!matches!(invoice.discount, Discount::None) && t.discount.amount_minor > 0)
            .then(|| fmt(t.discount)),
        tax: (invoice.tax_bp > 0).then(|| Line {
            label: format!("{} {}%", labels["tax"], percent(invoice.tax_bp, ru)),
            value: fmt(t.tax),
        }),
        total: fmt(t.total),
        amount_in_words: (template == "ru_invoice" && is_rub)
            .then(|| super::words::rubles_in_words(t.total.amount_minor)),
        vat_note,
        notes: invoice.notes_md.clone(),
        logo: has(ImageKind::Logo)?,
        signature: wants_signature && has(ImageKind::Signature)?,
        stamp: wants_signature && has(ImageKind::Stamp)?,
        labels,
    })
}

/// A party's name and requisites, labelled from the country schema in
/// the schema's order; empty values are left out.
fn party(requisites: Option<&Requisites>, name: String, lang: &str) -> Party {
    let lines = requisites
        .and_then(|r| {
            let schema = crate::countries::get(&r.country)?;
            let form = schema.legal_form(&r.legal_form)?;
            Some(
                form.fields
                    .iter()
                    .filter_map(|f| {
                        let value = r.values.get(&f.key).filter(|v| !v.trim().is_empty())?;
                        let def = schema.field(&f.key)?;
                        let label = def.labels.get(lang).or_else(|| def.labels.get("en"))?;
                        Some(Line {
                            label: label.clone(),
                            value: value.clone(),
                        })
                    })
                    .collect(),
            )
        })
        .unwrap_or_default();
    Party { name, lines }
}

/// `1 234,56` (Russian, no-break space) or `1,234.56`.
fn amount(minor: i64, units: u8, ru: bool) -> String {
    let scale = 10i64.pow(u32::from(units));
    let (whole, part) = (minor.abs() / scale, minor.abs() % scale);
    let digits = whole.to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(if ru { '\u{a0}' } else { ',' });
        }
        grouped.push(c);
    }
    let sign = if minor < 0 { "-" } else { "" };
    if units == 0 {
        format!("{sign}{grouped}")
    } else {
        let sep = if ru { ',' } else { '.' };
        format!(
            "{sign}{grouped}{sep}{part:0width$}",
            width = usize::from(units)
        )
    }
}

/// Thousandths without trailing zeros: `1,5` or `1.5`.
fn quantity(milli: i64, ru: bool) -> String {
    let text = format!("{}.{:03}", milli / 1000, milli % 1000);
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if ru {
        text.replace('.', ",")
    } else {
        text.to_owned()
    }
}

/// `20` or `12,5` for basis points.
fn percent(bp: i64, ru: bool) -> String {
    let text = format!("{}.{:02}", bp / 100, bp % 100);
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if ru {
        text.replace('.', ",")
    } else {
        text.to_owned()
    }
}

/// `01.03.2026` or `Mar 1, 2026`.
fn date(iso: &str, ru: bool) -> String {
    let Ok(d) = iso.parse::<jiff::civil::Date>() else {
        return iso.to_owned();
    };
    if ru {
        d.strftime("%d.%m.%Y").to_string()
    } else {
        d.strftime("%b %-d, %Y").to_string()
    }
}

fn labels_ru() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("seller", "Исполнитель"),
        ("buyer", "Заказчик"),
        ("date", "Дата"),
        ("due", "Оплатить до"),
        ("n", "№"),
        ("description", "Наименование"),
        ("quantity", "Кол-во"),
        ("unit", "Ед."),
        ("price", "Цена"),
        ("amount", "Сумма"),
        ("subtotal", "Итого"),
        ("discount", "Скидка"),
        ("tax", "Налог"),
        ("total", "Всего к оплате"),
        ("in_words", "Сумма прописью"),
        ("notes", "Примечание"),
        ("signature", "Исполнитель"),
    ])
}

fn labels_en() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("seller", "From"),
        ("buyer", "Bill to"),
        ("date", "Date"),
        ("due", "Due date"),
        ("n", "#"),
        ("description", "Description"),
        ("quantity", "Qty"),
        ("unit", "Unit"),
        ("price", "Price"),
        ("amount", "Amount"),
        ("subtotal", "Subtotal"),
        ("discount", "Discount"),
        ("tax", "Tax"),
        ("total", "Total due"),
        ("in_words", "Amount in words"),
        ("notes", "Notes"),
        ("signature", "Authorized signature"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::{FakeClock, Money, Timestamp};
    use crate::finance::invoices::{Invoice, InvoiceInput, InvoiceItemInput, StoredStatus};
    use crate::requisites::{RequisitesInput, RequisitesOwner, RequisitesService, SaveRequisites};

    struct Fixture {
        db: Db,
        clock: FakeClock,
        client: Id,
    }

    fn values(pairs: &[(&str, &str)]) -> crate::countries::Values {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    impl Fixture {
        /// A self-employed freelancer in Russia billing ООО «Ромашка».
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
                    legal_name: Some("ООО «Ромашка»".into()),
                    status: ClientStatus::Active,
                    default_currency: None,
                    default_rate_minor: None,
                })
                .unwrap()
                .id;
            let f = Self { db, clock, client };
            f.profile(
                "RU",
                "self_employed",
                "Иванов Иван Иванович",
                &[("inn", "500100732259"), ("bank_name", "АО «Банк»")],
            );
            f.requisites(
                RequisitesOwner::Client(client),
                "RU",
                "ooo",
                None,
                &[
                    ("inn", "7707083893"),
                    ("kpp", "773601001"),
                    ("ogrn", "1027700132195"),
                    ("address", "Москва, ул. Вавилова, 19"),
                ],
            );
            f
        }

        fn requisites(
            &self,
            owner: RequisitesOwner,
            country: &str,
            form: &str,
            name: Option<&str>,
            pairs: &[(&str, &str)],
        ) {
            let saved = RequisitesService::new(self.db.conn(), &self.clock)
                .save(
                    owner,
                    RequisitesInput {
                        country: country.into(),
                        legal_form: form.into(),
                        name: name.map(Into::into),
                        values: values(pairs),
                        confirm_country_change: true,
                    },
                )
                .unwrap();
            assert!(matches!(saved, SaveRequisites::Saved { .. }), "{saved:?}");
        }

        fn profile(&self, country: &str, form: &str, name: &str, pairs: &[(&str, &str)]) {
            self.requisites(RequisitesOwner::Profile, country, form, Some(name), pairs);
        }

        fn invoice(&self, status: StoredStatus, language: &str, currency: &str) -> Invoice {
            InvoiceRepo::new(self.db.conn(), &self.clock)
                .create(InvoiceInput {
                    number: String::new(),
                    client_id: self.client,
                    project_id: None,
                    currency: currency.parse().unwrap(),
                    issue_date: "2026-03-01".into(),
                    due_date: Some("2026-03-15".into()),
                    status,
                    items: vec![InvoiceItemInput {
                        description: "Разработка сайта".into(),
                        quantity_milli: 1500,
                        unit: "ч".into(),
                        price: Money::new(123_456, currency.parse().unwrap()),
                    }],
                    discount: Discount::None,
                    tax_bp: 0,
                    notes_md: "Спасибо!".into(),
                    language: language.into(),
                    show_signature: None,
                })
                .unwrap()
        }

        fn doc(&self, invoice: &Invoice, show_signature: bool) -> InvoiceDocument {
            document(self.db.conn(), &self.clock, invoice.id, show_signature).unwrap()
        }
    }

    #[test]
    fn russian_invoice_for_payment() {
        let f = Fixture::new();
        let inv = f.invoice(StoredStatus::Sent, "ru", "RUB");
        let d = f.doc(&inv, false);
        assert_eq!(d.template, "ru_invoice");
        assert_eq!(d.title, "Счёт на оплату № INV-2026-001 от 01.03.2026");
        assert_eq!(
            (d.issue_date.as_str(), d.due_date.as_deref()),
            ("01.03.2026", Some("15.03.2026"))
        );
        assert_eq!(d.seller.name, "Иванов Иван Иванович");
        assert_eq!(
            d.seller.lines,
            [
                Line {
                    label: "ИНН".into(),
                    value: "500100732259".into()
                },
                Line {
                    label: "Наименование банка".into(),
                    value: "АО «Банк»".into()
                },
            ]
        );
        assert_eq!(d.buyer.name, "ООО «Ромашка»");
        assert_eq!(
            d.buyer.lines[0],
            Line {
                label: "ИНН".into(),
                value: "7707083893".into()
            }
        );
        assert_eq!(d.buyer.lines.len(), 4);
        assert_eq!(
            d.items,
            [DocItem {
                n: 1,
                description: "Разработка сайта".into(),
                quantity: "1,5".into(),
                unit: "ч".into(),
                price: "1\u{a0}234,56".into(),
                amount: "1\u{a0}851,84".into(),
            }]
        );
        assert_eq!(
            (d.currency.as_str(), d.total.as_str()),
            ("руб.", "1\u{a0}851,84")
        );
        assert_eq!(
            d.amount_in_words.as_deref(),
            Some("Одна тысяча восемьсот пятьдесят один рубль 84 копейки")
        );
        assert_eq!(
            d.vat_note.as_deref(),
            Some(
                "Без НДС. Исполнитель применяет налог на профессиональный доход (Федеральный закон от 27.11.2018 № 422-ФЗ)."
            )
        );
        assert_eq!(d.labels["buyer"], "Заказчик");
        assert_eq!(d.notes, "Спасибо!");
        assert!(!d.logo && !d.signature && !d.stamp);
    }

    #[test]
    fn simplified_tax_has_its_own_vat_note() {
        let f = Fixture::new();
        f.profile(
            "RU",
            "ip",
            "ИП Иванов И. И.",
            &[
                ("inn", "500100732259"),
                ("ogrnip", "304500116000157"),
                ("address", "Москва"),
            ],
        );
        let d = f.doc(&f.invoice(StoredStatus::Draft, "ru", "RUB"), false);
        assert_eq!(
            d.vat_note.as_deref(),
            Some(
                "Без НДС на основании п. 2 ст. 346.11 НК РФ (упрощённая система налогообложения)."
            )
        );
    }

    #[test]
    fn international_invoice_and_languages() {
        let f = Fixture::new();
        f.profile("INTL", "individual", "John Smith", &[]);
        let en = f.doc(&f.invoice(StoredStatus::Draft, "en", "USD"), false);
        assert_eq!(en.template, "international");
        assert_eq!(en.title, "Invoice INV-2026-001");
        assert_eq!(
            (en.issue_date.as_str(), en.labels["buyer"]),
            ("Mar 1, 2026", "Bill to")
        );
        assert_eq!(
            (en.items[0].quantity.as_str(), en.items[0].price.as_str()),
            ("1.5", "1,234.56")
        );
        assert_eq!(
            (
                en.currency.as_str(),
                en.amount_in_words.as_ref(),
                en.vat_note.as_ref()
            ),
            ("USD", None, None)
        );
        // The same template speaks Russian on a Russian invoice.
        let ru = f.doc(&f.invoice(StoredStatus::Draft, "ru", "USD"), false);
        assert_eq!(
            (ru.template, ru.title.as_str(), ru.labels["buyer"]),
            ("international", "Счёт № INV-2026-002", "Заказчик")
        );
        assert_eq!(ru.currency, "USD");
    }

    #[test]
    fn issued_invoices_keep_the_requisites_they_had() {
        let f = Fixture::new();
        let sent = f.invoice(StoredStatus::Sent, "ru", "RUB");
        let draft = f.invoice(StoredStatus::Draft, "ru", "RUB");
        f.profile(
            "RU",
            "self_employed",
            "Петров Пётр Петрович",
            &[("inn", "500100732259")],
        );
        assert_eq!(f.doc(&sent, false).seller.name, "Иванов Иван Иванович");
        assert_eq!(f.doc(&draft, false).seller.name, "Петров Пётр Петрович");
    }

    #[test]
    fn signature_and_stamp_follow_the_setting_and_the_invoice() {
        let f = Fixture::new();
        let images = ProfileImages::new(f.db.conn(), &f.clock);
        let png = crate::files::thumbnails::tests::png(2, 2);
        images.set(ImageKind::Logo, png.clone()).unwrap();
        images.set(ImageKind::Signature, png.clone()).unwrap();
        let inv = f.invoice(StoredStatus::Draft, "ru", "RUB");
        let d = f.doc(&inv, false);
        assert!(d.logo && !d.signature && !d.stamp);
        assert!(f.doc(&inv, true).signature);
        // No stamp image: nothing to show even when enabled.
        assert!(!f.doc(&inv, true).stamp);
        f.db.conn()
            .execute("UPDATE invoices SET show_signature = 0", [])
            .unwrap();
        assert!(
            !f.doc(&inv, true).signature,
            "the invoice overrides the setting"
        );
        f.db.conn()
            .execute("UPDATE invoices SET show_signature = 1", [])
            .unwrap();
        assert!(f.doc(&inv, false).signature);
    }

    #[test]
    fn discounts_and_taxes_are_listed() {
        let f = Fixture::new();
        let inv = f.invoice(StoredStatus::Draft, "en", "EUR");
        f.db.conn()
            .execute("UPDATE invoices SET discount_kind = 'percent', discount_value = 1000, tax_bp = 2000", [])
            .unwrap();
        let d = f.doc(&inv, false);
        assert_eq!(d.subtotal, "1,851.84");
        assert_eq!(d.discount.as_deref(), Some("185.18"));
        assert_eq!(
            d.tax,
            Some(Line {
                label: "Tax 20%".into(),
                value: "333.33".into()
            })
        );
        assert_eq!(d.total, "1,999.99");
    }
}
