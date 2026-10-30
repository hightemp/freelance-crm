//! Automatic backups (FR-BK-2): daily or weekly into a chosen folder,
//! keeping the newest N. They are locked by a passphrase from the OS
//! keychain or by the vault ([`super::key`]). A failure (say, the folder
//! is gone) is recorded and retried an hour later, never fatal.
//!
//! A run is split so that the database is held only for its snapshot:
//! [`start`] decides and takes it, [`Job::finish`] packs and rotates
//! without the connection, [`record`] stores the outcome.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;
use zeroize::Zeroizing;

use super::key::VaultPassphrase;
use super::{BackupKey, Snapshot};
use crate::domain::{Clock, Timestamp};
use crate::settings::{BackupKeySource, BackupSchedule};
use crate::vault::Vault;
use crate::{CoreError, CoreResult};

/// Automatic backups are named `auto-<time>.fcrm-backup`; only those are
/// rotated.
pub const PREFIX: &str = "auto-";
/// After a failure, the next attempt waits this long.
pub const RETRY_MS: i64 = 3_600_000;
const DAY_MS: i64 = 24 * 3_600_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AutoBackupStatus {
    pub last_success_at: Option<Timestamp>,
    pub last_attempt_at: Option<Timestamp>,
    /// The error code of the last attempt, if it failed.
    pub last_error: Option<String>,
    /// Vault-locked backups are set up (needs the vault unlocked once).
    pub vault_key_ready: bool,
}

pub fn status(conn: &Connection) -> CoreResult<AutoBackupStatus> {
    Ok(conn.query_row(
        "SELECT last_success_at, last_attempt_at, last_error, vault_passphrase IS NOT NULL
         FROM backup_state WHERE id = 1",
        [],
        |r| {
            Ok(AutoBackupStatus {
                last_success_at: r.get(0)?,
                last_attempt_at: r.get(1)?,
                last_error: r.get(2)?,
                vault_key_ready: r.get(3)?,
            })
        },
    )?)
}

/// Generates the passphrase of vault-locked backups, once.
pub fn set_up_vault_key(conn: &Connection, vault: &Vault) -> CoreResult<()> {
    if status(conn)?.vault_key_ready {
        return Ok(());
    }
    let generated = VaultPassphrase::generate(vault)?;
    conn.execute(
        "UPDATE backup_state SET vault_passphrase = ?1, vault_nonce = ?2, vault_sealed = ?3
         WHERE id = 1",
        (
            generated.passphrase.as_str(),
            generated.sealed.nonce.as_slice(),
            &generated.sealed.ciphertext,
        ),
    )?;
    Ok(())
}

fn vault_passphrase(conn: &Connection) -> CoreResult<Option<VaultPassphrase>> {
    let row: (Option<String>, Option<Vec<u8>>, Option<Vec<u8>>) = conn.query_row(
        "SELECT vault_passphrase, vault_nonce, vault_sealed FROM backup_state WHERE id = 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let (Some(passphrase), Some(nonce), Some(ciphertext)) = row else {
        return Ok(None);
    };
    Ok(Some(VaultPassphrase {
        passphrase: Zeroizing::new(passphrase),
        sealed: crate::vault::crypto::Sealed {
            nonce: nonce
                .try_into()
                .map_err(|_| CoreError::new("backup.corrupt"))?,
            ciphertext,
        },
    }))
}

/// A backup under way, past its snapshot.
pub struct Job {
    snapshot: Snapshot,
    key: BackupKey,
    dest: PathBuf,
    dir: PathBuf,
    keep: u32,
}

/// Whether a backup is due now, and if so its snapshot. `keychain` is the
/// passphrase stored in the OS keychain, when the settings use it. An
/// error here counts as a failed attempt too.
pub fn start(
    conn: &Connection,
    clock: &dyn Clock,
    keychain: Option<Zeroizing<String>>,
) -> CoreResult<Option<Job>> {
    let settings = crate::settings::load_from(conn)?;
    let period = match settings.auto_backup_schedule {
        BackupSchedule::Off => return Ok(None),
        BackupSchedule::Daily => DAY_MS,
        BackupSchedule::Weekly => 7 * DAY_MS,
    };
    let Some(dir) = settings.auto_backup_dir.filter(|d| !d.trim().is_empty()) else {
        return Ok(None);
    };
    let now = clock.now().as_millis();
    let state = status(conn)?;
    let since = |at: Option<Timestamp>| at.map(|t| now - t.as_millis());
    if since(state.last_success_at).is_some_and(|ms| ms < period) {
        return Ok(None);
    }
    if state.last_error.is_some() && since(state.last_attempt_at).is_some_and(|ms| ms < RETRY_MS) {
        return Ok(None);
    }

    let key = match settings.auto_backup_key {
        BackupKeySource::Keychain => {
            BackupKey::Passphrase(keychain.ok_or_else(|| CoreError::new("backup.no_passphrase"))?)
        }
        BackupKeySource::Vault => vault_passphrase(conn)?
            .ok_or_else(|| CoreError::new("backup.vault_key_missing"))?
            .key(conn)?,
    };
    let dir = PathBuf::from(dir);
    if !dir.is_dir() {
        return Err(CoreError::new("backup.folder_unavailable"));
    }
    let stamp = jiff::Timestamp::from_millisecond(now)
        .map_err(|_| CoreError::new("internal.clock"))?
        .strftime("%Y%m%d-%H%M%S");
    let dest = dir.join(format!("{PREFIX}{stamp}.{}", super::EXTENSION));
    let snapshot =
        super::snapshot(conn, &dest).map_err(|_| CoreError::new("backup.folder_unavailable"))?;
    Ok(Some(Job {
        snapshot,
        key,
        dest,
        dir,
        keep: settings.auto_backup_keep,
    }))
}

impl Job {
    /// Packs the backup and drops the oldest beyond the count to keep.
    pub fn finish(self, files_root: &Path, clock: &dyn Clock) -> CoreResult<PathBuf> {
        self.finish_with(files_root, clock, None)
    }

    pub(crate) fn finish_with(
        self,
        files_root: &Path,
        clock: &dyn Clock,
        log_n: Option<u8>,
    ) -> CoreResult<PathBuf> {
        self.snapshot
            .seal_with(files_root, &self.dest, &self.key, clock, log_n)?;
        rotate(&self.dir, self.keep)?;
        Ok(self.dest)
    }
}

/// Stores how an attempt ended.
pub fn record(
    conn: &Connection,
    clock: &dyn Clock,
    outcome: &CoreResult<PathBuf>,
) -> CoreResult<()> {
    let now = clock.now();
    match outcome {
        Ok(_) => conn.execute(
            "UPDATE backup_state SET last_success_at = ?1, last_attempt_at = ?1, last_error = NULL
             WHERE id = 1",
            [now],
        )?,
        Err(e) => conn.execute(
            "UPDATE backup_state SET last_attempt_at = ?1, last_error = ?2 WHERE id = 1",
            (now, &e.code),
        )?,
    };
    Ok(())
}

/// Keeps the newest `keep` automatic backups in `dir`; their names sort
/// by time.
pub fn rotate(dir: &Path, keep: u32) -> CoreResult<()> {
    let unavailable = |_| CoreError::new("backup.folder_unavailable");
    let mut ours: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(unavailable)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                    n.starts_with(PREFIX) && n.ends_with(&format!(".{}", super::EXTENSION))
                })
        })
        .collect();
    ours.sort();
    let extra = ours.len().saturating_sub(keep as usize);
    for old in &ours[..extra] {
        std::fs::remove_file(old).map_err(unavailable)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::tests::{LOG_N, clock};
    use crate::backup::{KeyKind, Unlock, inspect, unpack};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::settings::Settings;
    use crate::vault::crypto::KdfParams;
    use std::sync::Arc;

    const FAST: KdfParams = KdfParams {
        m_kib: 64,
        t: 1,
        p: 1,
    };

    struct Fixture {
        dir: tempfile::TempDir,
        out: PathBuf,
        db: Db,
        clock: FakeClock,
        vault: Vault,
    }

    impl Fixture {
        /// A vault, and daily vault-locked backups into `out`, keeping 3.
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let out = dir.path().join("backups");
            std::fs::create_dir(&out).unwrap();
            let mut db = Db::open(&dir.path().join("crm.db")).unwrap();
            let clock = clock();
            let vault = Vault::new(FAST, Arc::new(FakeClock::at(clock.now())));
            vault.init(db.conn(), "master password").unwrap();
            crate::settings::save(
                &mut db,
                &Settings {
                    auto_backup_schedule: BackupSchedule::Daily,
                    auto_backup_dir: Some(out.to_string_lossy().into_owned()),
                    auto_backup_keep: 3,
                    ..Settings::default()
                },
            )
            .unwrap();
            Self {
                dir,
                out,
                db,
                clock,
                vault,
            }
        }

        /// One scheduler tick: what a backup produced, or why it failed.
        fn tick(&self, keychain: Option<&str>) -> Option<CoreResult<PathBuf>> {
            let job = start(
                self.db.conn(),
                &self.clock,
                keychain.map(|k| Zeroizing::new(k.to_owned())),
            );
            let outcome = match job {
                Ok(None) => return None,
                Ok(Some(job)) => job.finish_with(self.dir.path(), &self.clock, LOG_N),
                Err(e) => Err(e),
            };
            record(self.db.conn(), &self.clock, &outcome).unwrap();
            Some(outcome)
        }

        fn backups(&self) -> Vec<String> {
            let mut names: Vec<_> = std::fs::read_dir(&self.out)
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect();
            names.sort();
            names
        }

        fn hours(&self, n: i64) {
            self.clock.advance_millis(n * 3_600_000);
        }
    }

    #[test]
    fn backs_up_on_schedule() {
        let f = Fixture::new();
        set_up_vault_key(f.db.conn(), &f.vault).unwrap();
        f.vault.lock();

        let first = f.tick(None).unwrap().unwrap();
        assert_eq!(inspect(&first).unwrap(), KeyKind::Vault);
        let out = tempfile::tempdir().unwrap();
        unpack(
            &first,
            Unlock::MasterPassword("master password"),
            out.path(),
        )
        .unwrap();
        // Not again within the day.
        f.hours(23);
        assert!(f.tick(None).is_none());
        f.hours(1);
        assert!(f.tick(None).unwrap().is_ok());
        assert_eq!(f.backups().len(), 2);
        let s = status(f.db.conn()).unwrap();
        assert_eq!(s.last_success_at, Some(f.clock.now()));
        assert_eq!(s.last_error, None);
        assert!(s.vault_key_ready);
    }

    #[test]
    fn nothing_happens_when_off() {
        let mut f = Fixture::new();
        set_up_vault_key(f.db.conn(), &f.vault).unwrap();
        crate::settings::save(&mut f.db, &Settings::default()).unwrap();
        assert!(f.tick(None).is_none());
        assert!(f.backups().is_empty());
    }

    #[test]
    fn weekly_backups_wait_a_week() {
        let mut f = Fixture::new();
        set_up_vault_key(f.db.conn(), &f.vault).unwrap();
        let mut settings = crate::settings::load(&f.db).unwrap();
        settings.auto_backup_schedule = BackupSchedule::Weekly;
        crate::settings::save(&mut f.db, &settings).unwrap();
        assert!(f.tick(None).is_some());
        f.hours(6 * 24 + 23);
        assert!(f.tick(None).is_none());
        f.hours(1);
        assert!(f.tick(None).is_some());
    }

    #[test]
    fn rotation_keeps_the_newest() {
        let f = Fixture::new();
        set_up_vault_key(f.db.conn(), &f.vault).unwrap();
        // Another file in the folder is never touched.
        std::fs::write(f.out.join("notes.txt"), "mine").unwrap();
        let mut made = Vec::new();
        for _ in 0..5 {
            made.push(f.tick(None).unwrap().unwrap());
            f.hours(24);
        }
        let names: Vec<String> = made
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        let mut expected = names[2..].to_vec();
        expected.push("notes.txt".into());
        expected.sort();
        assert_eq!(f.backups(), expected);
    }

    #[test]
    fn a_missing_folder_is_an_error_retried_later() {
        let f = Fixture::new();
        set_up_vault_key(f.db.conn(), &f.vault).unwrap();
        std::fs::remove_dir(&f.out).unwrap();
        let failed = f.tick(None).unwrap().unwrap_err();
        assert_eq!(failed, CoreError::new("backup.folder_unavailable"));
        let s = status(f.db.conn()).unwrap();
        assert_eq!(s.last_error.as_deref(), Some("backup.folder_unavailable"));
        assert_eq!(s.last_success_at, None);
        // Not every minute: an hour later.
        f.hours(1);
        f.clock.advance_millis(-60_000);
        assert!(f.tick(None).is_none());
        std::fs::create_dir(&f.out).unwrap();
        f.clock.advance_millis(60_000);
        assert!(f.tick(None).unwrap().is_ok());
        assert_eq!(status(f.db.conn()).unwrap().last_error, None);
    }

    #[test]
    fn keys_must_be_available() {
        let mut f = Fixture::new();
        // Vault-locked, but never set up.
        assert_eq!(
            f.tick(None).unwrap().unwrap_err(),
            CoreError::new("backup.vault_key_missing")
        );
        // Keychain-locked, without a passphrase there.
        let mut settings = crate::settings::load(&f.db).unwrap();
        settings.auto_backup_key = BackupKeySource::Keychain;
        crate::settings::save(&mut f.db, &settings).unwrap();
        f.hours(1);
        assert_eq!(
            f.tick(None).unwrap().unwrap_err(),
            CoreError::new("backup.no_passphrase")
        );
        f.hours(1);
        let made = f.tick(Some("keychain passphrase")).unwrap().unwrap();
        assert_eq!(inspect(&made).unwrap(), KeyKind::Passphrase);
        let out = tempfile::tempdir().unwrap();
        unpack(&made, Unlock::Passphrase("keychain passphrase"), out.path()).unwrap();
    }

    #[test]
    fn setting_up_the_vault_key_needs_it_unlocked_and_happens_once() {
        let f = Fixture::new();
        f.vault.lock();
        assert_eq!(
            set_up_vault_key(f.db.conn(), &f.vault).unwrap_err(),
            CoreError::new("vault.locked")
        );
        f.vault.unlock(f.db.conn(), "master password").unwrap();
        set_up_vault_key(f.db.conn(), &f.vault).unwrap();
        let first: String =
            f.db.conn()
                .query_row("SELECT vault_passphrase FROM backup_state", [], |r| {
                    r.get(0)
                })
                .unwrap();
        set_up_vault_key(f.db.conn(), &f.vault).unwrap();
        let again: String =
            f.db.conn()
                .query_row("SELECT vault_passphrase FROM backup_state", [], |r| {
                    r.get(0)
                })
                .unwrap();
        assert_eq!(first, again);
    }
}
