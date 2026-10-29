//! Requisites of the freelancer's own profile and of clients, stored per
//! owner and validated against the country schema (FR-CO, FR-ST-4).

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::countries::{self, FieldError, Values};
use crate::db::ensure_exists;
use crate::domain::text::clean;
use crate::domain::{Clock, Id, Timestamp};
use crate::{CoreError, CoreResult};

/// Whose requisites these are. There is a single profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "id", rename_all = "lowercase")]
pub enum RequisitesOwner {
    Profile,
    Client(Id),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RequisitesInput {
    /// Schema code: `RU`, `INTL`, …
    pub country: String,
    pub legal_form: String,
    /// Name on documents; used by the profile (clients have their own names).
    pub name: Option<String>,
    pub values: Values,
    /// Must be set to change the country of saved requisites.
    pub confirm_country_change: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Requisites {
    pub country: String,
    pub legal_form: String,
    pub name: Option<String>,
    pub values: Values,
    pub updated_at: Timestamp,
}

/// Field errors are an expected outcome of saving a form, not a failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum SaveRequisites {
    Saved { requisites: Requisites },
    Invalid { errors: Vec<FieldError> },
}

impl RequisitesOwner {
    /// `owner_type` and `owner_id` columns; the profile is a singleton.
    fn parts(self) -> (&'static str, String) {
        match self {
            Self::Profile => ("profile", "self".to_owned()),
            Self::Client(id) => ("client", id.to_string()),
        }
    }
}

pub struct RequisitesService<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> RequisitesService<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    pub fn get(&self, owner: RequisitesOwner) -> CoreResult<Option<Requisites>> {
        let (owner_type, owner_id) = owner.parts();
        let row = self
            .conn
            .query_row(
                "SELECT country, legal_form, name, fields, updated_at FROM requisites
                 WHERE owner_type = ?1 AND owner_id = ?2",
                (owner_type, owner_id),
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, Timestamp>(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((country, legal_form, name, fields, updated_at)) = row else {
            return Ok(None);
        };
        // Stored by `save` from a map, so this only fails on manual edits.
        let values = serde_json::from_str(&fields).map_err(|_| CoreError::new("internal.db"))?;
        Ok(Some(Requisites {
            country,
            legal_form,
            name,
            values,
            updated_at,
        }))
    }

    pub fn save(
        &self,
        owner: RequisitesOwner,
        input: RequisitesInput,
    ) -> CoreResult<SaveRequisites> {
        let schema = countries::get(&input.country).ok_or_else(|| {
            CoreError::new("requisites.unknown_country").with("country", input.country.as_str())
        })?;
        if let RequisitesOwner::Client(id) = owner {
            ensure_exists(self.conn, "clients", "client", id)?;
        }
        if let Some(current) = self.get(owner)?
            && current.country != input.country
            && !input.confirm_country_change
        {
            return Err(CoreError::new("requisites.country_change_unconfirmed")
                .with("from", current.country)
                .with("to", input.country));
        }
        // Validation keeps only the legal form's fields, which also clears
        // fields of a previous country or form.
        let values = match schema.validate(&input.legal_form, &input.values) {
            Ok(values) => values,
            Err(errors) => return Ok(SaveRequisites::Invalid { errors }),
        };
        let (owner_type, owner_id) = owner.parts();
        let fields = serde_json::to_string(&values).expect("string map serializes");
        self.conn.execute(
            "INSERT INTO requisites (owner_type, owner_id, country, legal_form, name, fields, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (owner_type, owner_id) DO UPDATE SET
                 country = excluded.country, legal_form = excluded.legal_form,
                 name = excluded.name, fields = excluded.fields, updated_at = excluded.updated_at",
            (
                owner_type,
                owner_id,
                &input.country,
                &input.legal_form,
                clean(input.name),
                fields,
                self.clock.now(),
            ),
        )?;
        let requisites = self.get(owner)?.expect("just saved");
        Ok(SaveRequisites::Saved { requisites })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;

    const T0: i64 = 1_700_000_000_000;

    struct Fixture {
        db: Db,
        clock: FakeClock,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                db: Db::open_in_memory().unwrap(),
                clock: FakeClock::at(Timestamp::from_millis(T0)),
            }
        }

        fn service(&self) -> RequisitesService<'_> {
            RequisitesService::new(self.db.conn(), &self.clock)
        }

        fn clients(&self) -> ClientRepo<'_> {
            ClientRepo::new(self.db.conn(), &self.clock)
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
            self.clients().create(input).unwrap().id
        }

        fn saved(&self, owner: RequisitesOwner, input: RequisitesInput) -> Requisites {
            match self.service().save(owner, input).unwrap() {
                SaveRequisites::Saved { requisites } => requisites,
                SaveRequisites::Invalid { errors } => panic!("invalid: {errors:?}"),
            }
        }
    }

    fn values(pairs: &[(&str, &str)]) -> Values {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    fn input(country: &str, form: &str, pairs: &[(&str, &str)]) -> RequisitesInput {
        RequisitesInput {
            country: country.into(),
            legal_form: form.into(),
            name: None,
            values: values(pairs),
            confirm_country_change: false,
        }
    }

    fn ooo() -> RequisitesInput {
        input(
            "RU",
            "ooo",
            &[
                ("inn", "7707083893"),
                ("kpp", "773601001"),
                ("ogrn", "1027700132195"),
                ("address", "Москва, ул. Вавилова, 19"),
                ("bik", "044525225"),
            ],
        )
    }

    #[test]
    fn nothing_is_saved_initially() {
        let f = Fixture::new();
        assert_eq!(f.service().get(RequisitesOwner::Profile).unwrap(), None);
        assert_eq!(
            f.service()
                .get(RequisitesOwner::Client(f.client()))
                .unwrap(),
            None
        );
    }

    #[test]
    fn saves_normalized_requisites_of_a_client() {
        let f = Fixture::new();
        let owner = RequisitesOwner::Client(f.client());
        let mut data = ooo();
        data.values.insert("inn".into(), "7707 083 893".into());
        let saved = f.saved(owner, data);
        assert_eq!(saved.values["inn"], "7707083893");
        assert_eq!(saved.country, "RU");
        assert_eq!(saved.legal_form, "ooo");
        assert_eq!(saved.updated_at, Timestamp::from_millis(T0));
        assert_eq!(f.service().get(owner).unwrap(), Some(saved));
    }

    #[test]
    fn invalid_requisites_are_reported_and_not_saved() {
        let f = Fixture::new();
        let owner = RequisitesOwner::Profile;
        let result = f
            .service()
            .save(owner, input("RU", "ooo", &[("inn", "123")]))
            .unwrap();
        let SaveRequisites::Invalid { errors } = result else {
            panic!("expected errors");
        };
        let fields: Vec<_> = errors.iter().map(|e| e.field.as_str()).collect();
        assert_eq!(fields, ["inn", "kpp", "ogrn", "address"]);
        assert_eq!(f.service().get(owner).unwrap(), None);
    }

    #[test]
    fn profile_keeps_its_name() {
        let f = Fixture::new();
        let data = RequisitesInput {
            name: Some("  ИП Иванов Иван  ".into()),
            ..input("RU", "self_employed", &[("inn", "500100732259")])
        };
        let saved = f.saved(RequisitesOwner::Profile, data);
        assert_eq!(saved.name.as_deref(), Some("ИП Иванов Иван"));
    }

    #[test]
    fn saving_again_replaces_and_drops_fields_of_the_old_legal_form() {
        let f = Fixture::new();
        let owner = RequisitesOwner::Profile;
        f.saved(owner, ooo());
        f.clock.advance_millis(1_000);
        let saved = f.saved(
            owner,
            input(
                "RU",
                "self_employed",
                &[("inn", "500100732259"), ("kpp", "773601001")],
            ),
        );
        assert_eq!(saved.values, values(&[("inn", "500100732259")]));
        assert_eq!(saved.updated_at, Timestamp::from_millis(T0 + 1_000));
    }

    #[test]
    fn changing_the_country_needs_confirmation() {
        let f = Fixture::new();
        let owner = RequisitesOwner::Profile;
        f.saved(owner, ooo());
        let intl = input(
            "INTL",
            "company",
            &[
                ("address", "1 Main St"),
                ("inn", "7707083893"),
                ("iban", "DE89370400440532013000"),
            ],
        );
        assert_eq!(
            f.service().save(owner, intl.clone()).unwrap_err(),
            CoreError::new("requisites.country_change_unconfirmed")
                .with("from", "RU")
                .with("to", "INTL")
        );
        assert_eq!(f.service().get(owner).unwrap().unwrap().country, "RU");

        let saved = f.saved(
            owner,
            RequisitesInput {
                confirm_country_change: true,
                ..intl
            },
        );
        // Fields the new country does not have are cleared.
        assert_eq!(
            saved.values,
            values(&[("address", "1 Main St"), ("iban", "DE89370400440532013000")])
        );
    }

    #[test]
    fn unknown_country_and_client_are_errors() {
        let f = Fixture::new();
        assert_eq!(
            f.service()
                .save(RequisitesOwner::Profile, input("ZZ", "x", &[]))
                .unwrap_err(),
            CoreError::new("requisites.unknown_country").with("country", "ZZ")
        );
        let missing = Id::new();
        assert_eq!(
            f.service()
                .save(RequisitesOwner::Client(missing), ooo())
                .unwrap_err(),
            CoreError::not_found("client", missing)
        );
    }

    #[test]
    fn deleting_a_client_deletes_its_requisites() {
        let f = Fixture::new();
        let id = f.client();
        f.saved(RequisitesOwner::Client(id), ooo());
        f.clients().delete(id).unwrap();
        let left: i64 =
            f.db.conn()
                .query_row("SELECT count(*) FROM requisites", [], |r| r.get(0))
                .unwrap();
        assert_eq!(left, 0);
    }

    #[test]
    fn outcome_serializes_with_a_status_tag() {
        let invalid = SaveRequisites::Invalid {
            errors: vec![FieldError {
                field: "inn".into(),
                rule: "required".into(),
            }],
        };
        assert_eq!(
            serde_json::to_value(invalid).unwrap(),
            serde_json::json!({ "status": "invalid", "errors": [{ "field": "inn", "rule": "required" }] })
        );
        assert_eq!(
            serde_json::to_value(RequisitesOwner::Profile).unwrap(),
            serde_json::json!({ "type": "profile" })
        );
    }
}
