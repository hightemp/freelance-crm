//! Content-addressed file storage (FR-FI-2, 6, 7): files are streamed into
//! `<root>/blobs/ab/cd/<sha256>`, stored once however often they are
//! attached, and deleted when the last reference goes away.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::domain::{Clock, Timestamp};
use crate::vault::{Vault, stream};
use crate::{CoreError, CoreResult};

/// Default per-file limit (FR-FI-7).
pub const DEFAULT_MAX_SIZE: u64 = 500 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Blob {
    /// The SHA-256 of the content (hex) for plain blobs.
    pub id: String,
    pub sha256: String,
    #[specta(type = i32)]
    pub size: u64,
    pub mime: String,
    pub encrypted: bool,
    pub ref_count: u32,
    pub created_at: Timestamp,
}

fn write_failed(_: std::io::Error) -> CoreError {
    CoreError::new("file.write_failed")
}

/// Counts bytes read and fails once more than `max` were read.
struct Counted<R> {
    inner: R,
    size: u64,
    max: u64,
}

impl<R: Read> Read for Counted<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.size += n as u64;
        if self.size > self.max {
            return Err(std::io::Error::other("file too large"));
        }
        Ok(n)
    }
}

struct TempFile {
    path: PathBuf,
    sha256: String,
    size: u64,
}

pub struct BlobStore<'a> {
    root: &'a Path,
    conn: &'a Connection,
    clock: &'a dyn Clock,
    max_size: u64,
}

impl<'a> BlobStore<'a> {
    pub fn new(root: &'a Path, conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self {
            root,
            conn,
            clock,
            max_size: DEFAULT_MAX_SIZE,
        }
    }

    pub fn with_max_size(self, max_size: u64) -> Self {
        Self { max_size, ..self }
    }

    /// Stores the content (or reuses an identical blob) and takes one
    /// reference to it.
    pub fn put(&self, reader: impl Read, mime: &str) -> CoreResult<Blob> {
        let temp = self.copy_to_temp(reader)?;
        let id = temp.sha256.clone();
        let target = self.path(&id);
        let stored = (|| -> CoreResult<()> {
            if target.exists() {
                return Ok(());
            }
            std::fs::create_dir_all(target.parent().expect("blob path has a parent"))
                .map_err(write_failed)?;
            // Atomic within the data directory: readers never see a partial file.
            std::fs::rename(&temp.path, &target).map_err(write_failed)
        })();
        let _ = std::fs::remove_file(&temp.path);
        stored?;
        self.conn.execute(
            "INSERT INTO blobs (id, sha256, size, mime, encrypted, ref_count, created_at)
             VALUES (?1, ?1, ?2, ?3, 0, 1, ?4)
             ON CONFLICT (id) DO UPDATE SET ref_count = ref_count + 1",
            (&id, temp.size as i64, mime, self.clock.now()),
        )?;
        self.get(&id)
    }

    /// Streams the content into `<root>/tmp`, hashing it and enforcing the
    /// size limit. Nothing is left behind on failure.
    fn copy_to_temp(&self, mut reader: impl Read) -> CoreResult<TempFile> {
        use sha2::Digest;
        use std::io::Write;

        let dir = self.root.join("tmp");
        std::fs::create_dir_all(&dir).map_err(write_failed)?;
        let path = dir.join(format!("{}.part", crate::domain::Id::new()));
        let result = (|| -> CoreResult<TempFile> {
            let mut file = std::fs::File::create(&path).map_err(write_failed)?;
            let mut hasher = sha2::Sha256::new();
            let mut size: u64 = 0;
            let mut buf = vec![0u8; 64 * 1024];
            loop {
                let n = match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => return Err(CoreError::new("file.read_failed")),
                };
                size += n as u64;
                if size > self.max_size {
                    return Err(CoreError::new("file.too_large").with("max", self.max_size));
                }
                hasher.update(&buf[..n]);
                file.write_all(&buf[..n]).map_err(write_failed)?;
            }
            file.sync_all().map_err(write_failed)?;
            let sha256 = hasher
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            Ok(TempFile {
                path: path.clone(),
                sha256,
                size,
            })
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&path);
        }
        result
    }

    pub fn get(&self, id: &str) -> CoreResult<Blob> {
        self.conn
            .query_row(
                "SELECT id, sha256, size, mime, encrypted, ref_count, created_at FROM blobs
                 WHERE id = ?1",
                [id],
                |r| {
                    Ok(Blob {
                        id: r.get(0)?,
                        sha256: r.get(1)?,
                        size: r.get::<_, i64>(2)? as u64,
                        mime: r.get(3)?,
                        encrypted: r.get(4)?,
                        ref_count: r.get(5)?,
                        created_at: r.get(6)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| CoreError::not_found("blob", id))
    }

    /// Path of the stored file: `blobs/ab/cd/<id>`.
    pub fn path(&self, id: &str) -> PathBuf {
        let (a, b) = (id.get(0..2).unwrap_or("00"), id.get(2..4).unwrap_or("00"));
        self.root.join("blobs").join(a).join(b).join(id)
    }

    /// Stores the content encrypted with the vault's file key under a random
    /// id. Encrypted blobs are never deduplicated: equal ids would reveal
    /// equal contents.
    pub fn put_encrypted(&self, reader: impl Read, mime: &str, vault: &Vault) -> CoreResult<Blob> {
        let key = vault.with_dek(|dek| Ok(stream::file_key(dek)))?;
        let id = crate::domain::Id::new().to_string();
        let target = self.path(&id);
        let temp = self.root.join("tmp").join(format!("{id}.part"));
        let mut counted = Counted {
            inner: reader,
            size: 0,
            max: self.max_size,
        };
        let written = (|| -> CoreResult<()> {
            std::fs::create_dir_all(temp.parent().expect("temp has a parent"))
                .map_err(write_failed)?;
            let file = std::fs::File::create(&temp).map_err(write_failed)?;
            let mut out = std::io::BufWriter::new(file);
            stream::encrypt(&key, &mut counted, &mut out).map_err(|e| {
                if counted.size > counted.max {
                    CoreError::new("file.too_large").with("max", self.max_size)
                } else {
                    e
                }
            })?;
            out.into_inner()
                .map_err(|e| write_failed(e.into_error()))?
                .sync_all()
                .map_err(write_failed)?;
            std::fs::create_dir_all(target.parent().expect("blob path has a parent"))
                .map_err(write_failed)?;
            std::fs::rename(&temp, &target).map_err(write_failed)
        })();
        if written.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        written?;
        self.conn.execute(
            "INSERT INTO blobs (id, sha256, size, mime, encrypted, ref_count, created_at)
             VALUES (?1, '', ?2, ?3, 1, 1, ?4)",
            (&id, counted.size as i64, mime, self.clock.now()),
        )?;
        self.get(&id)
    }

    /// Decrypts an encrypted blob into `writer`; the vault must be unlocked.
    pub fn decrypt_to(&self, id: &str, vault: &Vault, writer: impl Write) -> CoreResult<()> {
        let blob = self.get(id)?;
        if !blob.encrypted {
            return Err(CoreError::new("blob.not_encrypted"));
        }
        let key = vault.with_dek(|dek| Ok(stream::file_key(dek)))?;
        let file =
            std::fs::File::open(self.path(id)).map_err(|_| CoreError::new("file.missing"))?;
        stream::decrypt(&key, std::io::BufReader::new(file), writer)
    }

    /// Drops one reference; the file is deleted with the last one.
    pub fn release(&self, id: &str) -> CoreResult<()> {
        let blob = self.get(id)?;
        if blob.ref_count > 1 {
            self.conn.execute(
                "UPDATE blobs SET ref_count = ref_count - 1 WHERE id = ?1",
                [id],
            )?;
            return Ok(());
        }
        self.conn.execute("DELETE FROM blobs WHERE id = ?1", [id])?;
        match std::fs::remove_file(self.path(id)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(write_failed(e)),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::domain::FakeClock;
    use std::io::Cursor;

    struct Fixture {
        dir: tempfile::TempDir,
        db: Db,
        clock: FakeClock,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                dir: tempfile::tempdir().unwrap(),
                db: Db::open_in_memory().unwrap(),
                clock: FakeClock::at(Timestamp::from_millis(1_700_000_000_000)),
            }
        }

        fn store(&self) -> BlobStore<'_> {
            BlobStore::new(self.dir.path(), self.db.conn(), &self.clock)
        }

        /// All regular files under the root, relative to it.
        fn files(&self) -> Vec<String> {
            fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
                for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        walk(&path, root, out);
                    } else {
                        out.push(
                            path.strip_prefix(root)
                                .unwrap()
                                .to_string_lossy()
                                .replace('\\', "/"),
                        );
                    }
                }
            }
            let mut out = Vec::new();
            walk(self.dir.path(), self.dir.path(), &mut out);
            out.sort();
            out
        }
    }

    const HELLO_SHA: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    #[test]
    fn stores_by_content_hash() {
        let f = Fixture::new();
        let blob = f.store().put(Cursor::new(b"hello"), "text/plain").unwrap();
        assert_eq!(blob.id, HELLO_SHA);
        assert_eq!(blob.sha256, HELLO_SHA);
        assert_eq!(blob.size, 5);
        assert_eq!(blob.mime, "text/plain");
        assert!(!blob.encrypted);
        assert_eq!(blob.ref_count, 1);
        assert_eq!(f.files(), [format!("blobs/2c/f2/{HELLO_SHA}")]);
        assert_eq!(std::fs::read(f.store().path(&blob.id)).unwrap(), b"hello");
        assert_eq!(f.store().get(&blob.id).unwrap(), blob);
    }

    #[test]
    fn identical_files_are_stored_once() {
        let f = Fixture::new();
        f.store().put(Cursor::new(b"hello"), "text/plain").unwrap();
        let again = f.store().put(Cursor::new(b"hello"), "text/plain").unwrap();
        assert_eq!(again.ref_count, 2);
        f.store().put(Cursor::new(b"other"), "text/plain").unwrap();
        assert_eq!(f.files().len(), 2);
    }

    #[test]
    fn the_last_release_deletes_the_file() {
        let f = Fixture::new();
        let blob = f.store().put(Cursor::new(b"hello"), "text/plain").unwrap();
        f.store().put(Cursor::new(b"hello"), "text/plain").unwrap();
        f.store().release(&blob.id).unwrap();
        assert_eq!(f.store().get(&blob.id).unwrap().ref_count, 1);
        assert_eq!(f.files().len(), 1);
        f.store().release(&blob.id).unwrap();
        assert!(f.files().is_empty());
        assert_eq!(
            f.store().get(&blob.id).unwrap_err(),
            CoreError::not_found("blob", HELLO_SHA)
        );
        assert_eq!(
            f.store().release(&blob.id).unwrap_err().code,
            "blob.not_found"
        );
    }

    #[test]
    fn files_over_the_limit_are_rejected_without_leftovers() {
        let f = Fixture::new();
        let err = f
            .store()
            .with_max_size(4)
            .put(Cursor::new(b"hello"), "text/plain")
            .unwrap_err();
        assert_eq!(err, CoreError::new("file.too_large").with("max", 4));
        assert!(f.files().is_empty());
        // Exactly at the limit is fine.
        assert!(
            f.store()
                .with_max_size(5)
                .put(Cursor::new(b"hello"), "text/plain")
                .is_ok()
        );
    }

    /// Fails after the first chunk, like a removed USB stick.
    struct Broken(usize);

    impl Read for Broken {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.0 == 0 {
                return Err(std::io::Error::other("device gone"));
            }
            self.0 -= 1;
            buf[..4].copy_from_slice(b"data");
            Ok(4)
        }
    }

    #[test]
    fn a_failed_copy_leaves_nothing_behind() {
        let f = Fixture::new();
        let err = f.store().put(Broken(3), "text/plain").unwrap_err();
        assert_eq!(err.code, "file.read_failed");
        assert!(f.files().is_empty());
        let rows: i64 =
            f.db.conn()
                .query_row("SELECT count(*) FROM blobs", [], |r| r.get(0))
                .unwrap();
        assert_eq!(rows, 0);
    }

    fn vault(f: &Fixture) -> Vault {
        let clock = std::sync::Arc::new(FakeClock::at(Timestamp::from_millis(1_700_000_000_000)));
        let vault = Vault::new(
            crate::vault::crypto::KdfParams {
                m_kib: 64,
                t: 1,
                p: 1,
            },
            clock,
        );
        vault.init(f.db.conn(), "master password").unwrap();
        vault
    }

    #[test]
    fn encrypted_blobs_round_trip_without_plaintext_on_disk() {
        let f = Fixture::new();
        let vault = vault(&f);
        let marker = b"PLAINTEXT_MARKER_42".repeat(5_000);
        let blob = f
            .store()
            .put_encrypted(Cursor::new(&marker), "text/plain", &vault)
            .unwrap();
        assert!(blob.encrypted);
        assert_eq!(blob.size, marker.len() as u64);
        assert_eq!(blob.sha256, "", "no plaintext hash for encrypted blobs");
        let on_disk = std::fs::read(f.store().path(&blob.id)).unwrap();
        assert!(!on_disk.windows(19).any(|w| w == b"PLAINTEXT_MARKER_42"));

        let mut out = Vec::new();
        f.store().decrypt_to(&blob.id, &vault, &mut out).unwrap();
        assert_eq!(out, marker);
    }

    #[test]
    fn equal_encrypted_files_are_not_deduplicated() {
        let f = Fixture::new();
        let vault = vault(&f);
        let a = f
            .store()
            .put_encrypted(Cursor::new(b"same"), "text/plain", &vault)
            .unwrap();
        let b = f
            .store()
            .put_encrypted(Cursor::new(b"same"), "text/plain", &vault)
            .unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(f.files().len(), 2);
        f.store().release(&a.id).unwrap();
        assert_eq!(f.files().len(), 1);
    }

    #[test]
    fn encrypted_blobs_need_an_unlocked_vault() {
        let f = Fixture::new();
        let vault = vault(&f);
        let blob = f
            .store()
            .put_encrypted(Cursor::new(b"x"), "text/plain", &vault)
            .unwrap();
        vault.lock();
        let locked = CoreError::new("vault.locked");
        assert_eq!(
            f.store()
                .decrypt_to(&blob.id, &vault, Vec::new())
                .unwrap_err(),
            locked
        );
        assert_eq!(
            f.store()
                .put_encrypted(Cursor::new(b"y"), "text/plain", &vault)
                .unwrap_err(),
            locked
        );
        assert_eq!(f.files().len(), 1, "no leftovers from the refused write");
    }

    #[test]
    fn plain_blobs_cannot_be_decrypted() {
        let f = Fixture::new();
        let vault = vault(&f);
        let blob = f.store().put(Cursor::new(b"plain"), "text/plain").unwrap();
        assert_eq!(
            f.store()
                .decrypt_to(&blob.id, &vault, Vec::new())
                .unwrap_err(),
            CoreError::new("blob.not_encrypted")
        );
    }

    #[test]
    fn large_files_are_streamed() {
        let f = Fixture::new();
        let data = vec![7u8; 3 * 1024 * 1024 + 17];
        let blob = f
            .store()
            .put(Cursor::new(&data), "application/octet-stream")
            .unwrap();
        assert_eq!(blob.size, data.len() as u64);
        assert_eq!(std::fs::read(f.store().path(&blob.id)).unwrap(), data);
    }
}
