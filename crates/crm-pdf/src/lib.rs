//! Renders Typst templates to PDF. Templates receive their data as JSON via
//! `sys.inputs.data` and decode it with `json(bytes(sys.inputs.data))`.

use typst::foundations::{Dict, IntoValue};
use typst_as_lib::TypstEngine;
use typst_layout::PagedDocument;

/// Noto Sans (SIL Open Font License 1.1): Latin, Cyrillic, Greek.
static FONTS: [&[u8]; 2] = [
    include_bytes!("../fonts/NotoSans-Regular.ttf"),
    include_bytes!("../fonts/NotoSans-Bold.ttf"),
];

#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("template compilation failed: {0}")]
    Compile(String),
    #[error("pdf export failed: {0}")]
    Export(String),
}

pub fn render(template: &str, data: &serde_json::Value) -> Result<Vec<u8>, PdfError> {
    let engine = TypstEngine::builder()
        .main_file(template)
        // Only fonts bundled into the binary: output must not depend on the OS.
        .fonts(FONTS)
        .build();

    let mut inputs = Dict::new();
    inputs.insert("data".into(), data.to_string().into_value());

    let doc: PagedDocument = engine
        .compile_with_input(inputs)
        .output
        .map_err(|e| PdfError::Compile(e.to_string()))?;

    typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default())
        .map_err(|diags| PdfError::Export(format!("{diags:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TEMPLATE: &str = r#"
#let data = json(bytes(sys.inputs.data))
#set page(paper: "a4")
#set text(lang: data.lang)
= #data.title
Total: #data.total
"#;

    fn text_of(pdf: &[u8]) -> String {
        pdf_extract::extract_text_from_mem(pdf)
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn renders_a_pdf_document() {
        let pdf = render(
            TEMPLATE,
            &json!({ "lang": "en", "title": "Invoice", "total": "10" }),
        )
        .unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
    }

    #[test]
    fn passes_data_and_renders_cyrillic() {
        let data = json!({ "lang": "ru", "title": "Счёт на оплату № 7", "total": "1 234,56 ₽" });
        let text = text_of(&render(TEMPLATE, &data).unwrap());
        assert!(text.contains("Счёт на оплату № 7"), "extracted: {text}");
        assert!(text.contains("1 234,56"), "extracted: {text}");
    }

    #[test]
    fn template_errors_are_reported() {
        let err = render("#let x = ", &json!({})).unwrap_err();
        assert!(matches!(err, PdfError::Compile(_)), "{err:?}");
    }
}
