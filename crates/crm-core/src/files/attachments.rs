//! Files attached to records (FR-FI-1). Attachments reference blobs; the
//! blob's reference count follows them (triggers), and unreferenced blobs
//! are removed by [`BlobStore::collect_garbage`].

use std::io::Read;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use super::blobs::BlobStore;
use crate::domain::{Clock, EntityRef, Id, Timestamp};
use crate::vault::Vault;
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: Id,
    pub blob_id: String,
    pub file_name: String,
    pub mime: String,
    #[specta(type = i32)]
    pub size: u64,
    pub encrypted: bool,
    pub created_at: Timestamp,
}

pub struct Attachments<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
    blobs: BlobStore<'a>,
    vault: &'a Vault,
}

impl<'a> Attachments<'a> {
    pub fn new(
        root: &'a Path,
        conn: &'a Connection,
        clock: &'a dyn Clock,
        vault: &'a Vault,
    ) -> Self {
        Self {
            conn,
            clock,
            blobs: BlobStore::new(root, conn, clock),
            vault,
        }
    }

    pub fn with_max_size(self, max_size: u64) -> Self {
        Self {
            blobs: self.blobs.with_max_size(max_size),
            ..self
        }
    }

    /// Attaches content under `file_name`. Secrets' files are always
    /// encrypted; others when `encrypt` is set (needs an unlocked vault).
    pub fn attach(
        &self,
        owner: EntityRef,
        file_name: &str,
        reader: impl Read,
        encrypt: bool,
    ) -> CoreResult<Attachment> {
        let file_name = file_name.trim();
        if file_name.is_empty() {
            return Err(CoreError::validation("fileName", "required"));
        }
        owner.ensure_exists(self.conn)?;
        let mime = mime_guess::from_path(file_name)
            .first_or_octet_stream()
            .essence_str()
            .to_owned();
        let blob = if encrypt || matches!(owner, EntityRef::Secret(_)) {
            self.blobs.put_encrypted(reader, &mime, self.vault)?
        } else {
            self.blobs.put(reader, &mime)?
        };
        let id = Id::new();
        let (owner_type, owner_id) = owner.parts();
        let inserted = self.conn.execute(
            "INSERT INTO attachments (id, owner_type, owner_id, blob_id, file_name, mime, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (id, owner_type, owner_id, &blob.id, file_name, &mime, self.clock.now()),
        );
        if let Err(e) = inserted {
            // Give back the reference `put` took.
            let _ = self.blobs.release(&blob.id);
            return Err(e.into());
        }
        self.get(id)
    }

    /// Attaches a file from disk, named after it.
    pub fn attach_path(
        &self,
        owner: EntityRef,
        path: &Path,
        encrypt: bool,
    ) -> CoreResult<Attachment> {
        let file = std::fs::File::open(path).map_err(|_| CoreError::new("file.read_failed"))?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.attach(owner, &name, std::io::BufReader::new(file), encrypt)
    }

    /// Newest first.
    pub fn list(&self, owner: EntityRef) -> CoreResult<Vec<Attachment>> {
        let (owner_type, owner_id) = owner.parts();
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT} WHERE a.owner_type = ?1 AND a.owner_id = ?2
             ORDER BY a.created_at DESC, a.rowid DESC"
        ))?;
        let rows = stmt.query_map((owner_type, owner_id), from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn get(&self, id: Id) -> CoreResult<Attachment> {
        self.conn
            .query_row(&format!("{SELECT} WHERE a.id = ?1"), [id], from_row)
            .optional()?
            .ok_or_else(|| CoreError::not_found("attachment", id))
    }

    /// Removes the attachment and, if it was the last reference, the file.
    pub fn delete(&self, id: Id) -> CoreResult<()> {
        let attachment = self.get(id)?;
        // The trigger drops the blob reference.
        self.conn
            .execute("DELETE FROM attachments WHERE id = ?1", [id])?;
        if self.blobs.get(&attachment.blob_id)?.ref_count == 0 {
            self.blobs.collect_garbage()?;
        }
        Ok(())
    }
}

const SELECT: &str =
    "SELECT a.id, a.blob_id, a.file_name, a.mime, b.size, b.encrypted, a.created_at
     FROM attachments a JOIN blobs b ON b.id = a.blob_id";

fn from_row(r: &Row) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: r.get(0)?,
        blob_id: r.get(1)?,
        file_name: r.get(2)?,
        mime: r.get(3)?,
        size: r.get::<_, i64>(4)? as u64,
        encrypted: r.get(5)?,
        created_at: r.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::vault::crypto::KdfParams;
    use crate::vault::secrets::{SecretInput, SecretPayload, SecretRepo};
    use crate::vault::templates::SecretKind;
    use std::io::Cursor;
    use std::sync::Arc;

    struct Fixture {
        dir: tempfile::TempDir,
        db: Db,
        clock: Arc<FakeClock>,
        vault: Vault,
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
            Self {
                dir: tempfile::tempdir().unwrap(),
                db,
                clock,
                vault,
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

        fn blobs(&self) -> BlobStore<'_> {
            BlobStore::new(self.dir.path(), self.db.conn(), self.clock.as_ref())
        }

        fn client(&self) -> Id {
            let input = ClientInput {
                kind: ClientKind::Company,
                last_name: None,
                first_name: None,
                middle_name: None,
                company_name: Some("Ромашка".into()),
                legal_name: None,
                status: ClientStatus::Active,
                default_currency: None,
                default_rate_minor: None,
            };
            ClientRepo::new(self.db.conn(), self.clock.as_ref())
                .create(input)
                .unwrap()
                .id
        }

        fn secret(&self, client: Option<Id>) -> Id {
            let input = SecretInput {
                title: "Ключ".into(),
                kind: SecretKind::Custom,
                client_id: client,
                project_id: None,
                expires_on: None,
                payload: SecretPayload {
                    fields: Default::default(),
                    custom: vec![crate::vault::secrets::CustomEntry {
                        key: "k".into(),
                        value: "v".into(),
                        secret: true,
                    }],
                },
            };
            SecretRepo::new(self.db.conn(), self.clock.as_ref(), &self.vault)
                .create(input)
                .unwrap()
                .id
        }

        fn stored_files(&self) -> usize {
            fn count(dir: &Path) -> usize {
                std::fs::read_dir(dir)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|e| {
                        if e.path().is_dir() {
                            count(&e.path())
                        } else {
                            1
                        }
                    })
                    .sum()
            }
            count(&self.dir.path().join("blobs"))
        }
    }

    fn attach(f: &Fixture, owner: EntityRef, name: &str, data: &[u8]) -> Attachment {
        f.files()
            .attach(owner, name, Cursor::new(data.to_vec()), false)
            .unwrap()
    }

    #[test]
    fn attaches_to_clients_with_metadata() {
        let f = Fixture::new();
        let owner = EntityRef::Client(f.client());
        let a = attach(&f, owner, "Договор.pdf", b"%PDF-1.7");
        assert_eq!(a.file_name, "Договор.pdf");
        assert_eq!(a.mime, "application/pdf");
        assert_eq!(a.size, 8);
        assert!(!a.encrypted);
        assert_eq!(f.files().list(owner).unwrap(), std::slice::from_ref(&a));
        assert_eq!(f.files().get(a.id).unwrap(), a);
    }

    #[test]
    fn attaches_to_every_owner_type() {
        use crate::contacts::{ContactInput, ContactRepo};
        use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};
        use crate::tasks::{Priority, TaskInput, TaskRepo};
        let f = Fixture::new();
        let client = f.client();
        let contact = ContactRepo::new(f.db.conn(), f.clock.as_ref())
            .create(
                client,
                ContactInput {
                    last_name: Some("Иванов".into()),
                    first_name: None,
                    middle_name: None,
                    position: None,
                    note: None,
                    is_primary: false,
                },
            )
            .unwrap()
            .id;
        let project = ProjectRepo::new(f.db.conn(), f.clock.as_ref())
            .create(ProjectInput {
                client_id: client,
                name: "Сайт".into(),
                description_md: String::new(),
                status: ProjectStatus::Active,
                billing_type: BillingType::Fixed,
                currency: Some("RUB".parse().unwrap()),
                rate_minor: None,
                budget_minor: Some(1),
                start_date: None,
                deadline: None,
            })
            .unwrap()
            .id;
        let task = TaskRepo::new(f.db.conn(), f.clock.as_ref())
            .create(TaskInput {
                title: "Макет".into(),
                description_md: String::new(),
                status_id: None,
                priority: Priority::Normal,
                project_id: None,
                client_id: None,
                due_date: None,
                due_time: None,
                estimate_minutes: None,
            })
            .unwrap()
            .id;
        let secret = f.secret(None);
        for owner in [
            EntityRef::Client(client),
            EntityRef::Contact(contact),
            EntityRef::Project(project),
            EntityRef::Task(task),
            EntityRef::Secret(secret),
        ] {
            attach(&f, owner, "a.txt", b"x");
            assert_eq!(f.files().list(owner).unwrap().len(), 1, "{owner:?}");
        }
        let missing = Id::new();
        assert_eq!(
            f.files()
                .attach(EntityRef::Task(missing), "a.txt", Cursor::new(b"x"), false)
                .unwrap_err(),
            CoreError::not_found("task", missing)
        );
    }

    #[test]
    fn file_names_are_required_and_trimmed() {
        let f = Fixture::new();
        let owner = EntityRef::Client(f.client());
        assert_eq!(
            f.files()
                .attach(owner, "  ", Cursor::new(b"x"), false)
                .unwrap_err(),
            CoreError::validation("fileName", "required")
        );
        assert_eq!(attach(&f, owner, "  a.txt ", b"x").file_name, "a.txt");
        // Unknown extensions fall back to a generic type.
        assert_eq!(
            attach(&f, owner, "notes", b"x").mime,
            "application/octet-stream"
        );
    }

    #[test]
    fn the_same_file_twice_shares_one_blob() {
        let f = Fixture::new();
        let owner = EntityRef::Client(f.client());
        let a = attach(&f, owner, "a.txt", b"same");
        let b = attach(&f, owner, "b.txt", b"same");
        assert_eq!(a.blob_id, b.blob_id);
        assert_eq!(f.stored_files(), 1);
        f.files().delete(a.id).unwrap();
        assert_eq!(f.stored_files(), 1);
        f.files().delete(b.id).unwrap();
        assert_eq!(f.stored_files(), 0);
        assert_eq!(
            f.files().delete(b.id).unwrap_err().code,
            "attachment.not_found"
        );
    }

    #[test]
    fn deleting_the_owner_frees_its_files() {
        let f = Fixture::new();
        let client = f.client();
        attach(&f, EntityRef::Client(client), "a.txt", b"one");
        let secret = f.secret(Some(client));
        attach(&f, EntityRef::Secret(secret), "key.pem", b"two");
        assert_eq!(f.stored_files(), 2);
        ClientRepo::new(f.db.conn(), f.clock.as_ref())
            .delete(client)
            .unwrap();
        let left: i64 =
            f.db.conn()
                .query_row("SELECT count(*) FROM attachments", [], |r| r.get(0))
                .unwrap();
        assert_eq!(left, 0);
        assert_eq!(f.blobs().collect_garbage().unwrap(), 2);
        assert_eq!(f.stored_files(), 0);
    }

    #[test]
    fn secret_files_are_always_encrypted() {
        let f = Fixture::new();
        let owner = EntityRef::Secret(f.secret(None));
        let a = attach(&f, owner, "id_ed25519", b"PRIVATE");
        assert!(a.encrypted);
        f.vault.lock();
        assert_eq!(
            f.files()
                .attach(owner, "x", Cursor::new(b"x"), false)
                .unwrap_err(),
            CoreError::new("vault.locked")
        );
    }

    #[test]
    fn other_files_are_encrypted_on_request() {
        let f = Fixture::new();
        let owner = EntityRef::Client(f.client());
        let a = f
            .files()
            .attach(owner, "passport.jpg", Cursor::new(b"JPEG"), true)
            .unwrap();
        assert!(a.encrypted);
        assert_eq!(a.mime, "image/jpeg");
    }

    #[test]
    fn attaches_files_from_disk() {
        let f = Fixture::new();
        let owner = EntityRef::Client(f.client());
        let source = f.dir.path().join("Смета.xlsx");
        std::fs::write(&source, b"PK").unwrap();
        let a = f.files().attach_path(owner, &source, false).unwrap();
        assert_eq!(a.file_name, "Смета.xlsx");
        assert_eq!(a.size, 2);
        assert_eq!(
            f.files()
                .attach_path(owner, &f.dir.path().join("missing.txt"), false)
                .unwrap_err(),
            CoreError::new("file.read_failed")
        );
    }

    #[test]
    fn size_limit_applies() {
        let f = Fixture::new();
        let owner = EntityRef::Client(f.client());
        let err = f
            .files()
            .with_max_size(3)
            .attach(owner, "big.bin", Cursor::new(b"four"), false)
            .unwrap_err();
        assert_eq!(err.code, "file.too_large");
        assert!(f.files().list(owner).unwrap().is_empty());
    }
}
