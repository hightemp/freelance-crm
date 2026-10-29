//! The secrets vault: master password, data key and encrypted secrets.

pub mod crypto;

pub mod state;

pub use state::{Vault, VaultStatus};
