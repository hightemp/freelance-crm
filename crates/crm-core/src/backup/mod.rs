//! Backups (FR-BK-1, 4): one `.fcrm-backup` file holding a consistent
//! snapshot of the database (`VACUUM INTO`) and every stored file, zipped
//! with a manifest of their hashes and encrypted with a passphrase (age).
//! Secrets and encrypted files stay encrypted inside: restoring them
//! still needs the master password.

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use age::secrecy::SecretString;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::{Clock, Timestamp};
use crate::{CoreError, CoreResult};

/// The archive layout version; newer ones are refused.
pub const FORMAT: u32 = 1;
pub const EXTENSION: &str = "fcrm-backup";
const MANIFEST: &str = "manifest.json";
/// Where the database is inside the archive.
pub const DB_ENTRY: &str = "data/crm.db";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ManifestFile {
    /// Path inside the archive: the database or `files/blobs/ab/cd/<id>`.
    pub path: String,
    /// Hex SHA-256 of the content.
    pub sha256: String,
    #[specta(type = f64)]
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format: u32,
    pub app_version: String,
    /// Migrations applied to the database in the archive.
    pub schema_version: u32,
    pub created_at: Timestamp,
    pub files: Vec<ManifestFile>,
    /// Stored files the database lists but the disk no longer has.
    pub missing: Vec<String>,
}

/// The shortest backup passphrase, as for the vault.
pub const MIN_PASSPHRASE_LEN: usize = 8;

pub fn check_passphrase(passphrase: &str) -> CoreResult<()> {
    if passphrase.chars().count() < MIN_PASSPHRASE_LEN {
        return Err(
            CoreError::validation("passphrase", "min_length").with("min", MIN_PASSPHRASE_LEN)
        );
    }
    Ok(())
}

/// Writes a backup of the database on `conn` and the files under
/// `files_root` to `dest`, encrypted with `passphrase`.
pub fn export(
    conn: &Connection,
    files_root: &Path,
    dest: &Path,
    passphrase: &str,
    clock: &dyn Clock,
) -> CoreResult<Manifest> {
    export_with(conn, files_root, dest, passphrase, clock, None)
}

/// [`export`] with a given scrypt work factor (tests make it cheap).
pub(crate) fn export_with(
    conn: &Connection,
    files_root: &Path,
    dest: &Path,
    passphrase: &str,
    clock: &dyn Clock,
    log_n: Option<u8>,
) -> CoreResult<Manifest> {
    check_passphrase(passphrase)?;
    snapshot(conn, dest)?.seal_with(files_root, dest, passphrase, clock, log_n)
}

/// The database copied for a backup; sealing it needs no connection, so
/// the app keeps working meanwhile.
pub struct Snapshot {
    work: tempfile::TempDir,
    schema_version: u32,
    blobs: Vec<String>,
}

/// Takes a consistent copy of the database for a backup to `dest`.
pub fn snapshot(conn: &Connection, dest: &Path) -> CoreResult<Snapshot> {
    // Work next to the destination, so that the result is a rename away.
    let parent = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let work = tempfile::Builder::new()
        .prefix(".fcrm-backup-")
        .tempdir_in(parent)
        .map_err(write_failed)?;

    // A consistent snapshot, even while the app keeps writing.
    let snapshot = work.path().join("crm.db");
    conn.execute("VACUUM INTO ?1", [snapshot.to_string_lossy()])?;
    let (schema_version, blobs) = {
        let snap = Connection::open(&snapshot)?;
        let version: u32 = snap.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let mut stmt = snap.prepare("SELECT id FROM blobs ORDER BY id")?;
        let ids = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        (version, ids)
    };
    Ok(Snapshot {
        work,
        schema_version,
        blobs,
    })
}

impl Snapshot {
    /// Packs the snapshot and the files under `files_root` into `dest`,
    /// encrypted with `passphrase`.
    pub fn seal(
        self,
        files_root: &Path,
        dest: &Path,
        passphrase: &str,
        clock: &dyn Clock,
    ) -> CoreResult<Manifest> {
        self.seal_with(files_root, dest, passphrase, clock, None)
    }

    fn seal_with(
        self,
        files_root: &Path,
        dest: &Path,
        passphrase: &str,
        clock: &dyn Clock,
        log_n: Option<u8>,
    ) -> CoreResult<Manifest> {
        let Snapshot {
            work,
            schema_version,
            blobs,
        } = self;
        let snapshot = work.path().join("crm.db");
        let zip_path = work.path().join("backup.zip");
        let mut zip = zip::ZipWriter::new(BufWriter::new(
            File::create(&zip_path).map_err(write_failed)?,
        ));
        let deflated = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .large_file(true);
        // Stored files are mostly compressed already (images, PDFs) or encrypted.
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .large_file(true);
        let mut files = vec![add(&mut zip, DB_ENTRY, &snapshot, deflated)?];
        let mut missing = Vec::new();
        for id in blobs {
            let relative = blob_path(&id);
            let source = files_root.join(&relative);
            let entry = format!("files/{}", relative.to_string_lossy().replace('\\', "/"));
            if source.is_file() {
                files.push(add(&mut zip, &entry, &source, stored)?);
            } else {
                missing.push(entry);
            }
        }
        let manifest = Manifest {
            format: FORMAT,
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            schema_version,
            created_at: clock.now(),
            files,
            missing,
        };
        zip.start_file(MANIFEST, deflated).map_err(zip_failed)?;
        serde_json::to_writer_pretty(&mut zip, &manifest)
            .map_err(|_| write_failed(io::ErrorKind::Other.into()))?;
        zip.finish()
            .map_err(zip_failed)?
            .into_inner()
            .map_err(|e| write_failed(e.into_error()))?
            .sync_all()
            .map_err(write_failed)?;

        let mut recipient = age::scrypt::Recipient::new(SecretString::from(passphrase.to_owned()));
        if let Some(log_n) = log_n {
            recipient.set_work_factor(log_n);
        }
        let encryptor = age::Encryptor::with_recipients(std::iter::once(&recipient as _))
            .map_err(|_| CoreError::new("backup.encrypt_failed"))?;
        let sealed = work.path().join("backup.age");
        {
            let mut out = encryptor
                .wrap_output(BufWriter::new(File::create(&sealed).map_err(write_failed)?))
                .map_err(write_failed)?;
            io::copy(&mut File::open(&zip_path).map_err(write_failed)?, &mut out)
                .map_err(write_failed)?;
            out.finish()
                .map_err(write_failed)?
                .into_inner()
                .map_err(|e| write_failed(e.into_error()))?
                .sync_all()
                .map_err(write_failed)?;
        }
        std::fs::rename(&sealed, dest).map_err(write_failed)?;
        Ok(manifest)
    }
}

/// Decrypts and unpacks a backup into `into` (the database at
/// [`DB_ENTRY`], files under `files/`), checking every hash. On an error
/// `into` is left empty.
pub fn unpack(path: &Path, passphrase: &str, into: &Path) -> CoreResult<Manifest> {
    let result = unpack_into(path, passphrase, into);
    if result.is_err() {
        clear(into);
    }
    result
}

fn unpack_into(path: &Path, passphrase: &str, into: &Path) -> CoreResult<Manifest> {
    let input = BufReader::new(File::open(path).map_err(|_| CoreError::new("file.read_failed"))?);
    let decryptor = age::Decryptor::new_buffered(input).map_err(|_| corrupt())?;
    let identity = age::scrypt::Identity::new(SecretString::from(passphrase.to_owned()));
    let mut plain = decryptor
        .decrypt(std::iter::once(&identity as _))
        .map_err(|e| match e {
            age::DecryptError::NoMatchingKeys | age::DecryptError::DecryptionFailed => {
                CoreError::new("backup.wrong_password")
            }
            _ => corrupt(),
        })?;

    // The zip needs seeking: decrypt it to a temporary file first. A
    // damaged payload fails its authentication here.
    let mut zip_file = tempfile::tempfile_in(into).map_err(write_failed)?;
    io::copy(&mut plain, &mut zip_file).map_err(|_| corrupt())?;
    let mut zip = zip::ZipArchive::new(zip_file).map_err(|_| corrupt())?;

    let manifest: Manifest = {
        let entry = zip.by_name(MANIFEST).map_err(|_| corrupt())?;
        serde_json::from_reader(entry).map_err(|_| corrupt())?
    };
    if manifest.format > FORMAT {
        return Err(CoreError::new("backup.unsupported_format"));
    }
    for file in &manifest.files {
        // Only paths the app writes: no escaping `into`.
        let safe = file.path == DB_ENTRY
            || (file.path.starts_with("files/blobs/")
                && file
                    .path
                    .split('/')
                    .all(|part| !part.is_empty() && part != ".." && part != "."));
        if !safe {
            return Err(corrupt());
        }
        let target = into.join(&file.path);
        std::fs::create_dir_all(target.parent().expect("entries are in folders"))
            .map_err(write_failed)?;
        let entry = zip.by_name(&file.path).map_err(|_| corrupt())?;
        let mut out = BufWriter::new(File::create(&target).map_err(write_failed)?);
        let (sha256, size) = copy_hashing(entry, &mut out).map_err(|_| corrupt())?;
        out.flush().map_err(write_failed)?;
        if sha256 != file.sha256 || size != file.size {
            return Err(corrupt());
        }
    }
    Ok(manifest)
}

/// Adds a file to the archive and describes it for the manifest.
fn add<W: Write + io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    entry: &str,
    source: &Path,
    options: zip::write::SimpleFileOptions,
) -> CoreResult<ManifestFile> {
    zip.start_file(entry, options).map_err(zip_failed)?;
    let input = File::open(source).map_err(|_| CoreError::new("file.read_failed"))?;
    let (sha256, size) = copy_hashing(input, zip).map_err(write_failed)?;
    Ok(ManifestFile {
        path: entry.to_owned(),
        sha256,
        size,
    })
}

/// Copies and returns the hex SHA-256 and size of what was copied.
fn copy_hashing(mut from: impl Read, to: &mut impl Write) -> io::Result<(String, u64)> {
    let mut hasher = Sha256::new();
    let mut buf = vec![0; 64 * 1024];
    let mut size = 0u64;
    loop {
        let n = from.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        to.write_all(&buf[..n])?;
        size += n as u64;
    }
    Ok((hex(&hasher.finalize()), size))
}

/// Where the blob store keeps a blob, relative to its root.
fn blob_path(id: &str) -> PathBuf {
    let (a, b) = (id.get(0..2).unwrap_or("00"), id.get(2..4).unwrap_or("00"));
    Path::new("blobs").join(a).join(b).join(id)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Removes what an unpacking left in `dir`, keeping the folder.
fn clear(dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let _ = if path.is_dir() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };
        }
    }
}

fn corrupt() -> CoreError {
    CoreError::new("backup.corrupt")
}

fn write_failed(_: io::Error) -> CoreError {
    CoreError::new("file.write_failed")
}

fn zip_failed(_: zip::result::ZipError) -> CoreError {
    CoreError::new("file.write_failed")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::files::blobs::BlobStore;

    /// Cheap scrypt for tests.
    pub(crate) const LOG_N: Option<u8> = Some(2);

    pub(crate) fn clock() -> FakeClock {
        FakeClock::at(Timestamp::from_millis(1_773_000_000_000))
    }

    /// A database file with a client and a stored file, in `dir`.
    fn data(dir: &Path, clock: &FakeClock) -> Db {
        let db = Db::open(&dir.join("crm.db")).unwrap();
        crate::clients::ClientRepo::new(db.conn(), clock)
            .create(crate::clients::ClientInput {
                kind: crate::clients::ClientKind::Company,
                last_name: None,
                first_name: None,
                middle_name: None,
                company_name: Some("Ромашка".into()),
                legal_name: None,
                status: crate::clients::ClientStatus::Active,
                default_currency: None,
                default_rate_minor: None,
            })
            .unwrap();
        BlobStore::new(dir, db.conn(), clock)
            .put(&b"%PDF-1.7 the contract"[..], "application/pdf")
            .unwrap();
        db
    }

    #[test]
    fn writes_an_archive_only_the_passphrase_opens() {
        let dir = tempfile::tempdir().unwrap();
        let clock = clock();
        let db = data(dir.path(), &clock);
        let dest = dir.path().join("out").join("my.fcrm-backup");
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        let manifest =
            export_with(db.conn(), dir.path(), &dest, "correct horse", &clock, LOG_N).unwrap();

        // An age file, not a readable zip.
        let raw = std::fs::read(&dest).unwrap();
        assert!(raw.starts_with(b"age-encryption.org/v1"));
        let name = "Ромашка".as_bytes();
        assert!(!raw.windows(name.len()).any(|w| w == name));

        let wrong = tempfile::tempdir().unwrap();
        assert_eq!(
            unpack(&dest, "wrong horse", wrong.path()).unwrap_err(),
            CoreError::new("backup.wrong_password")
        );
        assert_eq!(std::fs::read_dir(wrong.path()).unwrap().count(), 0);

        let out = tempfile::tempdir().unwrap();
        assert_eq!(
            unpack(&dest, "correct horse", out.path()).unwrap(),
            manifest
        );
        let restored = Connection::open(out.path().join(DB_ENTRY)).unwrap();
        let name: String = restored
            .query_row("SELECT display_name FROM clients", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "Ромашка");
        let blob: String = restored
            .query_row("SELECT id FROM blobs", [], |r| r.get(0))
            .unwrap();
        let file = out
            .path()
            .join("files")
            .join("blobs")
            .join(&blob[..2])
            .join(&blob[2..4])
            .join(&blob);
        assert_eq!(std::fs::read(file).unwrap(), b"%PDF-1.7 the contract");
    }

    #[test]
    fn the_manifest_lists_the_schema_and_every_file_with_its_hash() {
        let dir = tempfile::tempdir().unwrap();
        let clock = clock();
        let db = data(dir.path(), &clock);
        let dest = dir.path().join("b.fcrm-backup");
        let m = export_with(db.conn(), dir.path(), &dest, "password1", &clock, LOG_N).unwrap();
        assert_eq!(m.format, FORMAT);
        assert_eq!(m.schema_version, crate::db::LATEST_VERSION);
        assert_eq!(m.created_at, clock.now());
        assert_eq!(m.app_version, env!("CARGO_PKG_VERSION"));
        assert!(m.missing.is_empty());
        let paths: Vec<_> = m.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths[0], DB_ENTRY);
        assert_eq!(paths.len(), 2);
        assert!(paths[1].starts_with("files/blobs/"));
        let content = b"%PDF-1.7 the contract";
        assert_eq!(m.files[1].size, content.len() as u64);
        assert_eq!(m.files[1].sha256, hex(&Sha256::digest(content)));
    }

    #[test]
    fn passphrases_are_long_enough() {
        assert_eq!(
            check_passphrase("1234567").unwrap_err(),
            CoreError::validation("passphrase", "min_length").with("min", MIN_PASSPHRASE_LEN)
        );
        assert!(check_passphrase("12345678").is_ok());
    }

    #[test]
    fn missing_files_are_listed_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let clock = clock();
        let db = data(dir.path(), &clock);
        std::fs::remove_dir_all(dir.path().join("blobs")).unwrap();
        let m = export_with(
            db.conn(),
            dir.path(),
            &dir.path().join("b.fcrm-backup"),
            "password1",
            &clock,
            LOG_N,
        )
        .unwrap();
        assert_eq!(m.files.len(), 1);
        assert_eq!(m.missing.len(), 1);
    }

    #[test]
    fn damaged_archives_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let clock = clock();
        let db = data(dir.path(), &clock);
        let dest = dir.path().join("b.fcrm-backup");
        export_with(db.conn(), dir.path(), &dest, "password1", &clock, LOG_N).unwrap();
        let mut raw = std::fs::read(&dest).unwrap();
        let last = raw.len() - 100;
        raw[last] ^= 0xff;
        std::fs::write(&dest, &raw).unwrap();
        let out = tempfile::tempdir().unwrap();
        assert_eq!(
            unpack(&dest, "password1", out.path()).unwrap_err(),
            CoreError::new("backup.corrupt")
        );
        std::fs::write(&dest, b"not a backup").unwrap();
        assert_eq!(
            unpack(&dest, "password1", out.path()).unwrap_err(),
            CoreError::new("backup.corrupt")
        );
    }

    #[test]
    fn a_backup_taken_while_writing_is_consistent() {
        let dir = tempfile::tempdir().unwrap();
        let clock = clock();
        let db = data(dir.path(), &clock);
        let path = dir.path().join("crm.db");
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let writer = {
            let stop = stop.clone();
            std::thread::spawn(move || {
                let db = Db::open(&path).unwrap();
                let clock = super::tests::clock();
                let repo = crate::clients::ClientRepo::new(db.conn(), &clock);
                let mut n = 0;
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    let tx = db.conn().unchecked_transaction().unwrap();
                    // Two clients per transaction: a snapshot sees both or none.
                    for _ in 0..2 {
                        repo.create(crate::clients::ClientInput {
                            kind: crate::clients::ClientKind::Company,
                            last_name: None,
                            first_name: None,
                            middle_name: None,
                            company_name: Some(format!("Клиент {n}")),
                            legal_name: None,
                            status: crate::clients::ClientStatus::Active,
                            default_currency: None,
                            default_rate_minor: None,
                        })
                        .unwrap();
                        n += 1;
                    }
                    tx.commit().unwrap();
                }
            })
        };
        std::thread::sleep(std::time::Duration::from_millis(50));
        let dest = dir.path().join("b.fcrm-backup");
        export_with(db.conn(), dir.path(), &dest, "password1", &clock, LOG_N).unwrap();
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        writer.join().unwrap();

        let out = tempfile::tempdir().unwrap();
        unpack(&dest, "password1", out.path()).unwrap();
        let restored = Connection::open(out.path().join(DB_ENTRY)).unwrap();
        let check: String = restored
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .unwrap();
        assert_eq!(check, "ok");
        let clients: i64 = restored
            .query_row("SELECT count(*) FROM clients", [], |r| r.get(0))
            .unwrap();
        // "Ромашка" plus whole pairs.
        assert_eq!(clients % 2, 1);
        let indexed: i64 = restored
            .query_row(
                "SELECT count(*) FROM search_docs WHERE kind = 'client'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(indexed, clients);
    }
}
