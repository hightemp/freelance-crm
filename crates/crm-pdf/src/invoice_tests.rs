use serde_json::{Value, json};

use super::*;

/// A 1×1 PNG.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0,
    0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

fn text_of(pdf: &[u8]) -> String {
    pdf_extract::extract_text_from_mem(pdf)
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn russian() -> Value {
    json!({
        "template": "ru_invoice",
        "lang": "ru",
        "labels": {
            "seller": "Исполнитель", "buyer": "Заказчик", "date": "Дата", "due": "Оплатить до",
            "n": "№", "description": "Наименование", "quantity": "Кол-во", "unit": "Ед.",
            "price": "Цена", "amount": "Сумма", "subtotal": "Итого", "discount": "Скидка",
            "tax": "Налог", "total": "Всего к оплате", "in_words": "Сумма прописью",
            "notes": "Примечание", "signature": "Исполнитель"
        },
        "title": "Счёт на оплату № INV-2026-001 от 01.03.2026",
        "number": "INV-2026-001",
        "issue_date": "01.03.2026",
        "due_date": "15.03.2026",
        "seller": { "name": "Иванов Иван Иванович", "lines": [{ "label": "ИНН", "value": "500100732259" }] },
        "buyer": { "name": "ООО «Ромашка»", "lines": [{ "label": "ИНН", "value": "7707083893" }] },
        "items": [{ "n": 1, "description": "Разработка сайта #1 [этап]", "quantity": "1,5", "unit": "ч",
                    "price": "1\u{a0}234,56", "amount": "1\u{a0}851,84" }],
        "currency": "руб.",
        "subtotal": "1\u{a0}851,84",
        "discount": null,
        "tax": null,
        "total": "1\u{a0}851,84",
        "vat_note": "Без НДС. Исполнитель применяет налог на профессиональный доход.",
        "amount_in_words": "Одна тысяча восемьсот пятьдесят один рубль 84 копейки",
        "notes": "Спасибо!",
        "logo": false,
        "signature": false,
        "stamp": false
    })
}

fn english() -> Value {
    let mut d = russian();
    d["template"] = json!("international");
    d["lang"] = json!("en");
    d["labels"] = json!({
        "seller": "From", "buyer": "Bill to", "date": "Date", "due": "Due date", "n": "#",
        "description": "Description", "quantity": "Qty", "unit": "Unit", "price": "Price",
        "amount": "Amount", "subtotal": "Subtotal", "discount": "Discount", "tax": "Tax",
        "total": "Total due", "in_words": "Amount in words", "notes": "Notes",
        "signature": "Authorized signature"
    });
    d["title"] = json!("Invoice INV-2026-001");
    d["issue_date"] = json!("Mar 1, 2026");
    d["due_date"] = json!("Mar 15, 2026");
    d["seller"] =
        json!({ "name": "John Smith", "lines": [{ "label": "Tax ID", "value": "123-45" }] });
    d["buyer"] = json!({ "name": "Acme Inc.", "lines": [] });
    d["items"][0]["price"] = json!("1,234.56");
    d["items"][0]["amount"] = json!("1,851.84");
    d["currency"] = json!("USD");
    d["subtotal"] = json!("1,851.84");
    d["discount"] = json!("185.18");
    d["tax"] = json!({ "label": "Tax 20%", "value": "333.33" });
    d["total"] = json!("1,999.99");
    d["vat_note"] = Value::Null;
    d["amount_in_words"] = Value::Null;
    d
}

#[test]
fn russian_invoice_for_payment_has_the_legal_parts() {
    let text = text_of(&render_invoice(&russian(), &[]).unwrap());
    for part in [
        "Счёт на оплату № INV-2026-001 от 01.03.2026",
        "Иванов Иван Иванович",
        "ИНН: 500100732259",
        "ООО «Ромашка»",
        "Разработка сайта #1 [этап]",
        "Всего к оплате: 1 851,84 руб.",
        "Без НДС",
        "Сумма прописью: Одна тысяча восемьсот пятьдесят один рубль 84 копейки",
        "Оплатить до: 15.03.2026",
    ] {
        assert!(text.contains(part), "{part:?} missing in: {text}");
    }
}

#[test]
fn international_invoice_in_english() {
    let text = text_of(&render_invoice(&english(), &[]).unwrap());
    for part in [
        "Invoice INV-2026-001",
        "From",
        "Bill to",
        "Acme Inc.",
        "Discount: 185.18 USD",
        "Tax 20%: 333.33 USD",
        "Total due: 1,999.99 USD",
    ] {
        assert!(text.contains(part), "{part:?} missing in: {text}");
    }
    assert!(!text.contains("Сумма прописью"));
}

#[test]
fn text_layer_snapshot() {
    let text = text_of(&render_invoice(&english(), &[]).unwrap());
    // Recorded output; pdf-extract joins lines broken with `\` without a space.
    assert_eq!(
        text,
        "Invoice INV-2026-001 Date: Mar 1, 2026Due date: Mar 15, 2026 FromJohn SmithTax ID: \
         123-45 Bill toAcme Inc. # Description Qty Unit Price Amount 1 Разработка сайта #1 \
         [этап] 1,5 ч 1,234.56 1,851.84 Subtotal: 1,851.84 USD Discount: 185.18 USD Tax 20%: \
         333.33 USD Total due: 1,999.99 USD Notes: Спасибо! Authorized signature: John Smith"
    );
}

fn images_in(pdf: &[u8]) -> usize {
    pdf.windows(b"/Subtype /Image".len())
        .filter(|w| *w == b"/Subtype /Image")
        .count()
        + pdf
            .windows(b"/Subtype/Image".len())
            .filter(|w| *w == b"/Subtype/Image")
            .count()
}

#[test]
fn images_appear_only_when_asked_for() {
    // No logo, no signature: still a valid document.
    let plain = render_invoice(&russian(), &[]).unwrap();
    assert!(plain.starts_with(b"%PDF-"));
    assert_eq!(images_in(&plain), 0);

    let files = [("logo", PNG), ("signature", PNG), ("stamp", PNG)];
    // Images given but not enabled are not used.
    assert_eq!(images_in(&render_invoice(&russian(), &files).unwrap()), 0);
    // An RGBA PNG is an image plus its soft mask.
    let mut with = russian();
    with["signature"] = json!(true);
    let one = images_in(&render_invoice(&with, &files).unwrap());
    assert!(one > 0);
    with["logo"] = json!(true);
    assert!(images_in(&render_invoice(&with, &files).unwrap()) >= one);
}

#[test]
fn unknown_templates_are_an_error() {
    let mut d = russian();
    d["template"] = json!("de_rechnung");
    assert!(matches!(render_invoice(&d, &[]), Err(PdfError::Compile(_))));
}
