//! The OS keychain behind a trait (FR-BK-2): it keeps the passphrase of
//! automatic backups when the user allows it. Tests use memory instead.

use std::sync::Mutex;

use zeroize::Zeroizing;

pub trait Keychain: Send + Sync {
    fn get(&self) -> Result<Option<Zeroizing<String>>, String>;
    fn set(&self, secret: &str) -> Result<(), String>;
    fn delete(&self) -> Result<(), String>;
}

const SERVICE: &str = "freelance-crm";
const ACCOUNT: &str = "auto-backup";

/// macOS Keychain, Windows Credential Manager, Secret Service on Linux.
pub struct SystemKeychain;

impl SystemKeychain {
    fn entry() -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| e.to_string())
    }
}

impl Keychain for SystemKeychain {
    fn get(&self) -> Result<Option<Zeroizing<String>>, String> {
        match Self::entry()?.get_password() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn set(&self, secret: &str) -> Result<(), String> {
        Self::entry()?
            .set_password(secret)
            .map_err(|e| e.to_string())
    }

    fn delete(&self) -> Result<(), String> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Keeps the secret for the life of the process: tests and e2e runs.
#[derive(Default)]
pub struct MemoryKeychain(Mutex<Option<String>>);

impl Keychain for MemoryKeychain {
    fn get(&self) -> Result<Option<Zeroizing<String>>, String> {
        Ok(self.0.lock().unwrap().clone().map(Zeroizing::new))
    }

    fn set(&self, secret: &str) -> Result<(), String> {
        *self.0.lock().unwrap() = Some(secret.to_owned());
        Ok(())
    }

    fn delete(&self) -> Result<(), String> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}
