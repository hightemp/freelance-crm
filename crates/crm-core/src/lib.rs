pub mod app;
pub mod clients;
pub mod contact_methods;
pub mod contacts;
pub mod countries;
pub mod custom_fields;
pub mod db;
pub mod domain;
pub mod error;
pub mod notes;
pub mod projects;
pub mod requisites;
pub mod settings;
pub mod tags;

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
