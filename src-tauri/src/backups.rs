//! The automatic backup scheduler (FR-BK-2): each tick backs up when due.
//! The database is held only for the snapshot; a failure is recorded and
//! shown as a system notification, never fatal.

use std::path::PathBuf;

use crm_core::backup::auto;
use crm_core::settings::{BackupKeySource, Locale};

use crate::AppState;

/// Emitted after an automatic backup was written.
pub const DONE_EVENT: &str = "backup://done";

/// Backs up if due; returns the new file.
pub fn tick(state: &AppState) -> Option<PathBuf> {
    let clock = state.clock.as_ref();
    let (job, russian) = {
        let db = state.db.lock().expect("db mutex poisoned");
        let settings = crm_core::settings::load(&db).unwrap_or_default();
        let russian = settings.locale == Some(Locale::Ru);
        let keychain = match settings.auto_backup_key {
            BackupKeySource::Keychain => state.keychain.get().ok().flatten(),
            BackupKeySource::Vault => None,
        };
        (auto::start(db.conn(), clock, keychain), russian)
    };
    let outcome = match job {
        Ok(None) => return None,
        Ok(Some(job)) => job.finish(&state.files_root, clock),
        Err(e) => Err(e),
    };
    {
        let db = state.db.lock().expect("db mutex poisoned");
        let _ = auto::record(db.conn(), clock, &outcome);
    }
    match outcome {
        Ok(path) => Some(path),
        Err(e) => {
            let (title, body) = failure(&e.code, russian);
            let _ = state.notifier.notify(title, body);
            None
        }
    }
}

/// The notification for a failed backup. The backend has no UI locale
/// beyond the setting, so only English and Russian.
fn failure(code: &str, russian: bool) -> (&'static str, &'static str) {
    let title = if russian {
        "Автобэкап не удался"
    } else {
        "Automatic backup failed"
    };
    let body = match (code, russian) {
        ("backup.folder_unavailable", true) => "Папка для бэкапов недоступна.",
        ("backup.folder_unavailable", false) => "The backup folder is unavailable.",
        ("backup.no_passphrase", true) => "Пароль бэкапа не найден в связке ключей.",
        ("backup.no_passphrase", false) => "The backup password is not in the keychain.",
        ("backup.vault_key_missing", true) => "Разблокируйте хранилище, чтобы настроить бэкапы.",
        ("backup.vault_key_missing", false) => "Unlock the vault to set up backups.",
        (_, true) => "Откройте настройки бэкапа, чтобы узнать подробности.",
        (_, false) => "Open the backup settings for details.",
    };
    (title, body)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crm_core::domain::FakeClock;
    use crm_core::settings::{BackupSchedule, Settings};

    use super::*;
    use crate::reminders::tests::{Recording, app, utc};

    fn daily_into(state: &AppState, dir: &std::path::Path, key: BackupKeySource) {
        let mut db = state.db.lock().unwrap();
        crm_core::settings::save(
            &mut db,
            &Settings {
                auto_backup_schedule: BackupSchedule::Daily,
                auto_backup_dir: Some(dir.to_string_lossy().into_owned()),
                auto_backup_key: key,
                ..Settings::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn backs_up_with_the_keychain_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let clock = Arc::new(FakeClock::at(utc("2026-03-10T09:00:00Z")));
        let sent = Recording::default();
        let state = app(dir.path(), clock, sent.clone());
        daily_into(&state, out.path(), BackupKeySource::Keychain);
        state.keychain.set("keychain passphrase").unwrap();
        let made = tick(&state).unwrap();
        assert!(made.starts_with(out.path()));
        assert!(tick(&state).is_none(), "not due again");
        assert!(sent.0.lock().unwrap().is_empty());
    }

    #[test]
    fn an_unavailable_folder_notifies_and_does_not_crash() {
        let dir = tempfile::tempdir().unwrap();
        let clock = Arc::new(FakeClock::at(utc("2026-03-10T09:00:00Z")));
        let sent = Recording::default();
        let state = app(dir.path(), clock, sent.clone());
        state.keychain.set("keychain passphrase").unwrap();
        daily_into(&state, &dir.path().join("gone"), BackupKeySource::Keychain);
        assert!(tick(&state).is_none());
        assert_eq!(
            sent.0.lock().unwrap().as_slice(),
            [(
                "Automatic backup failed".to_owned(),
                "The backup folder is unavailable.".to_owned()
            )]
        );
        // Retried an hour later, not at the next tick.
        assert!(tick(&state).is_none());
        assert_eq!(sent.0.lock().unwrap().len(), 1);
    }

    #[test]
    fn messages_follow_the_language() {
        assert_eq!(
            failure("backup.no_passphrase", true).0,
            "Автобэкап не удался"
        );
        assert_eq!(
            failure("file.write_failed", false).1,
            "Open the backup settings for details."
        );
    }
}
