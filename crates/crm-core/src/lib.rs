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
    #[test]
    fn version_matches_cargo_manifest() {
        assert_eq!(super::version(), "0.1.0");
    }
}
