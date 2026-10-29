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

/// Renders an invoice from the document data (see `crm_core::finance::
/// document`) with the template it names. `images` are `logo`,
/// `signature` and `stamp` as PNG or JPEG bytes; each is shown only if the
/// data's flag of the same name is set.
pub fn render_invoice(
    data: &serde_json::Value,
    images: &[(&str, &[u8])],
) -> Result<Vec<u8>, PdfError> {
    let main = match data["template"].as_str() {
        Some("international") => include_str!("../templates/international.typ"),
        Some("ru_invoice") => include_str!("../templates/ru_invoice.typ"),
        other => return Err(PdfError::Compile(format!("unknown template {other:?}"))),
    };
    let mut data = data.clone();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for name in ["logo", "signature", "stamp"] {
        let image = images.iter().find(|(n, _)| *n == name);
        let file = match image {
            Some((_, bytes)) if data[name].as_bool() == Some(true) => {
                // Typst picks the decoder by the extension.
                let ext = if bytes.starts_with(&[0xff, 0xd8]) {
                    "jpg"
                } else {
                    "png"
                };
                let file = format!("{name}.{ext}");
                files.push((file.clone(), bytes.to_vec()));
                serde_json::Value::String(file)
            }
            _ => serde_json::Value::Null,
        };
        data[format!("{name}_file")] = file;
    }
    let engine = TypstEngine::builder()
        .main_file(main)
        .fonts(FONTS)
        .with_static_source_file_resolver([("common.typ", include_str!("../templates/common.typ"))])
        .with_static_file_resolver(
            files
                .iter()
                .map(|(name, bytes)| (name.as_str(), bytes.clone())),
        )
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
#[path = "invoice_tests.rs"]
mod invoice_tests;

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
