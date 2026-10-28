//! Machine-readable errors. The frontend translates `code` (with `params`)
//! into the user's language; backend never sends human text.

use serde::Serialize;
use serde_json::{Map, Value};

use crate::db::DbError;
use crate::domain::money::MoneyError;

#[derive(Debug, Clone, PartialEq, Serialize, thiserror::Error)]
#[error("{code}")]
pub struct CoreError {
    pub code: String,
    pub params: Map<String, Value>,
}

pub type CoreResult<T> = Result<T, CoreError>;

impl CoreError {
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            params: Map::new(),
        }
    }

    pub fn with(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.params.insert(key.to_owned(), value.into());
        self
    }

    pub fn validation(field: &str, rule: &str) -> Self {
        Self::new(format!("validation.{rule}")).with("field", field)
    }
}

impl From<DbError> for CoreError {
    fn from(_e: DbError) -> Self {
        // Details stay in backend logs; the UI only needs to know it failed.
        Self::new("internal.db")
    }
}

impl From<MoneyError> for CoreError {
    fn from(e: MoneyError) -> Self {
        match e {
            MoneyError::UnknownCurrency => Self::new("money.unknown_currency"),
            MoneyError::Overflow => Self::new("money.overflow"),
            MoneyError::CurrencyMismatch(left, right) => Self::new("money.currency_mismatch")
                .with("left", left.code())
                .with("right", right.code()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::money::Currency;
    use serde_json::json;

    #[test]
    fn serializes_as_code_and_params() {
        let err = CoreError::new("client.not_found").with("id", "abc");
        assert_eq!(
            serde_json::to_value(&err).unwrap(),
            json!({ "code": "client.not_found", "params": { "id": "abc" } })
        );
    }

    #[test]
    fn params_default_to_empty_object() {
        assert_eq!(
            serde_json::to_value(CoreError::new("vault.locked")).unwrap(),
            json!({ "code": "vault.locked", "params": {} })
        );
    }

    #[test]
    fn validation_helper() {
        let err = CoreError::validation("email", "format");
        assert_eq!(err.code, "validation.format");
        assert_eq!(err.params["field"], json!("email"));
    }

    #[test]
    fn money_errors_map_to_codes() {
        let rub: Currency = "RUB".parse().unwrap();
        let usd: Currency = "USD".parse().unwrap();
        assert_eq!(CoreError::from(MoneyError::Overflow).code, "money.overflow");
        assert_eq!(
            CoreError::from(MoneyError::UnknownCurrency).code,
            "money.unknown_currency"
        );
        let mismatch = CoreError::from(MoneyError::CurrencyMismatch(rub, usd));
        assert_eq!(mismatch.code, "money.currency_mismatch");
        assert_eq!(mismatch.params["left"], json!("RUB"));
        assert_eq!(mismatch.params["right"], json!("USD"));
    }

    #[test]
    fn db_errors_do_not_leak_details() {
        let err = CoreError::from(DbError::Sqlite(rusqlite::Error::InvalidQuery));
        assert_eq!(err.code, "internal.db");
        assert!(err.params.is_empty());
    }
}
