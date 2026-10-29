//! Opening and exporting attachments (FR-FI-4, 5). The system app gets a
//! copy named after the attachment in `<root>/open`: plain files under
//! `plain/`, decrypted ones under `secure/`, which is wiped when the vault
//! locks and when the app exits. Leftovers (a crash) are wiped at start.

use std::path::{Path, PathBuf};

use super::attachments::Attachments;
use crate::domain::Id;
use crate::vault::Vault;
use crate::{CoreError, CoreResult};

pub struct OpenFiles {
    dir: PathBuf,
}

impl OpenFiles {
    pub fn new(root: &Path) -> Self {
        Self {
            dir: root.join("open"),
        }
    }

    /// Wipes decrypted copies whenever `vault` locks.
    pub fn clear_on_lock(&self, vault: &Vault) {
        let secure = self.dir.join(SECURE);
        vault.on_lock(move || remove_dir(&secure));
    }

    /// Writes a copy for the system app to open; returns its path.
    pub fn copy_for_opening(&self, files: &Attachments, id: Id) -> CoreResult<PathBuf> {
        let attachment = files.get(id)?;
        let sub = if attachment.encrypted { SECURE } else { PLAIN };
        let dir = self.dir.join(sub).join(id.to_string());
        std::fs::create_dir_all(&dir).map_err(write_failed)?;
        let path = dir.join(safe_file_name(&attachment.file_name));
        write_atomically(&path, |file| files.write_to(id, file))?;
        Ok(path)
    }

    /// Removes decrypted copies.
    pub fn clear_secure(&self) {
        remove_dir(&self.dir.join(SECURE));
    }

    /// Removes every copy (at start and exit).
    pub fn clear_all(&self) {
        remove_dir(&self.dir);
    }
}

const PLAIN: &str = "plain";
const SECURE: &str = "secure";

fn remove_dir(dir: &Path) {
    // Best effort: a file still open on Windows cannot be removed.
    let _ = std::fs::remove_dir_all(dir);
}

fn write_failed(_: std::io::Error) -> CoreError {
    CoreError::new("file.write_failed")
}

/// Writes through a temp file next to `path`, renamed only on success.
fn write_atomically(
    path: &Path,
    write: impl FnOnce(&mut std::io::BufWriter<std::fs::File>) -> CoreResult<()>,
) -> CoreResult<()> {
    use std::io::Write;

    let dir = path
        .parent()
        .ok_or_else(|| CoreError::new("file.write_failed"))?;
    let temp = dir.join(format!(".{}.part", Id::new()));
    let result = (|| {
        let mut out = std::io::BufWriter::new(std::fs::File::create(&temp).map_err(write_failed)?);
        write(&mut out)?;
        out.flush().map_err(write_failed)?;
        drop(out);
        std::fs::rename(&temp, path).map_err(write_failed)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

/// "Save as": writes the (decrypted) content to `dest`, replacing it only
/// once fully written.
pub fn export(files: &Attachments, id: Id, dest: &Path) -> CoreResult<()> {
    write_atomically(dest, |file| files.write_to(id, file))
}

/// A file name safe to create inside a directory on every platform.
fn safe_file_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() || cleaned.chars().all(|c| c == '.') {
        "file".to_owned()
    } else {
        cleaned.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::{EntityRef, FakeClock, Timestamp};
    use crate::vault::crypto::KdfParams;
    use std::io::Cursor;
    use std::sync::Arc;

    struct Fixture {
        dir: tempfile::TempDir,
        db: Db,
        clock: Arc<FakeClock>,
        vault: Vault,
        owner: EntityRef,
    }

    impl Fixture {
        fn new() -> Self {
            let clock = Arc::new(FakeClock::at(Timestamp::from_millis(1_700_000_000_000)));
            let db = Db::open_in_memory().unwrap();
            let vault = Vault::new(
                KdfParams {
                    m_kib: 64,
                    t: 1,
                    p: 1,
                },
                clock.clone(),
            );
            vault.init(db.conn(), "master password").unwrap();
            let client = ClientRepo::new(db.conn(), clock.as_ref())
                .create(ClientInput {
                    kind: ClientKind::Company,
                    last_name: None,
                    first_name: None,
                    middle_name: None,
                    company_name: Some("Ромашка".into()),
                    legal_name: None,
                    status: ClientStatus::Active,
                    default_currency: None,
                    default_rate_minor: None,
                })
                .unwrap();
            Self {
                dir: tempfile::tempdir().unwrap(),
                db,
                clock,
                vault,
                owner: EntityRef::Client(client.id),
            }
        }

        fn files(&self) -> Attachments<'_> {
            Attachments::new(
                self.dir.path(),
                self.db.conn(),
                self.clock.as_ref(),
                &self.vault,
            )
        }

        fn open(&self) -> OpenFiles {
            OpenFiles::new(self.dir.path())
        }

        fn attach(&self, name: &str, data: &[u8], encrypt: bool) -> Id {
            self.files()
                .attach(self.owner, name, Cursor::new(data.to_vec()), encrypt)
                .unwrap()
                .id
        }
    }

    #[test]
    fn plain_files_are_copied_under_their_name() {
        let f = Fixture::new();
        let id = f.attach("Договор.pdf", b"%PDF", false);
        let path = f.open().copy_for_opening(&f.files(), id).unwrap();
        assert_eq!(path.file_name().unwrap(), "Договор.pdf");
        assert!(path.starts_with(f.dir.path().join("open").join("plain")));
        assert_eq!(std::fs::read(&path).unwrap(), b"%PDF");
        // Opening again replaces the copy.
        assert_eq!(f.open().copy_for_opening(&f.files(), id).unwrap(), path);
    }

    #[test]
    fn decrypted_copies_disappear_when_the_vault_locks() {
        let f = Fixture::new();
        let open = f.open();
        open.clear_on_lock(&f.vault);
        let plain = f.attach("a.txt", b"plain", false);
        let secret = f.attach("passport.jpg", b"JPEG", true);
        let plain_copy = open.copy_for_opening(&f.files(), plain).unwrap();
        let secure_copy = open.copy_for_opening(&f.files(), secret).unwrap();
        assert!(secure_copy.starts_with(f.dir.path().join("open").join("secure")));
        assert_eq!(std::fs::read(&secure_copy).unwrap(), b"JPEG");

        f.vault.lock();
        assert!(!secure_copy.exists());
        assert!(plain_copy.exists(), "plain copies are not secret");
        assert_eq!(
            open.copy_for_opening(&f.files(), secret).unwrap_err(),
            CoreError::new("vault.locked")
        );
    }

    #[test]
    fn auto_lock_also_wipes_decrypted_copies() {
        let f = Fixture::new();
        let open = f.open();
        open.clear_on_lock(&f.vault);
        f.vault.set_autolock_minutes(1);
        let secret = f.attach("a.txt", b"x", true);
        let copy = open.copy_for_opening(&f.files(), secret).unwrap();
        f.clock.advance_millis(60_000);
        assert!(f.vault.tick());
        assert!(!copy.exists());
    }

    #[test]
    fn leftovers_are_wiped_at_start() {
        let f = Fixture::new();
        let plain = f.attach("a.txt", b"x", false);
        let secret = f.attach("b.txt", b"y", true);
        let a = f.open().copy_for_opening(&f.files(), plain).unwrap();
        let b = f.open().copy_for_opening(&f.files(), secret).unwrap();
        // A fresh start after a crash.
        f.open().clear_all();
        assert!(!a.exists() && !b.exists());
        assert!(!f.dir.path().join("open").exists());
        f.open().clear_all(); // Nothing to remove is fine.
    }

    #[test]
    fn export_writes_the_decrypted_content() {
        let f = Fixture::new();
        let secret = f.attach("key.pem", b"PRIVATE", true);
        let dest = f.dir.path().join("out").join("key.pem");
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        export(&f.files(), secret, &dest).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"PRIVATE");

        f.vault.lock();
        let other = f.dir.path().join("out").join("again.pem");
        assert_eq!(
            export(&f.files(), secret, &other).unwrap_err(),
            CoreError::new("vault.locked")
        );
        assert!(!other.exists());
        assert_eq!(
            std::fs::read_dir(dest.parent().unwrap()).unwrap().count(),
            1,
            "no partial files left"
        );
    }

    #[test]
    fn unsafe_names_are_cleaned() {
        assert_eq!(safe_file_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_file_name(r"C:\Users\a.txt"), "a.txt");
        assert_eq!(safe_file_name("a:b*c?.txt"), "a_b_c_.txt");
        assert_eq!(safe_file_name(".."), "file");
        assert_eq!(safe_file_name(""), "file");
        assert_eq!(safe_file_name("Смета 2026.xlsx"), "Смета 2026.xlsx");
    }
}
