//! Application-level information shown in the About dialog.

use serde::Serialize;

use crate::CoreResult;
use crate::db::Db;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub schema_version: u32,
}

pub fn app_info(db: &Db) -> CoreResult<AppInfo> {
    Ok(AppInfo {
        version: crate::version().to_owned(),
        schema_version: db.schema_version()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::LATEST_VERSION;

    #[test]
    fn reports_crate_and_schema_versions() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(
            app_info(&db).unwrap(),
            AppInfo {
                version: crate::version().to_owned(),
                schema_version: LATEST_VERSION,
            }
        );
    }

    #[test]
    fn serializes_in_camel_case() {
        let info = AppInfo {
            version: "1.0.0".into(),
            schema_version: 3,
        };
        assert_eq!(
            serde_json::to_value(info).unwrap(),
            serde_json::json!({ "version": "1.0.0", "schemaVersion": 3 })
        );
    }
}
