//! Spike probe (T-0.9, docs/adr/0001-pdf.md): render time of an invoice-sized document.
//! `cargo run --release -p crm-pdf --example probe`

use std::time::Instant;

const TEMPLATE: &str = r#"
#let data = json(bytes(sys.inputs.data))
#set page(paper: "a4", margin: 2cm)
#set text(lang: "ru", size: 10pt)
= Счёт на оплату № #data.number от #data.date
#table(
  columns: (auto, 1fr, auto, auto, auto),
  [№], [Наименование], [Кол-во], [Цена], [Сумма],
  ..data.items.enumerate().map(((i, it)) => (str(i + 1), it.name, it.qty, it.price, it.sum)).flatten()
)
*Итого: #data.total* \
Без НДС
"#;

fn main() {
    let items: Vec<_> = (1..=30)
        .map(|i| {
            serde_json::json!({
                "name": format!("Разработка модуля {i}"), "qty": "8 ч",
                "price": "2 500,00 ₽", "sum": "20 000,00 ₽"
            })
        })
        .collect();
    let data = serde_json::json!({
        "number": "INV-2026-001", "date": "28.10.2026",
        "items": items, "total": "600 000,00 ₽"
    });

    for run in 1..=3 {
        let start = Instant::now();
        let pdf = crm_pdf::render(TEMPLATE, &data).expect("render failed");
        println!("run {run}: {:?}, {} bytes", start.elapsed(), pdf.len());
    }
}
