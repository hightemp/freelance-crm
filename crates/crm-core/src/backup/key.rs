//! How a backup file is locked. The file starts with a plain header line
//! saying so, then the age payload:
//!
//! - by a **passphrase** the user chose (manual backups, and automatic ones
//!   whose passphrase is kept in the OS keychain);
//! - by the **vault**: a random passphrase sealed with the vault's DEK, with
//!   the DEK wrapped as in the vault (Argon2id salt and parameters). The
//!   master password opens it on any machine; the header alone gives
//!   nothing without it.

use std::io::{BufRead, Read, Write};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::vault::Vault;
use crate::vault::crypto::{self, KdfParams, Key, Sealed};
use crate::{CoreError, CoreResult};

/// The first line of every backup file.
pub const MAGIC: &[u8] = b"fcrm-backup/1\n";
/// Binds a sealed backup passphrase to its purpose.
const PASSPHRASE_AAD: &[u8] = b"freelance-crm/backup-passphrase/v1";
/// Length of a generated passphrase, in random bytes.
const GENERATED_BYTES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum KeyKind {
    /// Opened by a backup passphrase.
    Passphrase,
    /// Opened by the master password of the vault it was made with.
    Vault,
}

/// What the master password needs to recover a backup passphrase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultLock {
    kdf: KdfParams,
    /// Hex fields, as in the vault.
    salt: String,
    dek_nonce: String,
    wrapped_dek: String,
    passphrase_nonce: String,
    sealed_passphrase: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub key: KeyKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault: Option<VaultLock>,
}

/// The passphrase a backup is encrypted with, and how to tell it again.
pub enum BackupKey {
    Passphrase(Zeroizing<String>),
    Vault {
        lock: VaultLock,
        passphrase: Zeroizing<String>,
    },
}

impl BackupKey {
    pub fn passphrase(text: &str) -> Self {
        Self::Passphrase(Zeroizing::new(text.to_owned()))
    }

    pub(crate) fn secret(&self) -> &str {
        match self {
            Self::Passphrase(p) | Self::Vault { passphrase: p, .. } => p,
        }
    }

    fn header(&self) -> Header {
        match self {
            Self::Passphrase(_) => Header {
                key: KeyKind::Passphrase,
                vault: None,
            },
            Self::Vault { lock, .. } => Header {
                key: KeyKind::Vault,
                vault: Some(lock.clone()),
            },
        }
    }
}

/// What the user types to open a backup.
pub enum Unlock<'a> {
    Passphrase(&'a str),
    MasterPassword(&'a str),
}

/// A passphrase for automatic backups locked by the vault: generated once
/// while the vault is unlocked, kept as text for the scheduler and sealed
/// with the DEK for the header.
pub struct VaultPassphrase {
    pub passphrase: Zeroizing<String>,
    pub sealed: Sealed,
}

impl VaultPassphrase {
    pub fn generate(vault: &Vault) -> CoreResult<Self> {
        let mut bytes = Zeroizing::new([0u8; GENERATED_BYTES]);
        getrandom::fill(bytes.as_mut_slice()).expect("the OS random number generator is available");
        let passphrase = Zeroizing::new(super::hex(bytes.as_slice()));
        let sealed =
            vault.with_dek(|dek| Ok(crypto::seal(dek, passphrase.as_bytes(), PASSPHRASE_AAD)))?;
        Ok(Self { passphrase, sealed })
    }

    /// The key for a backup now: the vault's current wrap of the DEK, so
    /// the current master password opens it.
    pub fn key(&self, conn: &Connection) -> CoreResult<BackupKey> {
        let (kdf, salt, wrapped) = crate::vault::state::wrapped_dek(conn)?;
        Ok(BackupKey::Vault {
            lock: VaultLock {
                kdf,
                salt: super::hex(&salt),
                dek_nonce: super::hex(&wrapped.nonce),
                wrapped_dek: super::hex(&wrapped.ciphertext),
                passphrase_nonce: super::hex(&self.sealed.nonce),
                sealed_passphrase: super::hex(&self.sealed.ciphertext),
            },
            passphrase: self.passphrase.clone(),
        })
    }
}

pub(crate) fn write_header(out: &mut impl Write, key: &BackupKey) -> std::io::Result<()> {
    out.write_all(MAGIC)?;
    serde_json::to_writer(&mut *out, &key.header())?;
    out.write_all(b"\n")
}

/// Reads the header lines, leaving `input` at the age payload.
pub(crate) fn read_header(input: &mut impl BufRead) -> CoreResult<Header> {
    let corrupt = || CoreError::new("backup.corrupt");
    let mut magic = vec![0; MAGIC.len()];
    input.read_exact(&mut magic).map_err(|_| corrupt())?;
    if magic != MAGIC {
        return Err(corrupt());
    }
    // A header is small; anything longer is not one.
    let mut line = Vec::new();
    Read::take(&mut *input, 64 * 1024)
        .read_until(b'\n', &mut line)
        .map_err(|_| corrupt())?;
    serde_json::from_slice(&line).map_err(|_| corrupt())
}

impl Header {
    /// The key that opens the backup, from what the user typed.
    pub(crate) fn key(&self, unlock: Unlock<'_>) -> CoreResult<BackupKey> {
        let wrong = || CoreError::new("backup.wrong_password");
        match (self.key, unlock, &self.vault) {
            (KeyKind::Passphrase, Unlock::Passphrase(p), _) => Ok(BackupKey::passphrase(p)),
            (KeyKind::Vault, Unlock::MasterPassword(password), Some(lock)) => {
                let corrupt = |_| CoreError::new("backup.corrupt");
                let salt: [u8; crypto::SALT_LEN] =
                    unhex(&lock.salt)?.try_into().map_err(corrupt)?;
                let kek = crypto::derive_kek(password.as_bytes(), &salt, lock.kdf)?;
                let dek: Key = crypto::unwrap(
                    &kek,
                    &Sealed {
                        nonce: unhex(&lock.dek_nonce)?.try_into().map_err(corrupt)?,
                        ciphertext: unhex(&lock.wrapped_dek)?,
                    },
                )
                .map_err(|_| wrong())?;
                let passphrase = crypto::open(
                    &dek,
                    &Sealed {
                        nonce: unhex(&lock.passphrase_nonce)?.try_into().map_err(corrupt)?,
                        ciphertext: unhex(&lock.sealed_passphrase)?,
                    },
                    PASSPHRASE_AAD,
                )
                .map_err(|_| CoreError::new("backup.corrupt"))?;
                let passphrase = String::from_utf8(passphrase.to_vec())
                    .map_err(|_| CoreError::new("backup.corrupt"))?;
                Ok(BackupKey::Vault {
                    lock: lock.clone(),
                    passphrase: Zeroizing::new(passphrase),
                })
            }
            (KeyKind::Vault, _, None) => Err(CoreError::new("backup.corrupt")),
            // The form asked for the other kind of secret.
            _ => Err(wrong()),
        }
    }
}

fn unhex(text: &str) -> CoreResult<Vec<u8>> {
    let corrupt = || CoreError::new("backup.corrupt");
    if !text.len().is_multiple_of(2) {
        return Err(corrupt());
    }
    (0..text.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(text.get(i..i + 2).ok_or_else(corrupt)?, 16).map_err(|_| corrupt())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::tests::{LOG_N, clock};
    use crate::backup::{export_with, inspect, unpack};
    use crate::db::Db;
    use crate::domain::{Clock, FakeClock};
    use std::sync::Arc;

    const FAST: KdfParams = KdfParams {
        m_kib: 64,
        t: 1,
        p: 1,
    };

    #[test]
    fn vault_locked_backups_open_with_the_master_password_anywhere() {
        let dir = tempfile::tempdir().unwrap();
        let clock = clock();
        let db = Db::open(&dir.path().join("crm.db")).unwrap();
        let vault = Vault::new(FAST, Arc::new(FakeClock::at(clock.now())));
        vault.init(db.conn(), "master password").unwrap();
        let generated = VaultPassphrase::generate(&vault).unwrap();
        // The scheduler needs no unlocked vault.
        vault.lock();
        let dest = dir.path().join("auto.fcrm-backup");
        export_with(
            db.conn(),
            dir.path(),
            &dest,
            &generated.key(db.conn()).unwrap(),
            &clock,
            LOG_N,
        )
        .unwrap();
        assert_eq!(inspect(&dest).unwrap(), KeyKind::Vault);
        let raw = std::fs::read(&dest).unwrap();
        let text = generated.passphrase.as_bytes();
        assert!(
            !raw.windows(text.len()).any(|w| w == text),
            "the passphrase is not in clear"
        );

        // Elsewhere, with only the file and the master password.
        let out = tempfile::tempdir().unwrap();
        let wrong = CoreError::new("backup.wrong_password");
        assert_eq!(
            unpack(
                &dest,
                Unlock::MasterPassword("not the password"),
                out.path()
            )
            .unwrap_err(),
            wrong
        );
        assert_eq!(
            unpack(&dest, Unlock::Passphrase(&generated.passphrase), out.path()).unwrap_err(),
            wrong,
            "a vault backup asks for the master password"
        );
        unpack(&dest, Unlock::MasterPassword("master password"), out.path()).unwrap();
        assert!(out.path().join(crate::backup::DB_ENTRY).is_file());
    }

    #[test]
    fn backups_after_a_password_change_open_with_the_new_one() {
        let dir = tempfile::tempdir().unwrap();
        let clock = clock();
        let db = Db::open(&dir.path().join("crm.db")).unwrap();
        let vault = Vault::new(FAST, Arc::new(FakeClock::at(clock.now())));
        vault.init(db.conn(), "master password").unwrap();
        let generated = VaultPassphrase::generate(&vault).unwrap();
        vault
            .change_password(db.conn(), "master password", "new master password")
            .unwrap();
        let dest = dir.path().join("auto.fcrm-backup");
        export_with(
            db.conn(),
            dir.path(),
            &dest,
            &generated.key(db.conn()).unwrap(),
            &clock,
            LOG_N,
        )
        .unwrap();
        let out = tempfile::tempdir().unwrap();
        unpack(
            &dest,
            Unlock::MasterPassword("new master password"),
            out.path(),
        )
        .unwrap();
    }

    #[test]
    fn headers_must_be_ours() {
        let mut bad: &[u8] = b"age-encryption.org/v1\n";
        assert_eq!(
            read_header(&mut bad).unwrap_err(),
            CoreError::new("backup.corrupt")
        );
        let mut garbage: &[u8] = b"fcrm-backup/1\n{not json\n";
        assert_eq!(
            read_header(&mut garbage).unwrap_err(),
            CoreError::new("backup.corrupt")
        );
        assert!(unhex("abc").is_err());
        assert!(unhex("zz").is_err());
        assert_eq!(unhex("00ff").unwrap(), [0, 255]);
    }
}
