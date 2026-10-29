//! Ways to reach a client or a contact person: emails, phones, messengers,
//! websites. An owner may have several values of the same kind.

use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::db::text_enum;
use crate::domain::{Clock, EntityRef, Id};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ContactMethodKind {
    Email,
    Phone,
    Telegram,
    Whatsapp,
    Website,
    Other,
}

text_enum!(ContactMethodKind {
    Email => "email",
    Phone => "phone",
    Telegram => "telegram",
    Whatsapp => "whatsapp",
    Website => "website",
    Other => "other",
});

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContactMethodInput {
    pub kind: ContactMethodKind,
    pub value: String,
    /// Free-form label such as "work" or "personal".
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContactMethod {
    pub id: Id,
    pub kind: ContactMethodKind,
    pub value: String,
    pub label: Option<String>,
}

/// Validates a value for its kind and returns its canonical form.
/// The error is the validation rule that failed.
pub fn normalize_value(kind: ContactMethodKind, value: &str) -> Result<String, &'static str> {
    let value = value.trim();
    if value.is_empty() {
        return Err("required");
    }
    match kind {
        ContactMethodKind::Email => email(value).ok_or("email"),
        ContactMethodKind::Phone | ContactMethodKind::Whatsapp => phone(value).ok_or("phone"),
        ContactMethodKind::Telegram => telegram(value).ok_or("telegram"),
        ContactMethodKind::Website => website(value).ok_or("url"),
        ContactMethodKind::Other => Ok(value.to_owned()),
    }
}

/// Pragmatic check: one `@`, no spaces, a dotted domain. The domain is
/// lowercased; the local part is case-sensitive by the RFC.
fn email(value: &str) -> Option<String> {
    let (local, domain) = value.split_once('@')?;
    let labels_ok = domain.split('.').count() >= 2 && domain.split('.').all(|l| !l.is_empty());
    let valid = !local.is_empty()
        && labels_ok
        && !value.contains(char::is_whitespace)
        && !domain.contains('@');
    valid.then(|| format!("{local}@{}", domain.to_lowercase()))
}

/// Digits with common separators and an optional leading `+`, 5–15 digits
/// (E.164 maximum). Kept as typed, with runs of spaces collapsed.
fn phone(value: &str) -> Option<String> {
    let body = value.strip_prefix('+').unwrap_or(value);
    let allowed = body
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, ' ' | '-' | '(' | ')' | '.'));
    let digits = body.chars().filter(char::is_ascii_digit).count();
    (allowed && (5..=15).contains(&digits))
        .then(|| value.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// `@username`, `username` or a t.me link, stored as `@username`;
/// a phone number is accepted as well.
fn telegram(value: &str) -> Option<String> {
    if value.starts_with('+') || value.starts_with(|c: char| c.is_ascii_digit()) {
        return phone(value);
    }
    let rest = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"));
    let name = match rest.unwrap_or(value).strip_prefix("t.me/") {
        Some(name) => name,
        None if rest.is_some() => return None,
        None => value.strip_prefix('@').unwrap_or(value),
    };
    let valid = (5..=32).contains(&name.len())
        && name.starts_with(|c: char| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    valid.then(|| format!("@{name}"))
}

/// http(s) URL with a dotted host; `https://` is assumed when the scheme
/// is missing. International domains are stored in punycode.
fn website(value: &str) -> Option<String> {
    let with_scheme = if value.contains("://") {
        value.to_owned()
    } else {
        format!("https://{value}")
    };
    let url = Url::parse(&with_scheme).ok()?;
    let host_ok = url.host_str().is_some_and(|h| h.contains('.'));
    (matches!(url.scheme(), "http" | "https") && host_ok).then(|| url.into())
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<ContactMethod> {
    Ok(ContactMethod {
        id: row.get("id")?,
        kind: row.get("kind")?,
        value: row.get("value")?,
        label: row.get("label")?,
    })
}

pub struct ContactMethodRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> ContactMethodRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    /// Methods of the owner in the order they were saved.
    pub fn list(&self, owner: EntityRef) -> CoreResult<Vec<ContactMethod>> {
        let (owner_type, owner_id) = owner.parts();
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, value, label FROM contact_methods
             WHERE owner_type = ?1 AND owner_id = ?2
             ORDER BY position",
        )?;
        let rows = stmt.query_map((owner_type, owner_id), from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Replaces all methods of the owner atomically, as the edit form saves
    /// the whole list at once. Errors carry the `index` of the bad entry.
    pub fn replace(
        &self,
        owner: EntityRef,
        methods: Vec<ContactMethodInput>,
    ) -> CoreResult<Vec<ContactMethod>> {
        let methods = methods
            .into_iter()
            .enumerate()
            .map(|(index, m)| {
                let value = normalize_value(m.kind, &m.value)
                    .map_err(|rule| CoreError::validation("value", rule).with("index", index))?;
                let label = m
                    .label
                    .map(|l| l.trim().to_owned())
                    .filter(|l| !l.is_empty());
                Ok(ContactMethodInput { value, label, ..m })
            })
            .collect::<CoreResult<Vec<_>>>()?;
        owner.ensure_exists(self.conn)?;

        let (owner_type, owner_id) = owner.parts();
        let now = self.clock.now();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM contact_methods WHERE owner_type = ?1 AND owner_id = ?2",
            (owner_type, owner_id),
        )?;
        let mut insert = tx.prepare(
            "INSERT INTO contact_methods
                 (id, owner_type, owner_id, kind, value, label, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        )?;
        for (position, m) in methods.iter().enumerate() {
            insert.execute((
                Id::new(),
                owner_type,
                owner_id,
                m.kind,
                &m.value,
                &m.label,
                position as i64,
                now,
            ))?;
        }
        drop(insert);
        tx.commit()?;
        self.list(owner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::{FakeClock, Timestamp};
    use ContactMethodKind::*;
    use rstest::rstest;

    #[rstest]
    #[case(Email, "ivan@example.com", "ivan@example.com")]
    #[case(Email, "  Ivan.Petrov+crm@Example.COM ", "Ivan.Petrov+crm@example.com")]
    #[case(Email, "иван@почта.рф", "иван@почта.рф")]
    #[case(Phone, "+7 (912) 345-67-89", "+7 (912) 345-67-89")]
    #[case(Phone, "8 912 345 67 89", "8 912 345 67 89")]
    #[case(Phone, "+44  20   7946 0958", "+44 20 7946 0958")]
    #[case(Whatsapp, "+79123456789", "+79123456789")]
    #[case(Telegram, "@ivan_petrov", "@ivan_petrov")]
    #[case(Telegram, "ivan_petrov", "@ivan_petrov")]
    #[case(Telegram, "https://t.me/ivan_petrov", "@ivan_petrov")]
    #[case(Telegram, "t.me/ivan_petrov", "@ivan_petrov")]
    #[case(Telegram, "+7 912 345-67-89", "+7 912 345-67-89")]
    #[case(Website, "example.com", "https://example.com/")]
    #[case(Website, "http://example.com/path?q=1", "http://example.com/path?q=1")]
    #[case(Website, "https://пример.рф", "https://xn--e1afmkfd.xn--p1ai/")]
    #[case(Other, "  skype: ivan  ", "skype: ivan")]
    fn valid_values_are_normalized(
        #[case] kind: ContactMethodKind,
        #[case] input: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(normalize_value(kind, input), Ok(expected.to_owned()));
    }

    #[rstest]
    #[case(Email, "", "required")]
    #[case(Email, "ivan", "email")]
    #[case(Email, "ivan@", "email")]
    #[case(Email, "@example.com", "email")]
    #[case(Email, "ivan@example", "email")]
    #[case(Email, "iv an@example.com", "email")]
    #[case(Email, "ivan@@example.com", "email")]
    #[case(Phone, "12-34", "phone")]
    #[case(Phone, "+7 912 abc", "phone")]
    #[case(Phone, "7+912345678", "phone")]
    #[case(Phone, "1234567890123456", "phone")]
    #[case(Whatsapp, "@ivan", "phone")]
    #[case(Telegram, "@abc", "telegram")]
    #[case(Telegram, "@ivan-petrov", "telegram")]
    #[case(Telegram, "https://example.com/ivan", "telegram")]
    #[case(Website, "not a url", "url")]
    #[case(Website, "ftp://example.com", "url")]
    #[case(Website, "https://", "url")]
    #[case(Website, "localhost", "url")]
    #[case(Other, "   ", "required")]
    fn invalid_values_are_rejected(
        #[case] kind: ContactMethodKind,
        #[case] input: &str,
        #[case] rule: &str,
    ) {
        assert_eq!(normalize_value(kind, input), Err(rule));
    }

    struct Fixture {
        db: Db,
        clock: FakeClock,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                db: Db::open_in_memory().unwrap(),
                clock: FakeClock::at(Timestamp::from_millis(1_700_000_000_000)),
            }
        }

        fn client(&self) -> EntityRef {
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
            let client = ClientRepo::new(self.db.conn(), &self.clock)
                .create(input)
                .unwrap();
            EntityRef::Client(client.id)
        }

        fn repo(&self) -> ContactMethodRepo<'_> {
            ContactMethodRepo::new(self.db.conn(), &self.clock)
        }
    }

    fn method(kind: ContactMethodKind, value: &str, label: Option<&str>) -> ContactMethodInput {
        ContactMethodInput {
            kind,
            value: value.into(),
            label: label.map(Into::into),
        }
    }

    fn values(methods: &[ContactMethod]) -> Vec<(ContactMethodKind, &str, Option<&str>)> {
        methods
            .iter()
            .map(|m| (m.kind, m.value.as_str(), m.label.as_deref()))
            .collect()
    }

    #[test]
    fn owner_starts_without_methods() {
        let f = Fixture::new();
        assert!(f.repo().list(f.client()).unwrap().is_empty());
    }

    #[test]
    fn several_values_of_one_kind_keep_their_order() {
        let f = Fixture::new();
        let owner = f.client();
        let saved = f
            .repo()
            .replace(
                owner,
                vec![
                    method(Email, "work@example.com", Some(" work ")),
                    method(Phone, "+7 912 345-67-89", None),
                    method(Email, "home@example.com", Some("  ")),
                ],
            )
            .unwrap();
        let expected = [
            (Email, "work@example.com", Some("work")),
            (Phone, "+7 912 345-67-89", None),
            (Email, "home@example.com", None),
        ];
        assert_eq!(values(&saved), expected);
        assert_eq!(values(&f.repo().list(owner).unwrap()), expected);
    }

    #[test]
    fn replace_overwrites_previous_methods() {
        let f = Fixture::new();
        let owner = f.client();
        f.repo()
            .replace(owner, vec![method(Email, "old@example.com", None)])
            .unwrap();
        f.repo()
            .replace(owner, vec![method(Website, "example.com", None)])
            .unwrap();
        assert_eq!(
            values(&f.repo().list(owner).unwrap()),
            [(Website, "https://example.com/", None)]
        );
        f.repo().replace(owner, vec![]).unwrap();
        assert!(f.repo().list(owner).unwrap().is_empty());
    }

    #[test]
    fn invalid_entry_reports_its_index_and_changes_nothing() {
        let f = Fixture::new();
        let owner = f.client();
        f.repo()
            .replace(owner, vec![method(Email, "keep@example.com", None)])
            .unwrap();
        let err = f
            .repo()
            .replace(
                owner,
                vec![
                    method(Email, "new@example.com", None),
                    method(Email, "broken", None),
                ],
            )
            .unwrap_err();
        assert_eq!(
            err,
            CoreError::validation("value", "email").with("index", 1)
        );
        assert_eq!(
            values(&f.repo().list(owner).unwrap()),
            [(Email, "keep@example.com", None)]
        );
    }

    #[test]
    fn methods_of_unknown_client_are_rejected() {
        let f = Fixture::new();
        let id = Id::new();
        let err = f
            .repo()
            .replace(EntityRef::Client(id), vec![method(Other, "x", None)])
            .unwrap_err();
        assert_eq!(
            err,
            CoreError::new("client.not_found").with("id", id.to_string())
        );
    }

    #[test]
    fn owners_do_not_see_each_others_methods() {
        let f = Fixture::new();
        let (a, b) = (f.client(), f.client());
        f.repo().replace(a, vec![method(Other, "a", None)]).unwrap();
        assert!(f.repo().list(b).unwrap().is_empty());
    }

    #[test]
    fn deleting_client_deletes_its_methods() {
        let f = Fixture::new();
        let owner = f.client();
        let (_, id) = owner.parts();
        f.repo()
            .replace(owner, vec![method(Other, "x", None)])
            .unwrap();
        ClientRepo::new(f.db.conn(), &f.clock).delete(id).unwrap();
        let left: i64 =
            f.db.conn()
                .query_row("SELECT count(*) FROM contact_methods", [], |r| r.get(0))
                .unwrap();
        assert_eq!(left, 0);
    }
}
