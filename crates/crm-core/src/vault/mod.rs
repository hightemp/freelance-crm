//! The secrets vault: master password, data key and encrypted secrets.

pub mod crypto;
pub mod secrets;

pub mod state;
pub mod templates;

pub use state::{Vault, VaultStatus};
