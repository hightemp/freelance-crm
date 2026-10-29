//! Secret kinds and their field templates (FR-VA-5). Labels live in the UI
//! message catalogs as `secret_field_<key>`.

use serde::{Deserialize, Serialize};

use crate::db::text_enum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SecretKind {
    Login,
    SshKey,
    ApiToken,
    Database,
    Server,
    Ftp,
    Domain,
    Env,
    BankCard,
    /// Free key/value pairs only.
    Custom,
}

text_enum!(SecretKind {
    Login => "login",
    SshKey => "ssh_key",
    ApiToken => "api_token",
    Database => "database",
    Server => "server",
    Ftp => "ftp",
    Domain => "domain",
    Env => "env",
    BankCard => "bank_card",
    Custom => "custom",
});

pub const ALL_KINDS: &[SecretKind] = &[
    SecretKind::Login,
    SecretKind::SshKey,
    SecretKind::ApiToken,
    SecretKind::Database,
    SecretKind::Server,
    SecretKind::Ftp,
    SecretKind::Domain,
    SecretKind::Env,
    SecretKind::BankCard,
    SecretKind::Custom,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TemplateField {
    pub key: &'static str,
    pub required: bool,
    /// Hidden by default in the UI; revealed or copied on demand.
    pub secret: bool,
    pub multiline: bool,
}

const fn plain(key: &'static str) -> TemplateField {
    TemplateField {
        key,
        required: false,
        secret: false,
        multiline: false,
    }
}

const fn hidden(key: &'static str) -> TemplateField {
    TemplateField {
        secret: true,
        ..plain(key)
    }
}

const fn required(field: TemplateField) -> TemplateField {
    TemplateField {
        required: true,
        ..field
    }
}

const fn multiline(field: TemplateField) -> TemplateField {
    TemplateField {
        multiline: true,
        ..field
    }
}

impl SecretKind {
    /// Fields of the template in display order.
    pub fn fields(self) -> &'static [TemplateField] {
        match self {
            Self::Login => {
                const F: &[TemplateField] = &[
                    plain("url"),
                    plain("username"),
                    required(hidden("password")),
                ];
                F
            }
            Self::SshKey => {
                const F: &[TemplateField] = &[
                    required(multiline(hidden("private_key"))),
                    multiline(plain("public_key")),
                    hidden("passphrase"),
                    plain("host"),
                    plain("user"),
                    plain("port"),
                ];
                F
            }
            Self::ApiToken => {
                const F: &[TemplateField] =
                    &[plain("service"), required(hidden("token")), plain("scope")];
                F
            }
            Self::Database => {
                const F: &[TemplateField] = &[
                    plain("db_type"),
                    required(plain("host")),
                    plain("port"),
                    plain("database"),
                    plain("user"),
                    hidden("password"),
                    hidden("connection_string"),
                ];
                F
            }
            Self::Server => {
                const F: &[TemplateField] = &[
                    required(plain("host")),
                    plain("ip"),
                    plain("user"),
                    hidden("password"),
                    plain("panel_url"),
                ];
                F
            }
            Self::Ftp => {
                const F: &[TemplateField] = &[
                    required(plain("host")),
                    plain("port"),
                    plain("user"),
                    hidden("password"),
                ];
                F
            }
            Self::Domain => {
                const F: &[TemplateField] = &[
                    required(plain("domain")),
                    plain("registrar"),
                    plain("login"),
                    hidden("password"),
                ];
                F
            }
            Self::Env => {
                const F: &[TemplateField] = &[required(multiline(hidden("content")))];
                F
            }
            Self::BankCard => {
                const F: &[TemplateField] = &[
                    required(hidden("number")),
                    plain("holder"),
                    plain("expiry"),
                    hidden("cvv"),
                    hidden("pin"),
                ];
                F
            }
            Self::Custom => {
                const F: &[TemplateField] = &[];
                F
            }
        }
    }

    pub fn field(self, key: &str) -> Option<&'static TemplateField> {
        self.fields().iter().find(|f| f.key == key)
    }
}

/// A kind with its fields, for the UI to build forms.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SecretTemplate {
    pub kind: SecretKind,
    pub fields: Vec<TemplateField>,
}

pub fn templates() -> Vec<SecretTemplate> {
    ALL_KINDS
        .iter()
        .map(|&kind| SecretTemplate {
            kind,
            fields: kind.fields().to_vec(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_but_custom_has_a_required_field() {
        for &kind in ALL_KINDS {
            let has_required = kind.fields().iter().any(|f| f.required);
            assert_eq!(has_required, kind != SecretKind::Custom, "{kind:?}");
        }
    }

    #[test]
    fn field_keys_are_unique_snake_case() {
        for &kind in ALL_KINDS {
            let mut keys: Vec<_> = kind.fields().iter().map(|f| f.key).collect();
            assert!(
                keys.iter()
                    .all(|k| k.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
            );
            keys.sort();
            keys.dedup();
            assert_eq!(keys.len(), kind.fields().len(), "{kind:?}");
        }
    }

    #[test]
    fn templates_list_every_kind() {
        let kinds: Vec<_> = templates().iter().map(|t| t.kind).collect();
        assert_eq!(kinds, ALL_KINDS);
        assert!(SecretKind::SshKey.field("private_key").unwrap().multiline);
        assert!(SecretKind::Login.field("password").unwrap().secret);
        assert!(!SecretKind::Login.field("username").unwrap().secret);
    }

    #[test]
    fn kinds_and_fields_have_messages_in_every_locale() {
        for locale in crate::countries::LOCALES {
            let path = format!(
                "{}/../../messages/{locale}.json",
                env!("CARGO_MANIFEST_DIR")
            );
            let catalog: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            for &kind in ALL_KINDS {
                let key = format!("secret_kind_{}", kind.as_str());
                assert!(catalog.get(&key).is_some(), "{locale}: missing {key}");
                for field in kind.fields() {
                    let key = format!("secret_field_{}", field.key);
                    assert!(catalog.get(&key).is_some(), "{locale}: missing {key}");
                }
            }
        }
    }
}
