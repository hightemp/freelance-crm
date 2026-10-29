//! Requisite schemas by country (FR-CO-1): legal forms, fields, groups,
//! labels, tax presets and the invoice template are data in
//! `countries/*.toml`, embedded into the binary. Adding a country needs no
//! code unless it brings new validators.

pub mod validators;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

pub use validators::Values;

/// Languages every label must be translated into.
pub const LOCALES: &[&str] = &["en", "ru"];

const SOURCES: &[(&str, &str)] = &[
    ("ru.toml", include_str!("../../countries/ru.toml")),
    ("intl.toml", include_str!("../../countries/intl.toml")),
];

/// Locale → text.
pub type Labels = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct CountrySchema {
    /// ISO 3166-1 alpha-2, or `INTL` for the international schema.
    pub code: String,
    pub labels: Labels,
    pub invoice_template: String,
    pub groups: Vec<FieldGroup>,
    pub fields: Vec<FieldDef>,
    pub legal_forms: Vec<LegalForm>,
    #[serde(default)]
    pub tax_presets: Vec<TaxPreset>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct FieldGroup {
    pub key: String,
    pub labels: Labels,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    #[default]
    Text,
    Textarea,
}

/// How input is normalized (and masked in the UI).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum InputKind {
    #[default]
    Text,
    /// Only digits are kept (spaces and dashes are dropped).
    Digits,
    /// Upper-cased, without spaces (IBAN, SWIFT, KPP).
    Upper,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct FieldDef {
    pub key: String,
    pub group: String,
    #[serde(default)]
    pub kind: FieldKind,
    #[serde(default)]
    pub input: InputKind,
    pub max_length: Option<u32>,
    /// Name from [`validators::NAMES`].
    pub validator: Option<String>,
    pub labels: Labels,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct LegalForm {
    pub key: String,
    pub labels: Labels,
    /// Fields shown for this form, in order.
    pub fields: Vec<FormField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct FormField {
    pub key: String,
    #[serde(default)]
    pub required: bool,
    /// Replaces the field's validator for this form (e.g. 10-digit INN).
    pub validator: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct TaxPreset {
    pub key: String,
    pub labels: Labels,
    pub legal_forms: Vec<String>,
}

/// A requisite field that failed validation; `rule` is `required` or a
/// validator name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct FieldError {
    pub field: String,
    pub rule: String,
}

impl CountrySchema {
    pub fn parse(source: &str) -> Result<Self, String> {
        toml::from_str(source).map_err(|e| e.to_string())
    }

    /// Structural problems of the schema; empty when it is valid.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut labels = |owner: String, labels: &Labels| {
            for locale in LOCALES {
                if labels.get(*locale).is_none_or(|l| l.trim().is_empty()) {
                    problems.push(format!("{owner}: missing {locale} label"));
                }
            }
        };
        labels(self.code.clone(), &self.labels);
        for g in &self.groups {
            labels(format!("group {}", g.key), &g.labels);
        }
        for f in &self.fields {
            labels(format!("field {}", f.key), &f.labels);
        }
        for form in &self.legal_forms {
            labels(format!("legal form {}", form.key), &form.labels);
        }
        for preset in &self.tax_presets {
            labels(format!("tax preset {}", preset.key), &preset.labels);
        }

        let unique = |problems: &mut Vec<String>, what: &str, keys: Vec<&str>| {
            let mut seen = BTreeSet::new();
            for key in keys {
                if !seen.insert(key) {
                    problems.push(format!("duplicate {what} {key}"));
                }
            }
        };
        unique(
            &mut problems,
            "group",
            self.groups.iter().map(|g| g.key.as_str()).collect(),
        );
        unique(
            &mut problems,
            "field",
            self.fields.iter().map(|f| f.key.as_str()).collect(),
        );
        unique(
            &mut problems,
            "legal form",
            self.legal_forms.iter().map(|f| f.key.as_str()).collect(),
        );

        let known_validator = |name: &Option<String>| {
            name.as_deref()
                .is_none_or(|n| validators::NAMES.contains(&n))
        };
        for f in &self.fields {
            if !self.groups.iter().any(|g| g.key == f.group) {
                problems.push(format!("field {}: unknown group {}", f.key, f.group));
            }
            if !known_validator(&f.validator) {
                let name = f.validator.as_deref().unwrap_or_default();
                problems.push(format!("field {}: unknown validator {name}", f.key));
            }
        }
        if self.legal_forms.is_empty() {
            problems.push(format!("{}: no legal forms", self.code));
        }
        for form in &self.legal_forms {
            for ff in &form.fields {
                if self.field(&ff.key).is_none() {
                    problems.push(format!("legal form {}: unknown field {}", form.key, ff.key));
                }
                if !known_validator(&ff.validator) {
                    let name = ff.validator.as_deref().unwrap_or_default();
                    problems.push(format!("legal form {}: unknown validator {name}", form.key));
                }
            }
        }
        for preset in &self.tax_presets {
            for form in &preset.legal_forms {
                if self.legal_form(form).is_none() {
                    problems.push(format!(
                        "tax preset {}: unknown legal form {form}",
                        preset.key
                    ));
                }
            }
        }
        if self.invoice_template.trim().is_empty() {
            problems.push(format!("{}: no invoice template", self.code));
        }
        problems
    }

    pub fn legal_form(&self, key: &str) -> Option<&LegalForm> {
        self.legal_forms.iter().find(|f| f.key == key)
    }

    pub fn field(&self, key: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.key == key)
    }

    /// Normalizes and validates requisites for a legal form. Only the
    /// form's fields are kept; blank optional fields are dropped.
    pub fn validate(&self, legal_form: &str, values: &Values) -> Result<Values, Vec<FieldError>> {
        let error = |field: &str, rule: &str| FieldError {
            field: field.to_owned(),
            rule: rule.to_owned(),
        };
        let Some(form) = self.legal_form(legal_form) else {
            return Err(vec![error("legalForm", "unknown")]);
        };

        let mut normalized = Values::new();
        for ff in &form.fields {
            let Some(def) = self.field(&ff.key) else {
                continue;
            };
            let raw = values.get(&ff.key).map(String::as_str).unwrap_or_default();
            let value = normalize(def.input, raw);
            if !value.is_empty() {
                normalized.insert(ff.key.clone(), value);
            }
        }

        // Validators run on normalized values so cross-field checks (account
        // against BIK) see the same data that is saved.
        let mut errors = Vec::new();
        for ff in &form.fields {
            let Some(def) = self.field(&ff.key) else {
                continue;
            };
            let Some(value) = normalized.get(&ff.key) else {
                if ff.required {
                    errors.push(error(&ff.key, "required"));
                }
                continue;
            };
            if def
                .max_length
                .is_some_and(|max| value.chars().count() > max as usize)
            {
                errors.push(error(&ff.key, "max_length"));
                continue;
            }
            if let Some(name) = ff.validator.as_ref().or(def.validator.as_ref())
                && let Err(rule) = validators::check(name, value, &normalized)
            {
                errors.push(error(&ff.key, rule));
            }
        }
        if errors.is_empty() {
            Ok(normalized)
        } else {
            Err(errors)
        }
    }
}

fn normalize(input: InputKind, raw: &str) -> String {
    match input {
        InputKind::Text => raw.trim().to_owned(),
        InputKind::Digits => raw
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .collect(),
        InputKind::Upper => raw
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_uppercase(),
    }
}

/// Built-in schemas, parsed once. Panics on an invalid built-in schema,
/// which `builtin_schemas_are_valid` rules out.
pub fn builtin() -> &'static [CountrySchema] {
    static SCHEMAS: OnceLock<Vec<CountrySchema>> = OnceLock::new();
    SCHEMAS.get_or_init(|| {
        SOURCES
            .iter()
            .map(|(file, source)| {
                CountrySchema::parse(source)
                    .unwrap_or_else(|e| panic!("built-in schema {file}: {e}"))
            })
            .collect()
    })
}

pub fn get(code: &str) -> Option<&'static CountrySchema> {
    builtin().iter().find(|s| s.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Locale;

    fn values(pairs: &[(&str, &str)]) -> Values {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    fn errors(result: Result<Values, Vec<FieldError>>) -> Vec<(String, String)> {
        result
            .unwrap_err()
            .into_iter()
            .map(|e| (e.field, e.rule))
            .collect()
    }

    fn ru() -> &'static CountrySchema {
        get("RU").unwrap()
    }

    #[test]
    fn locales_match_the_app_languages() {
        for locale in [Locale::En, Locale::Ru] {
            let code = serde_json::to_value(locale).unwrap();
            assert!(LOCALES.contains(&code.as_str().unwrap()));
        }
    }

    #[test]
    fn builtin_schemas_are_valid() {
        for (file, source) in SOURCES {
            let schema = CountrySchema::parse(source).unwrap_or_else(|e| panic!("{file}: {e}"));
            assert_eq!(schema.problems(), Vec::<String>::new(), "{file}");
        }
        let codes: Vec<_> = builtin().iter().map(|s| s.code.as_str()).collect();
        assert_eq!(codes, ["RU", "INTL"]);
        assert!(get("XX").is_none());
    }

    const MINIMAL: &str = r#"
code = "XX"
invoice_template = "international"
labels = { en = "X", ru = "Х" }
groups = [{ key = "main", labels = { en = "Main", ru = "Основные" } }]
fields = [{ key = "id", group = "main", labels = { en = "ID", ru = "ID" } }]
legal_forms = [{ key = "company", labels = { en = "Co", ru = "Ко" }, fields = [{ key = "id" }] }]
"#;

    #[test]
    fn minimal_schema_parses_with_defaults() {
        let schema = CountrySchema::parse(MINIMAL).unwrap();
        assert!(schema.problems().is_empty());
        let field = schema.field("id").unwrap();
        assert_eq!(
            (field.kind, field.input),
            (FieldKind::Text, InputKind::Text)
        );
        assert!(schema.tax_presets.is_empty());
    }

    #[test]
    fn syntax_errors_are_reported() {
        assert!(CountrySchema::parse("code = ").is_err());
        assert!(CountrySchema::parse("code = \"XX\"").is_err());
    }

    #[test]
    fn problems_are_detected() {
        let broken = MINIMAL
            .replace(
                r#"labels = { en = "X", ru = "Х" }"#,
                r#"labels = { en = "X" }"#,
            )
            .replace(r#"group = "main""#, r#"group = "bank", validator = "nope""#)
            .replace(
                r#"fields = [{ key = "id" }]"#,
                r#"fields = [{ key = "id" }, { key = "ghost" }]"#,
            )
            + r#"
[[tax_presets]]
key = "vat"
legal_forms = ["llc"]
labels = { en = "VAT", ru = "НДС" }
"#;
        let problems = CountrySchema::parse(&broken).unwrap().problems();
        assert_eq!(
            problems,
            [
                "XX: missing ru label",
                "field id: unknown group bank",
                "field id: unknown validator nope",
                "legal form company: unknown field ghost",
                "tax preset vat: unknown legal form llc",
            ]
        );
    }

    #[test]
    fn duplicates_and_empty_schemas_are_detected() {
        let dup = MINIMAL.replace(
            r#"fields = [{ key = "id", group = "main", labels = { en = "ID", ru = "ID" } }]"#,
            r#"fields = [
  { key = "id", group = "main", labels = { en = "ID", ru = "ID" } },
  { key = "id", group = "main", labels = { en = "ID", ru = "ID" } },
]"#,
        );
        assert_eq!(
            CountrySchema::parse(&dup).unwrap().problems(),
            ["duplicate field id"]
        );
        let no_forms = MINIMAL.replace(
            r#"legal_forms = [{ key = "company", labels = { en = "Co", ru = "Ко" }, fields = [{ key = "id" }] }]"#,
            "legal_forms = []",
        );
        assert_eq!(
            CountrySchema::parse(&no_forms).unwrap().problems(),
            ["XX: no legal forms"]
        );
    }

    #[test]
    fn required_fields_depend_on_the_legal_form() {
        let required = |form: &str| -> Vec<&str> {
            ru().legal_form(form)
                .unwrap()
                .fields
                .iter()
                .filter(|f| f.required)
                .map(|f| f.key.as_str())
                .collect()
        };
        assert_eq!(required("self_employed"), ["inn"]);
        assert_eq!(required("ip"), ["inn", "ogrnip", "address"]);
        assert_eq!(required("ooo"), ["inn", "kpp", "ogrn", "address"]);
        assert_eq!(required("person"), Vec::<&str>::new());

        let has = |form: &str, key: &str| {
            ru().legal_form(form)
                .unwrap()
                .fields
                .iter()
                .any(|f| f.key == key)
        };
        assert!(!has("self_employed", "kpp") && !has("self_employed", "ogrn"));
        assert!(has("ooo", "kpp") && has("ooo", "ogrn"));
    }

    #[test]
    fn validate_reports_missing_required_fields() {
        assert_eq!(
            errors(ru().validate("ooo", &values(&[("inn", "7707083893")]))),
            [
                ("kpp".into(), "required".into()),
                ("ogrn".into(), "required".into()),
                ("address".into(), "required".into()),
            ]
        );
        assert!(
            ru().validate("self_employed", &values(&[("inn", "500100732259")]))
                .is_ok()
        );
    }

    #[test]
    fn validate_normalizes_and_drops_foreign_fields() {
        let input = values(&[
            ("inn", " 7707 083-893 "),
            ("kpp", "773601001"),
            ("ogrn", "1027700132195"),
            ("address", "  Москва, ул. Вавилова, 19  "),
            ("bik", "044 525 225"),
            ("account", "4070 2810 9380 0000 0001"),
            ("bank_name", "   "),
            ("ogrnip", "304500116000157"),
            ("iban", "DE89370400440532013000"),
        ]);
        let saved = ru().validate("ooo", &input).unwrap();
        assert_eq!(
            saved,
            values(&[
                ("inn", "7707083893"),
                ("kpp", "773601001"),
                ("ogrn", "1027700132195"),
                ("address", "Москва, ул. Вавилова, 19"),
                ("bik", "044525225"),
                ("account", "40702810938000000001"),
            ])
        );
    }

    #[test]
    fn validate_uses_the_form_specific_validator() {
        let ooo = values(&[
            ("inn", "500100732259"),
            ("kpp", "773601001"),
            ("ogrn", "1027700132195"),
            ("address", "Москва"),
        ]);
        assert_eq!(
            errors(ru().validate("ooo", &ooo)),
            [("inn".into(), "ru_inn_10".into())]
        );
        assert_eq!(
            errors(ru().validate(
                "ip",
                &values(&[("inn", "7707083893"), ("ogrnip", "1"), ("address", "x")])
            )),
            [
                ("inn".into(), "ru_inn_12".into()),
                ("ogrnip".into(), "ru_ogrnip".into()),
            ]
        );
    }

    #[test]
    fn validate_checks_the_account_against_the_bik() {
        let form = |bik: &str| {
            values(&[
                ("inn", "500100732259"),
                ("bik", bik),
                ("account", "40702810938000000001"),
            ])
        };
        assert!(ru().validate("self_employed", &form("044525225")).is_ok());
        assert_eq!(
            errors(ru().validate("self_employed", &form("044525226"))),
            [("account".into(), "ru_account".into())]
        );
    }

    #[test]
    fn validate_upper_cases_international_codes() {
        let intl = get("INTL").unwrap();
        let saved = intl
            .validate(
                "company",
                &values(&[
                    ("address", "1 Main St"),
                    ("iban", "de89 3704 0044 0532 0130 00"),
                    ("swift", "deutdeff"),
                ]),
            )
            .unwrap();
        assert_eq!(saved["iban"], "DE89370400440532013000");
        assert_eq!(saved["swift"], "DEUTDEFF");
        assert_eq!(
            errors(intl.validate(
                "company",
                &values(&[("address", "x"), ("iban", "DE00 0000")])
            )),
            [("iban".into(), "iban".into())]
        );
    }

    #[test]
    fn unknown_legal_form_is_an_error() {
        assert_eq!(
            errors(ru().validate("gmbh", &Values::new())),
            [("legalForm".into(), "unknown".into())]
        );
    }

    #[test]
    fn serializes_camel_case_for_the_frontend() {
        let json = serde_json::to_value(ru()).unwrap();
        assert_eq!(json["invoiceTemplate"], "ru_invoice");
        assert_eq!(json["legalForms"][0]["key"], "self_employed");
        assert_eq!(json["fields"][0]["maxLength"], 12);
        assert_eq!(json["taxPresets"][0]["legalForms"][0], "self_employed");
    }
}
