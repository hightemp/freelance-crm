pub mod app;
pub mod backup;
pub mod calendar;
pub mod checklists;
pub mod clients;
pub mod contact_methods;
pub mod contacts;
pub mod countries;
pub mod custom_fields;
pub mod db;
pub mod domain;
pub mod error;
pub mod export;
pub mod files;
pub mod finance;
pub mod home;
pub mod interactions;
pub mod notes;
pub mod profile_images;
pub mod projects;
pub mod requisites;
pub mod search;
pub mod settings;
pub mod statuses;
#[cfg(any(test, feature = "synthetic"))]
pub mod synthetic;
pub mod tags;
pub mod tasks;
pub mod time_entries;
pub mod time_report;
pub mod vault;

pub use error::{CoreError, CoreResult};

/// Version of the core crate, shown in the About dialog.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    /// One version for the whole app: the crates, the Tauri config and the
    /// frontend package move together.
    #[test]
    fn versions_agree() {
        let version = super::version();
        let json = |text: &str| -> String {
            serde_json::from_str::<serde_json::Value>(text).unwrap()["version"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        assert_eq!(
            json(include_str!("../../../src-tauri/tauri.conf.json")),
            version
        );
        assert_eq!(json(include_str!("../../../package.json")), version);
        for manifest in [
            include_str!("../../../src-tauri/Cargo.toml"),
            include_str!("../../crm-pdf/Cargo.toml"),
        ] {
            assert!(
                manifest.contains(&format!("\nversion = \"{version}\"\n")),
                "{manifest}"
            );
        }
    }
}
