//! Secrets (FR-VA-4): title, kind, links and dates are stored in the open;
//! all field values are one JSON payload encrypted with the DEK, bound to
//! the record by AAD (`secret:<id>:<kind>:<key version>`).

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, Row, named_params};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use super::Vault;
use super::crypto::{self, Sealed};
use super::templates::SecretKind;
use crate::db::id_list;
use crate::domain::date::is_iso_date;
use crate::domain::text::{clean, contains_pattern, fold};
use crate::domain::{Clock, Id, Timestamp};
use crate::{CoreError, CoreResult};

/// Bumped when the encryption scheme of payloads changes.
pub const KEY_VERSION: u32 = 1;

/// A free key/value pair (any kind may have them; `custom` only has them).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CustomEntry {
    pub key: String,
    pub value: String,
    pub secret: bool,
}

/// The encrypted part of a secret.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SecretPayload {
    /// Template field key → value.
    pub fields: BTreeMap<String, String>,
    pub custom: Vec<CustomEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SecretInput {
    pub title: String,
    pub kind: SecretKind,
    pub client_id: Option<Id>,
    pub project_id: Option<Id>,
    /// Renewal or expiry date `YYYY-MM-DD` (domains, certificates).
    pub expires_on: Option<String>,
    pub payload: SecretPayload,
}

/// One value of a secret: a template field or a free entry by position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FieldRef {
    Field { key: String },
    Custom { index: u32 },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SecretFilter {
    /// Case-insensitive substring of the title.
    pub query: Option<String>,
    pub kind: Option<SecretKind>,
    /// With its projects' secrets.
    pub client_id: Option<Id>,
    pub project_id: Option<Id>,
}

/// A secret without its values, safe to list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Secret {
    pub id: Id,
    pub title: String,
    pub kind: SecretKind,
    pub client_id: Option<Id>,
    pub client_name: Option<String>,
    pub project_id: Option<Id>,
    pub project_name: Option<String>,
    pub expires_on: Option<String>,
    pub tag_ids: Vec<Id>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl SecretPayload {
    /// Drops empty values and checks fields against the kind's template.
    pub fn validated(self, kind: SecretKind) -> CoreResult<Self> {
        let mut fields = BTreeMap::new();
        for (key, value) in self.fields {
            if kind.field(&key).is_none() {
                return Err(CoreError::validation(&format!("fields.{key}"), "unknown"));
            }
            // Values are kept as typed; only emptiness is judged by trimming.
            if !value.trim().is_empty() {
                fields.insert(key, value);
            }
        }
        if let Some(missing) = kind
            .fields()
            .iter()
            .find(|f| f.required && !fields.contains_key(f.key))
        {
            return Err(CoreError::validation(
                &format!("fields.{}", missing.key),
                "required",
            ));
        }
        let mut custom = Vec::new();
        for (index, entry) in self.custom.into_iter().enumerate() {
            let key = entry.key.trim().to_owned();
            if key.is_empty() && entry.value.trim().is_empty() {
                continue;
            }
            if key.is_empty() {
                return Err(CoreError::validation(
                    &format!("custom.{index}.key"),
                    "required",
                ));
            }
            custom.push(CustomEntry { key, ..entry });
        }
        if kind == SecretKind::Custom && custom.is_empty() {
            return Err(CoreError::validation("custom", "required"));
        }
        Ok(Self { fields, custom })
    }
}

const SELECT: &str = "SELECT s.id, s.title, s.kind, s.client_id, c.display_name AS client_name,
    s.project_id, p.name AS project_name, s.expires_on, s.created_at, s.updated_at,
    (SELECT json_group_array(g.tag_id ORDER BY t.name_key)
     FROM taggings g JOIN tags t ON t.id = g.tag_id
     WHERE g.owner_type = 'secret' AND g.owner_id = s.id) AS tag_ids
    FROM secrets s
    LEFT JOIN clients c ON c.id = s.client_id
    LEFT JOIN projects p ON p.id = s.project_id";

fn from_row(row: &Row<'_>) -> rusqlite::Result<Secret> {
    Ok(Secret {
        id: row.get("id")?,
        title: row.get("title")?,
        kind: row.get("kind")?,
        client_id: row.get("client_id")?,
        client_name: row.get("client_name")?,
        project_id: row.get("project_id")?,
        project_name: row.get("project_name")?,
        expires_on: row.get("expires_on")?,
        tag_ids: id_list(row, "tag_ids")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn not_found(id: Id) -> CoreError {
    CoreError::not_found("secret", id)
}

/// Binds a ciphertext to its record and kind.
fn aad(id: Id, kind: SecretKind) -> String {
    format!("secret:{id}:{}:{KEY_VERSION}", kind.as_str())
}

/// Validated open fields with the client taken from the project.
struct Resolved {
    title: String,
    client_id: Option<Id>,
    project_id: Option<Id>,
    expires_on: Option<String>,
    payload: SecretPayload,
}

pub struct SecretRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
    vault: &'a Vault,
}

impl<'a> SecretRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock, vault: &'a Vault) -> Self {
        Self { conn, clock, vault }
    }

    fn resolve(&self, input: SecretInput) -> CoreResult<Resolved> {
        let title = input.title.trim().to_owned();
        if title.is_empty() {
            return Err(CoreError::validation("title", "required"));
        }
        let expires_on = clean(input.expires_on);
        if expires_on.as_deref().is_some_and(|d| !is_iso_date(d)) {
            return Err(CoreError::validation("expiresOn", "date"));
        }
        let client_id = match input.project_id {
            Some(project_id) => {
                let owner: Option<Id> = self
                    .conn
                    .query_row(
                        "SELECT client_id FROM projects WHERE id = ?1",
                        [project_id],
                        |r| r.get(0),
                    )
                    .optional()?;
                let owner = owner.ok_or_else(|| CoreError::not_found("project", project_id))?;
                if input.client_id.is_some_and(|c| c != owner) {
                    return Err(CoreError::new("secret.client_mismatch"));
                }
                Some(owner)
            }
            None => {
                if let Some(id) = input.client_id {
                    crate::db::ensure_exists(self.conn, "clients", "client", id)?;
                }
                input.client_id
            }
        };
        Ok(Resolved {
            title,
            client_id,
            project_id: input.project_id,
            expires_on,
            payload: input.payload.validated(input.kind)?,
        })
    }

    fn encrypt(&self, id: Id, kind: SecretKind, payload: &SecretPayload) -> CoreResult<Sealed> {
        let json = Zeroizing::new(serde_json::to_vec(payload).expect("payload serializes"));
        self.vault
            .with_dek(|dek| Ok(crypto::seal(dek, &json, aad(id, kind).as_bytes())))
    }

    pub fn create(&self, input: SecretInput) -> CoreResult<Secret> {
        let kind = input.kind;
        let r = self.resolve(input)?;
        let id = Id::new();
        let sealed = self.encrypt(id, kind, &r.payload)?;
        self.conn.execute(
            "INSERT INTO secrets (id, title, title_key, kind, client_id, project_id, expires_on,
                 nonce, ciphertext, key_version, created_at, updated_at)
             VALUES (:id, :title, :title_key, :kind, :client, :project, :expires, :nonce,
                 :ciphertext, :version, :now, :now)",
            named_params! {
                ":id": id,
                ":title_key": fold(&r.title),
                ":title": r.title,
                ":kind": kind,
                ":client": r.client_id,
                ":project": r.project_id,
                ":expires": r.expires_on,
                ":nonce": sealed.nonce.as_slice(),
                ":ciphertext": sealed.ciphertext,
                ":version": KEY_VERSION,
                ":now": self.clock.now(),
            },
        )?;
        self.get(id)
    }

    pub fn update(&self, id: Id, input: SecretInput) -> CoreResult<Secret> {
        let kind = input.kind;
        let r = self.resolve(input)?;
        let sealed = self.encrypt(id, kind, &r.payload)?;
        let changed = self.conn.execute(
            "UPDATE secrets SET title = :title, title_key = :title_key, kind = :kind,
                 client_id = :client, project_id = :project, expires_on = :expires,
                 nonce = :nonce, ciphertext = :ciphertext, key_version = :version,
                 updated_at = :now
             WHERE id = :id",
            named_params! {
                ":id": id,
                ":title_key": fold(&r.title),
                ":title": r.title,
                ":kind": kind,
                ":client": r.client_id,
                ":project": r.project_id,
                ":expires": r.expires_on,
                ":nonce": sealed.nonce.as_slice(),
                ":ciphertext": sealed.ciphertext,
                ":version": KEY_VERSION,
                ":now": self.clock.now(),
            },
        )?;
        if changed == 0 {
            return Err(not_found(id));
        }
        self.get(id)
    }

    pub fn get(&self, id: Id) -> CoreResult<Secret> {
        self.conn
            .query_row(&format!("{SELECT} WHERE s.id = ?1"), [id], from_row)
            .optional()?
            .ok_or_else(|| not_found(id))
    }

    /// Decrypts the values; needs an unlocked vault.
    pub fn payload(&self, id: Id) -> CoreResult<SecretPayload> {
        let row: Option<(SecretKind, Vec<u8>, Vec<u8>)> = self
            .conn
            .query_row(
                "SELECT kind, nonce, ciphertext FROM secrets WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let (kind, nonce, ciphertext) = row.ok_or_else(|| not_found(id))?;
        let sealed = Sealed {
            nonce: nonce
                .try_into()
                .map_err(|_| CoreError::new("vault.corrupt"))?,
            ciphertext,
        };
        let json = self
            .vault
            .with_dek(|dek| crypto::open(dek, &sealed, aad(id, kind).as_bytes()))?;
        serde_json::from_slice(&json).map_err(|_| CoreError::new("vault.corrupt"))
    }

    /// Titles and links only; values are never listed.
    pub fn list(&self, filter: &SecretFilter) -> CoreResult<Vec<Secret>> {
        let query = filter
            .query
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
            .map(contains_pattern);
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT}
             WHERE (:query IS NULL OR s.title_key LIKE :query ESCAPE '\\')
               AND (:kind IS NULL OR s.kind = :kind)
               AND (:client IS NULL OR s.client_id = :client)
               AND (:project IS NULL OR s.project_id = :project)
             ORDER BY s.title_key, s.id"
        ))?;
        let rows = stmt.query_map(
            named_params! {
                ":query": query,
                ":kind": filter.kind,
                ":client": filter.client_id,
                ":project": filter.project_id,
            },
            from_row,
        )?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Decrypts a single value; `secret.no_field` if it is not set.
    pub fn reveal(&self, id: Id, field: &FieldRef) -> CoreResult<Zeroizing<String>> {
        let mut payload = self.payload(id)?;
        let value = match field {
            FieldRef::Field { key } => payload.fields.remove(key),
            FieldRef::Custom { index } => {
                let index = *index as usize;
                (index < payload.custom.len()).then(|| payload.custom.swap_remove(index).value)
            }
        };
        let result = value
            .map(Zeroizing::new)
            .ok_or_else(|| CoreError::new("secret.no_field"));
        // The rest of the decrypted payload is wiped before returning.
        for value in payload.fields.values_mut() {
            value.zeroize();
        }
        for entry in &mut payload.custom {
            entry.value.zeroize();
        }
        result
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        match self
            .conn
            .execute("DELETE FROM secrets WHERE id = ?1", [id])?
        {
            0 => Err(not_found(id)),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::vault::crypto::KdfParams;
    use rstest::rstest;
    use std::sync::Arc;

    const FAST: KdfParams = KdfParams {
        m_kib: 64,
        t: 1,
        p: 1,
    };
    const MARKER: &str = "PLAINTEXT_MARKER_42";

    struct Fixture {
        db: Db,
        clock: Arc<FakeClock>,
        vault: Vault,
        _dir: Option<tempfile::TempDir>,
    }

    impl Fixture {
        fn with_db(db: Db, dir: Option<tempfile::TempDir>) -> Self {
            let clock = Arc::new(FakeClock::at(Timestamp::from_millis(1_700_000_000_000)));
            let vault = Vault::new(FAST, clock.clone());
            vault.init(db.conn(), "master password").unwrap();
            Self {
                db,
                clock,
                vault,
                _dir: dir,
            }
        }

        fn new() -> Self {
            Self::with_db(Db::open_in_memory().unwrap(), None)
        }

        fn repo(&self) -> SecretRepo<'_> {
            SecretRepo::new(self.db.conn(), self.clock.as_ref(), &self.vault)
        }

        fn client(&self) -> Id {
            let input = ClientInput {
                kind: ClientKind::Company,
                last_name: None,
                first_name: None,
                middle_name: None,
                company_name: Some("Ромашка".into()),
                legal_name: None,
                status: ClientStatus::Active,
                default_currency: None,
                default_rate_minor: None,
            };
            ClientRepo::new(self.db.conn(), self.clock.as_ref())
                .create(input)
                .unwrap()
                .id
        }
    }

    fn payload(pairs: &[(&str, &str)]) -> SecretPayload {
        SecretPayload {
            fields: pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            custom: vec![],
        }
    }

    fn login(title: &str, password: &str) -> SecretInput {
        SecretInput {
            title: title.into(),
            kind: SecretKind::Login,
            client_id: None,
            project_id: None,
            expires_on: None,
            payload: payload(&[("username", "admin"), ("password", password)]),
        }
    }

    #[rstest]
    #[case(SecretKind::Login, "password")]
    #[case(SecretKind::SshKey, "private_key")]
    #[case(SecretKind::ApiToken, "token")]
    #[case(SecretKind::Database, "host")]
    #[case(SecretKind::Server, "host")]
    #[case(SecretKind::Ftp, "host")]
    #[case(SecretKind::Domain, "domain")]
    #[case(SecretKind::Env, "content")]
    #[case(SecretKind::BankCard, "number")]
    fn each_template_requires_its_fields(#[case] kind: SecretKind, #[case] field: &str) {
        let err = payload(&[(field, "   ")]).validated(kind).unwrap_err();
        assert_eq!(
            err,
            CoreError::validation(&format!("fields.{field}"), "required")
        );
        assert!(payload(&[(field, "x")]).validated(kind).is_ok());
    }

    #[test]
    fn custom_secrets_need_a_named_entry() {
        let entry = |key: &str, value: &str| CustomEntry {
            key: key.into(),
            value: value.into(),
            secret: true,
        };
        let empty = SecretPayload::default();
        assert_eq!(
            empty.validated(SecretKind::Custom).unwrap_err(),
            CoreError::validation("custom", "required")
        );
        let unnamed = SecretPayload {
            custom: vec![entry(" ", "v")],
            ..Default::default()
        };
        assert_eq!(
            unnamed.validated(SecretKind::Custom).unwrap_err(),
            CoreError::validation("custom.0.key", "required")
        );
        let ok = SecretPayload {
            custom: vec![entry(" PIN ", "1234")],
            ..Default::default()
        }
        .validated(SecretKind::Custom)
        .unwrap();
        assert_eq!(ok.custom[0].key, "PIN");
    }

    #[test]
    fn unknown_fields_are_rejected_and_empty_ones_dropped() {
        assert_eq!(
            payload(&[("password", "x"), ("pin", "1")])
                .validated(SecretKind::Login)
                .unwrap_err(),
            CoreError::validation("fields.pin", "unknown")
        );
        let kept = payload(&[("password", " p a s s "), ("url", "")])
            .validated(SecretKind::Login)
            .unwrap();
        // Values are kept as typed: spaces may be part of a password.
        assert_eq!(kept, payload(&[("password", " p a s s ")]));
    }

    #[test]
    fn create_list_fields_and_decrypt() {
        let f = Fixture::new();
        let client = f.client();
        let s = f
            .repo()
            .create(SecretInput {
                client_id: Some(client),
                expires_on: Some("2025-03-01".into()),
                ..login("  Хостинг  ", "hunter2")
            })
            .unwrap();
        assert_eq!(s.title, "Хостинг");
        assert_eq!(s.kind, SecretKind::Login);
        assert_eq!(s.client_name.as_deref(), Some("Ромашка"));
        assert_eq!(s.expires_on.as_deref(), Some("2025-03-01"));
        assert_eq!(f.repo().get(s.id).unwrap(), s);
        assert_eq!(
            f.repo().payload(s.id).unwrap(),
            payload(&[("password", "hunter2"), ("username", "admin")])
        );
    }

    #[test]
    fn title_and_dates_are_validated() {
        let f = Fixture::new();
        assert_eq!(
            f.repo().create(login(" ", "x")).unwrap_err(),
            CoreError::validation("title", "required")
        );
        assert_eq!(
            f.repo()
                .create(SecretInput {
                    expires_on: Some("soon".into()),
                    ..login("X", "x")
                })
                .unwrap_err(),
            CoreError::validation("expiresOn", "date")
        );
        let missing = Id::new();
        assert_eq!(
            f.repo()
                .create(SecretInput {
                    client_id: Some(missing),
                    ..login("X", "x")
                })
                .unwrap_err(),
            CoreError::not_found("client", missing)
        );
    }

    #[test]
    fn a_locked_vault_refuses_values() {
        let f = Fixture::new();
        let s = f.repo().create(login("X", "hunter2")).unwrap();
        f.vault.lock();
        let locked = CoreError::new("vault.locked");
        assert_eq!(f.repo().payload(s.id).unwrap_err(), locked);
        assert_eq!(f.repo().create(login("Y", "y")).unwrap_err(), locked);
        assert_eq!(f.repo().update(s.id, login("Y", "y")).unwrap_err(), locked);
        // Listing data stays available.
        assert_eq!(f.repo().get(s.id).unwrap().title, "X");
    }

    #[test]
    fn update_re_encrypts_and_changes_fields() {
        let f = Fixture::new();
        let s = f.repo().create(login("X", "old")).unwrap();
        f.clock.advance_millis(1_000);
        let updated = f.repo().update(s.id, login("Y", "new")).unwrap();
        assert_eq!(updated.title, "Y");
        assert_eq!(
            updated.updated_at,
            Timestamp::from_millis(1_700_000_001_000)
        );
        assert_eq!(f.repo().payload(s.id).unwrap().fields["password"], "new");
        assert_eq!(
            f.repo()
                .update(Id::new(), login("Z", "z"))
                .unwrap_err()
                .code,
            "secret.not_found"
        );
    }

    #[test]
    fn ciphertexts_cannot_be_swapped_between_secrets() {
        let f = Fixture::new();
        let a = f.repo().create(login("A", "aaa")).unwrap();
        let b = f.repo().create(login("B", "bbb")).unwrap();
        f.db.conn()
            .execute(
                "UPDATE secrets SET nonce = (SELECT nonce FROM secrets WHERE id = ?2),
                     ciphertext = (SELECT ciphertext FROM secrets WHERE id = ?2)
                 WHERE id = ?1",
                (a.id, b.id),
            )
            .unwrap();
        assert_eq!(
            f.repo().payload(a.id).unwrap_err(),
            CoreError::new("vault.decrypt")
        );
    }

    #[test]
    fn no_plaintext_reaches_the_database_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("crm.db");
        let f = Fixture::with_db(Db::open(&path).unwrap(), Some(dir));
        let mut input = login("Сервер", MARKER);
        input.payload.custom.push(CustomEntry {
            key: "note".into(),
            value: MARKER.into(),
            secret: false,
        });
        let s = f.repo().create(input).unwrap();
        f.repo().update(s.id, login("Сервер", MARKER)).unwrap();

        let base = path.to_str().unwrap().to_owned();
        for file in [base.clone(), format!("{base}-wal")] {
            let bytes = std::fs::read(&file).unwrap_or_default();
            let found = bytes.windows(MARKER.len()).any(|w| w == MARKER.as_bytes());
            assert!(!found, "{file} contains the plaintext marker");
        }
        assert_eq!(f.repo().payload(s.id).unwrap().fields["password"], MARKER);
    }

    #[test]
    fn lists_without_values_and_filters() {
        let f = Fixture::new();
        let client = f.client();
        f.repo()
            .create(SecretInput {
                client_id: Some(client),
                ..login("Хостинг Ромашки", "hunter2")
            })
            .unwrap();
        f.repo().create(login("Личная почта", "x")).unwrap();
        f.repo()
            .create(SecretInput {
                title: "ENV".into(),
                kind: SecretKind::Env,
                payload: payload(&[("content", "A=1")]),
                ..login("", "")
            })
            .unwrap();

        let titles = |filter: SecretFilter| -> Vec<String> {
            f.repo()
                .list(&filter)
                .unwrap()
                .into_iter()
                .map(|s| s.title)
                .collect()
        };
        assert_eq!(
            titles(SecretFilter::default()),
            ["ENV", "Личная почта", "Хостинг Ромашки"]
        );
        assert_eq!(
            titles(SecretFilter {
                client_id: Some(client),
                ..Default::default()
            }),
            ["Хостинг Ромашки"]
        );
        assert_eq!(
            titles(SecretFilter {
                query: Some("ПОЧТ".into()),
                ..Default::default()
            }),
            ["Личная почта"]
        );
        assert_eq!(
            titles(SecretFilter {
                kind: Some(SecretKind::Env),
                ..Default::default()
            }),
            ["ENV"]
        );

        // Listing works while locked and never carries values.
        f.vault.lock();
        let json =
            serde_json::to_string(&f.repo().list(&SecretFilter::default()).unwrap()).unwrap();
        assert!(!json.contains("hunter2"));
        assert!(!json.contains("payload") && !json.contains("ciphertext"));
    }

    #[test]
    fn reveals_one_value_at_a_time() {
        let f = Fixture::new();
        let mut input = login("X", "hunter2");
        input.payload.custom.push(CustomEntry {
            key: "PIN".into(),
            value: "1234".into(),
            secret: true,
        });
        let s = f.repo().create(input).unwrap();
        let reveal = |field: FieldRef| f.repo().reveal(s.id, &field);
        assert_eq!(
            *reveal(FieldRef::Field {
                key: "password".into()
            })
            .unwrap(),
            "hunter2"
        );
        assert_eq!(*reveal(FieldRef::Custom { index: 0 }).unwrap(), "1234");
        let no_field = CoreError::new("secret.no_field");
        assert_eq!(
            reveal(FieldRef::Field { key: "url".into() }).unwrap_err(),
            no_field
        );
        assert_eq!(reveal(FieldRef::Custom { index: 5 }).unwrap_err(), no_field);
        f.vault.lock();
        assert_eq!(
            reveal(FieldRef::Field {
                key: "password".into()
            })
            .unwrap_err(),
            CoreError::new("vault.locked")
        );
    }

    #[test]
    fn deleting_a_secret_or_its_client() {
        let f = Fixture::new();
        let s = f.repo().create(login("X", "x")).unwrap();
        f.repo().delete(s.id).unwrap();
        assert_eq!(f.repo().get(s.id).unwrap_err().code, "secret.not_found");
        assert_eq!(f.repo().delete(s.id).unwrap_err().code, "secret.not_found");

        let client = f.client();
        let owned = f
            .repo()
            .create(SecretInput {
                client_id: Some(client),
                ..login("Y", "y")
            })
            .unwrap();
        let clients = ClientRepo::new(f.db.conn(), f.clock.as_ref());
        assert_eq!(clients.delete_impact(client).unwrap().secrets, 1);
        clients.delete(client).unwrap();
        assert_eq!(f.repo().get(owned.id).unwrap_err().code, "secret.not_found");
    }
}
