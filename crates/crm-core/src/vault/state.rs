//! Vault lifecycle (FR-VA-1, 2, 10): `uninitialized` → `unlocked` ⇄
//! `locked`. The wrapped DEK lives in `vault_meta`; the plain DEK only in
//! this process's memory while unlocked.

use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::crypto::{self, KdfParams, Key, Sealed};
use crate::domain::{Clock, Timestamp};
use crate::{CoreError, CoreResult};

pub const MIN_PASSWORD_LEN: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum VaultStatus {
    Uninitialized,
    Locked,
    Unlocked,
}

/// Stored wrap of the DEK with what is needed to derive the KEK again.
struct Meta {
    kdf: KdfParams,
    salt: [u8; crypto::SALT_LEN],
    wrapped: Sealed,
}

fn load_meta(conn: &Connection) -> CoreResult<Option<Meta>> {
    let row = conn
        .query_row(
            "SELECT kdf_m_kib, kdf_t, kdf_p, salt, wrap_nonce, wrapped_dek FROM vault_meta
             WHERE id = 1",
            [],
            |r| {
                Ok((
                    KdfParams {
                        m_kib: r.get(0)?,
                        t: r.get(1)?,
                        p: r.get(2)?,
                    },
                    r.get::<_, Vec<u8>>(3)?,
                    r.get::<_, Vec<u8>>(4)?,
                    r.get::<_, Vec<u8>>(5)?,
                ))
            },
        )
        .optional()?;
    let Some((kdf, salt, nonce, ciphertext)) = row else {
        return Ok(None);
    };
    let corrupt = |_| CoreError::new("vault.corrupt");
    Ok(Some(Meta {
        kdf,
        salt: salt.try_into().map_err(corrupt)?,
        wrapped: Sealed {
            nonce: nonce.try_into().map_err(corrupt)?,
            ciphertext,
        },
    }))
}

fn check_password(password: &str) -> CoreResult<()> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(CoreError::validation("password", "min_length").with("min", MIN_PASSWORD_LEN));
    }
    Ok(())
}

/// Default inactivity timeout (FR-VA-3).
pub const DEFAULT_AUTOLOCK_MINUTES: u32 = 15;

/// A gap between ticks this much longer than expected means the computer
/// slept (best-effort sleep detection that works on every OS).
pub const SLEEP_GAP_MS: i64 = 2 * 60_000;

struct AutoLock {
    /// 0 means never.
    timeout_ms: i64,
    last_activity: Timestamp,
    last_tick: Option<Timestamp>,
}

pub struct Vault {
    /// Parameters for new key derivations (stored ones are used to unlock).
    kdf: KdfParams,
    clock: Arc<dyn Clock>,
    dek: Mutex<Option<Key>>,
    autolock: Mutex<AutoLock>,
}

impl Vault {
    pub fn new(kdf: KdfParams, clock: Arc<dyn Clock>) -> Self {
        let now = clock.now();
        Self {
            kdf,
            clock,
            dek: Mutex::new(None),
            autolock: Mutex::new(AutoLock {
                timeout_ms: i64::from(DEFAULT_AUTOLOCK_MINUTES) * 60_000,
                last_activity: now,
                last_tick: None,
            }),
        }
    }

    /// Sets the inactivity timeout; 0 disables auto-locking.
    pub fn set_autolock_minutes(&self, minutes: u32) {
        self.autolock().timeout_ms = i64::from(minutes) * 60_000;
    }

    fn autolock(&self) -> std::sync::MutexGuard<'_, AutoLock> {
        self.autolock.lock().expect("vault mutex poisoned")
    }

    /// Records user activity, postponing the auto-lock.
    pub fn touch(&self) {
        self.autolock().last_activity = self.clock.now();
    }

    /// Called periodically: locks after the inactivity timeout or when the
    /// computer apparently slept. Returns true if this call locked the vault.
    pub fn tick(&self) -> bool {
        let now = self.clock.now();
        let due = {
            let mut auto = self.autolock();
            let idle = now.as_millis() - auto.last_activity.as_millis();
            let slept = auto
                .last_tick
                .is_some_and(|last| now.as_millis() - last.as_millis() > SLEEP_GAP_MS);
            auto.last_tick = Some(now);
            slept || (auto.timeout_ms > 0 && idle >= auto.timeout_ms)
        };
        let mut dek = self.dek();
        if due && dek.is_some() {
            *dek = None;
            return true;
        }
        false
    }

    /// Stores the DEK and restarts the inactivity timer.
    fn open_with(&self, dek: Key) {
        *self.dek() = Some(dek);
        let mut auto = self.autolock();
        auto.last_activity = self.clock.now();
        auto.last_tick = None;
    }

    fn dek(&self) -> std::sync::MutexGuard<'_, Option<Key>> {
        self.dek.lock().expect("vault mutex poisoned")
    }

    pub fn status(&self, conn: &Connection) -> CoreResult<VaultStatus> {
        if self.dek().is_some() {
            return Ok(VaultStatus::Unlocked);
        }
        let exists: bool =
            conn.query_row("SELECT EXISTS (SELECT 1 FROM vault_meta)", [], |r| r.get(0))?;
        Ok(if exists {
            VaultStatus::Locked
        } else {
            VaultStatus::Uninitialized
        })
    }

    /// Wraps the DEK with a key from `password` under a fresh salt.
    fn wrap_for(&self, password: &str, dek: &Key) -> CoreResult<([u8; crypto::SALT_LEN], Sealed)> {
        let salt = crypto::random_salt();
        let kek = crypto::derive_kek(password.as_bytes(), &salt, self.kdf)?;
        Ok((salt, crypto::wrap(&kek, dek)))
    }

    /// Creates the vault with a master password and leaves it unlocked.
    pub fn init(&self, conn: &Connection, password: &str) -> CoreResult<()> {
        check_password(password)?;
        if load_meta(conn)?.is_some() {
            return Err(CoreError::new("vault.already_initialized"));
        }
        let dek = Key::random();
        let (salt, wrapped) = self.wrap_for(password, &dek)?;
        conn.execute(
            "INSERT INTO vault_meta (id, version, kdf_m_kib, kdf_t, kdf_p, salt, wrap_nonce,
                 wrapped_dek, created_at, updated_at)
             VALUES (1, 1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            (
                self.kdf.m_kib,
                self.kdf.t,
                self.kdf.p,
                salt.as_slice(),
                wrapped.nonce.as_slice(),
                &wrapped.ciphertext,
                self.clock.now(),
            ),
        )?;
        self.open_with(dek);
        Ok(())
    }

    /// Unwraps the stored DEK; a wrong password fails authentication.
    fn unwrap_with(&self, conn: &Connection, password: &str) -> CoreResult<Key> {
        let meta = load_meta(conn)?.ok_or_else(|| CoreError::new("vault.uninitialized"))?;
        let kek = crypto::derive_kek(password.as_bytes(), &meta.salt, meta.kdf)?;
        crypto::unwrap(&kek, &meta.wrapped)
    }

    pub fn unlock(&self, conn: &Connection, password: &str) -> CoreResult<()> {
        let dek = self.unwrap_with(conn, password)?;
        self.open_with(dek);
        Ok(())
    }

    /// Forgets the DEK (it is zeroized on drop).
    pub fn lock(&self) {
        *self.dek() = None;
    }

    /// Re-wraps the same DEK with a key from the new password; secrets are
    /// not re-encrypted (FR-VA-10).
    pub fn change_password(&self, conn: &Connection, old: &str, new: &str) -> CoreResult<()> {
        check_password(new)?;
        let dek = self.unwrap_with(conn, old)?;
        let (salt, wrapped) = self.wrap_for(new, &dek)?;
        conn.execute(
            "UPDATE vault_meta SET kdf_m_kib = ?1, kdf_t = ?2, kdf_p = ?3, salt = ?4,
                 wrap_nonce = ?5, wrapped_dek = ?6
             WHERE id = 1",
            (
                self.kdf.m_kib,
                self.kdf.t,
                self.kdf.p,
                salt.as_slice(),
                wrapped.nonce.as_slice(),
                &wrapped.ciphertext,
            ),
        )?;
        self.open_with(dek);
        Ok(())
    }

    /// Runs `f` with the DEK; fails with `vault.locked` when locked.
    pub fn with_dek<T>(&self, f: impl FnOnce(&Key) -> CoreResult<T>) -> CoreResult<T> {
        let guard = self.dek();
        let dek = guard
            .as_ref()
            .ok_or_else(|| CoreError::new("vault.locked"))?;
        f(dek)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::domain::FakeClock;

    const FAST: KdfParams = KdfParams {
        m_kib: 64,
        t: 1,
        p: 1,
    };
    const PASSWORD: &str = "correct horse battery";

    struct Fixture {
        db: Db,
        clock: Arc<FakeClock>,
        vault: Vault,
    }

    impl Fixture {
        fn new() -> Self {
            let clock = Arc::new(FakeClock::at(Timestamp::from_millis(1_700_000_000_000)));
            Self {
                db: Db::open_in_memory().unwrap(),
                vault: Vault::new(FAST, clock.clone()),
                clock,
            }
        }

        fn restarted(&self, kdf: KdfParams) -> Vault {
            Vault::new(kdf, self.clock.clone())
        }

        fn status(&self) -> VaultStatus {
            self.vault.status(self.db.conn()).unwrap()
        }

        fn init(&self) {
            self.vault.init(self.db.conn(), PASSWORD).unwrap();
        }
    }

    #[test]
    fn state_machine() {
        let f = Fixture::new();
        assert_eq!(f.status(), VaultStatus::Uninitialized);
        f.init();
        assert_eq!(f.status(), VaultStatus::Unlocked);
        f.vault.lock();
        assert_eq!(f.status(), VaultStatus::Locked);
        f.vault.unlock(f.db.conn(), PASSWORD).unwrap();
        assert_eq!(f.status(), VaultStatus::Unlocked);
        // Locking twice is harmless.
        f.vault.lock();
        f.vault.lock();
        assert_eq!(f.status(), VaultStatus::Locked);
    }

    #[test]
    fn a_restarted_app_finds_the_vault_locked() {
        let f = Fixture::new();
        f.init();
        let restarted = f.restarted(FAST);
        assert_eq!(restarted.status(f.db.conn()).unwrap(), VaultStatus::Locked);
        restarted.unlock(f.db.conn(), PASSWORD).unwrap();
        assert_eq!(
            restarted.status(f.db.conn()).unwrap(),
            VaultStatus::Unlocked
        );
    }

    #[test]
    fn init_twice_is_an_error() {
        let f = Fixture::new();
        f.init();
        assert_eq!(
            f.vault.init(f.db.conn(), "another password").unwrap_err(),
            CoreError::new("vault.already_initialized")
        );
    }

    #[test]
    fn passwords_must_be_long_enough() {
        let f = Fixture::new();
        assert_eq!(
            f.vault.init(f.db.conn(), "short").unwrap_err(),
            CoreError::validation("password", "min_length").with("min", MIN_PASSWORD_LEN)
        );
        assert_eq!(f.status(), VaultStatus::Uninitialized);
    }

    #[test]
    fn wrong_password_does_not_unlock() {
        let f = Fixture::new();
        assert_eq!(
            f.vault.unlock(f.db.conn(), PASSWORD).unwrap_err(),
            CoreError::new("vault.uninitialized")
        );
        f.init();
        f.vault.lock();
        assert_eq!(
            f.vault.unlock(f.db.conn(), "wrong password").unwrap_err(),
            CoreError::new("vault.wrong_password")
        );
        assert_eq!(f.status(), VaultStatus::Locked);
    }

    #[test]
    fn locked_vault_refuses_secret_operations() {
        let f = Fixture::new();
        let locked = CoreError::new("vault.locked");
        assert_eq!(f.vault.with_dek(|_| Ok(())).unwrap_err(), locked);
        f.init();
        assert!(f.vault.with_dek(|_| Ok(())).is_ok());
        f.vault.lock();
        assert_eq!(f.vault.with_dek(|_| Ok(())).unwrap_err(), locked);
    }

    #[test]
    fn changing_the_password_keeps_secrets_readable() {
        let f = Fixture::new();
        f.init();
        let sealed = f
            .vault
            .with_dek(|dek| Ok(crypto::seal(dek, b"hunter2", b"s1")))
            .unwrap();

        assert_eq!(
            f.vault
                .change_password(f.db.conn(), "wrong password", "new password 1")
                .unwrap_err(),
            CoreError::new("vault.wrong_password")
        );
        assert_eq!(
            f.vault
                .change_password(f.db.conn(), PASSWORD, "tiny")
                .unwrap_err()
                .code,
            "validation.min_length"
        );
        f.vault
            .change_password(f.db.conn(), PASSWORD, "new password 1")
            .unwrap();

        let restarted = f.restarted(FAST);
        assert_eq!(
            restarted.unlock(f.db.conn(), PASSWORD).unwrap_err(),
            CoreError::new("vault.wrong_password")
        );
        restarted.unlock(f.db.conn(), "new password 1").unwrap();
        let plain = restarted
            .with_dek(|dek| crypto::open(dek, &sealed, b"s1"))
            .unwrap();
        assert_eq!(&*plain, b"hunter2");
    }

    #[test]
    fn kdf_parameters_are_stored_with_the_vault() {
        let f = Fixture::new();
        f.init();
        // A vault created with other parameters still unlocks with its own.
        let stronger = f.restarted(KdfParams {
            m_kib: 128,
            t: 2,
            p: 1,
        });
        stronger.unlock(f.db.conn(), PASSWORD).unwrap();
    }

    const MINUTE: i64 = 60_000;

    #[test]
    fn locks_exactly_after_the_inactivity_timeout() {
        let f = Fixture::new();
        f.vault.set_autolock_minutes(5);
        f.init();
        f.clock.advance_millis(5 * MINUTE - 1);
        assert!(!f.vault.tick());
        assert_eq!(f.status(), VaultStatus::Unlocked);
        f.clock.advance_millis(1);
        assert!(f.vault.tick());
        assert_eq!(f.status(), VaultStatus::Locked);
        // Ticking a locked vault reports nothing new.
        assert!(!f.vault.tick());
    }

    #[test]
    fn activity_resets_the_timer() {
        let f = Fixture::new();
        f.vault.set_autolock_minutes(5);
        f.init();
        // Ticks every minute (as in the app, well below the sleep gap);
        // activity every four minutes keeps the vault open.
        for minute in 1..=12 {
            f.clock.advance_millis(MINUTE);
            if minute % 4 == 0 {
                f.vault.touch();
            }
            assert!(!f.vault.tick(), "minute {minute}");
        }
        for _ in 0..4 {
            f.clock.advance_millis(MINUTE);
            assert!(!f.vault.tick());
        }
        f.clock.advance_millis(MINUTE);
        assert!(f.vault.tick());
    }

    #[test]
    fn unlocking_restarts_the_timer() {
        let f = Fixture::new();
        f.vault.set_autolock_minutes(5);
        f.init();
        f.vault.lock();
        f.clock.advance_millis(10 * MINUTE);
        f.vault.unlock(f.db.conn(), PASSWORD).unwrap();
        assert!(!f.vault.tick());
    }

    #[test]
    fn zero_timeout_means_never() {
        let f = Fixture::new();
        f.vault.set_autolock_minutes(0);
        f.init();
        // Regular ticks over a long time: no sleep gap, no timeout.
        for _ in 0..(24 * 60) {
            f.clock.advance_millis(MINUTE);
            assert!(!f.vault.tick());
        }
        assert_eq!(f.status(), VaultStatus::Unlocked);
    }

    #[test]
    fn a_long_gap_between_ticks_means_the_computer_slept() {
        let f = Fixture::new();
        f.vault.set_autolock_minutes(0);
        f.init();
        assert!(!f.vault.tick());
        f.clock.advance_millis(SLEEP_GAP_MS);
        assert!(!f.vault.tick());
        f.clock.advance_millis(SLEEP_GAP_MS + 1);
        assert!(f.vault.tick());
    }
}
