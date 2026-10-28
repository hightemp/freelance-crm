//! Typed application settings stored as JSON values in the `settings` table.
//! Missing or unreadable rows fall back to defaults, so a corrupted value
//! never prevents the app from starting.

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::domain::Currency;
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    En,
    Ru,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    System,
}

/// UI scale levels offered in the app (percent).
pub const ZOOM_LEVELS: [u16; 6] = [80, 90, 100, 110, 125, 150];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// `None` follows the OS language.
    pub locale: Option<Locale>,
    pub theme: Theme,
    pub default_currency: Currency,
    /// First day of the week: 0 = Sunday … 6 = Saturday.
    pub week_start: u8,
    /// UI scale in percent, one of [`ZOOM_LEVELS`].
    pub zoom_percent: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: None,
            theme: Theme::System,
            default_currency: "USD".parse().expect("valid currency"),
            week_start: 1,
            zoom_percent: 100,
        }
    }
}

fn valid_week_start(day: &u8) -> bool {
    *day <= 6
}

fn valid_zoom(percent: &u16) -> bool {
    ZOOM_LEVELS.contains(percent)
}

impl Settings {
    pub fn validate(&self) -> CoreResult<()> {
        if !valid_week_start(&self.week_start) {
            return Err(CoreError::validation("weekStart", "range"));
        }
        if !valid_zoom(&self.zoom_percent) {
            return Err(CoreError::validation("zoomPercent", "range"));
        }
        Ok(())
    }
}

/// Reads one key; a missing, unparsable or invalid value yields `default`.
fn read<T: DeserializeOwned>(
    rows: &HashMap<String, String>,
    key: &str,
    default: T,
    valid: impl Fn(&T) -> bool,
) -> T {
    rows.get(key)
        .and_then(|raw| serde_json::from_str::<T>(raw).ok())
        .filter(|value| valid(value))
        .unwrap_or(default)
}

pub fn load(db: &Db) -> CoreResult<Settings> {
    let mut stmt = db.conn().prepare("SELECT key, value FROM settings")?;
    let rows: HashMap<String, String> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let d = Settings::default();
    Ok(Settings {
        locale: read(&rows, "locale", d.locale, |_| true),
        theme: read(&rows, "theme", d.theme, |_| true),
        default_currency: read(&rows, "defaultCurrency", d.default_currency, |_| true),
        week_start: read(&rows, "weekStart", d.week_start, valid_week_start),
        zoom_percent: read(&rows, "zoomPercent", d.zoom_percent, valid_zoom),
    })
}

/// Validates and stores all settings atomically, returning what was saved.
pub fn save(db: &mut Db, settings: &Settings) -> CoreResult<Settings> {
    settings.validate()?;
    let serde_json::Value::Object(fields) =
        serde_json::to_value(settings).expect("settings serialize to an object")
    else {
        unreachable!("Settings is a struct");
    };
    let tx = db.conn_mut().transaction()?;
    for (key, value) in fields {
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET
               value = excluded.value,
               updated_at = unixepoch('subsec') * 1000",
            (key, value.to_string()),
        )?;
    }
    tx.commit()?;
    Ok(settings.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    #[test]
    fn empty_database_yields_defaults() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(load(&db).unwrap(), Settings::default());
    }

    #[test]
    fn save_and_load_round_trip() {
        let mut db = Db::open_in_memory().unwrap();
        let settings = Settings {
            locale: Some(Locale::Ru),
            theme: Theme::Dark,
            default_currency: "RUB".parse().unwrap(),
            week_start: 0,
            zoom_percent: 125,
        };
        assert_eq!(save(&mut db, &settings).unwrap(), settings);
        assert_eq!(load(&db).unwrap(), settings);
    }

    #[test]
    fn saving_again_overwrites_values() {
        let mut db = Db::open_in_memory().unwrap();
        save(
            &mut db,
            &Settings {
                theme: Theme::Dark,
                ..Settings::default()
            },
        )
        .unwrap();
        save(
            &mut db,
            &Settings {
                theme: Theme::Light,
                ..Settings::default()
            },
        )
        .unwrap();
        assert_eq!(load(&db).unwrap().theme, Theme::Light);
    }

    #[test]
    fn missing_keys_fall_back_to_defaults() {
        let db = Db::open_in_memory().unwrap();
        db.conn()
            .execute(
                "INSERT INTO settings (key, value) VALUES ('theme', '\"dark\"')",
                [],
            )
            .unwrap();
        let loaded = load(&db).unwrap();
        assert_eq!(loaded.theme, Theme::Dark);
        assert_eq!(loaded.default_currency, usd());
    }

    #[test]
    fn corrupted_values_fall_back_to_defaults() {
        let db = Db::open_in_memory().unwrap();
        db.conn()
            .execute(
                "INSERT INTO settings (key, value) VALUES
                 ('theme', 'not json'), ('defaultCurrency', '\"XXX\"'), ('weekStart', '42')",
                [],
            )
            .unwrap();
        assert_eq!(load(&db).unwrap(), Settings::default());
    }

    #[test]
    fn invalid_week_start_is_rejected() {
        let mut db = Db::open_in_memory().unwrap();
        let err = save(
            &mut db,
            &Settings {
                week_start: 7,
                ..Settings::default()
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "validation.range");
        assert_eq!(err.params["field"], "weekStart");
        assert_eq!(load(&db).unwrap(), Settings::default());
    }

    #[test]
    fn invalid_zoom_is_rejected() {
        let mut db = Db::open_in_memory().unwrap();
        let err = save(
            &mut db,
            &Settings {
                zoom_percent: 333,
                ..Settings::default()
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "validation.range");
        assert_eq!(err.params["field"], "zoomPercent");
    }

    #[test]
    fn serializes_for_the_frontend_in_camel_case() {
        let json = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "locale": null, "theme": "system", "defaultCurrency": "USD",
                "weekStart": 1, "zoomPercent": 100
            })
        );
    }
}
