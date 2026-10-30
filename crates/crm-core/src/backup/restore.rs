//! Restoring a backup (FR-BK-3): the archive is unpacked and checked
//! aside, its database migrated if it is older, the current data backed
//! up, and only then replaced. Until that last step nothing changes.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use super::Manifest;
use crate::db::Db;
use crate::domain::Clock;
use crate::{CoreError, CoreResult};

/// Where safety backups taken before a restore go, inside the data folder.
pub const SAFETY_DIR: &str = "backups";

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Restored {
    pub manifest: Manifest,
    /// The backup of the data that was replaced, encrypted with the same
    /// passphrase as the restored one.
    pub safety_backup: PathBuf,
}

/// Replaces the data in `db` and under `data_dir` with the backup at
/// `path`.
pub fn restore(
    db: &mut Db,
    data_dir: &Path,
    path: &Path,
    passphrase: &str,
    clock: &dyn Clock,
) -> CoreResult<Restored> {
    restore_with(db, data_dir, path, passphrase, clock, None)
}

pub(crate) fn restore_with(
    db: &mut Db,
    data_dir: &Path,
    path: &Path,
    passphrase: &str,
    clock: &dyn Clock,
    log_n: Option<u8>,
) -> CoreResult<Restored> {
    let staging = tempfile::Builder::new()
        .prefix(".fcrm-restore-")
        .tempdir_in(data_dir)
        .map_err(|_| CoreError::new("file.write_failed"))?;
    let manifest = super::unpack(path, passphrase, staging.path())?;
    let staged = staging.path().join(super::DB_ENTRY);

    // Trust the database over the manifest about its schema.
    let version: u32 =
        Connection::open(&staged)?.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version > crate::db::LATEST_VERSION || manifest.schema_version > crate::db::LATEST_VERSION {
        return Err(CoreError::new("backup.newer_schema"));
    }
    // Migrates an older one; then it must be sound.
    {
        let migrated = Db::open(&staged).map_err(|_| CoreError::new("backup.corrupt"))?;
        let check: String = migrated
            .conn()
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if check != "ok" {
            return Err(CoreError::new("backup.corrupt"));
        }
        // Folds the write-ahead log back into the file.
        migrated
            .conn()
            .execute_batch("PRAGMA journal_mode = DELETE")?;
    }

    // The data about to be replaced, in case the user wants it back.
    let safety_dir = data_dir.join(SAFETY_DIR);
    std::fs::create_dir_all(&safety_dir).map_err(|_| CoreError::new("file.write_failed"))?;
    let stamp = jiff::Timestamp::from_millisecond(clock.now().as_millis())
        .map_err(|_| CoreError::new("internal.clock"))?
        .strftime("%Y%m%d-%H%M%S");
    let safety_backup = safety_dir.join(format!("before-restore-{stamp}.{}", super::EXTENSION));
    super::export_with(
        db.conn(),
        data_dir,
        &safety_backup,
        passphrase,
        clock,
        log_n,
    )?;

    // The database in place, through the open connection: a single
    // transaction on it, so a failure leaves it as it was.
    {
        let source = Connection::open(&staged)?;
        let copy = rusqlite::backup::Backup::new(&source, db.conn_mut())?;
        copy.run_to_completion(1024, std::time::Duration::ZERO, None)?;
    }
    // Then the files: the old folder goes with the staging one.
    let blobs = data_dir.join("blobs");
    let restored = staging.path().join("files").join("blobs");
    if blobs.exists() {
        std::fs::rename(&blobs, staging.path().join("old-blobs"))
            .map_err(|_| CoreError::new("file.write_failed"))?;
    }
    if restored.exists() {
        std::fs::rename(&restored, &blobs).map_err(|_| CoreError::new("file.write_failed"))?;
    } else {
        std::fs::create_dir_all(&blobs).map_err(|_| CoreError::new("file.write_failed"))?;
    }
    Ok(Restored {
        manifest,
        safety_backup,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::export_with;
    use crate::backup::tests::{LOG_N, clock};
    use crate::domain::{EntityRef, FakeClock, Id, Money, Timestamp};
    use crate::vault::Vault;
    use crate::vault::crypto::KdfParams;
    use std::sync::Arc;

    const FAST: KdfParams = KdfParams {
        m_kib: 64,
        t: 1,
        p: 1,
    };
    const PASS: &str = "backup password";

    /// Something of every kind: clients and contacts, projects, tasks with
    /// tags, events with reminders, notes, fields, time, an invoice with a
    /// payment, an expense, taxes, files and a secret.
    fn sample(db: &Db, dir: &Path, clock: &FakeClock) {
        use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
        let conn = db.conn();
        let vault = Vault::new(FAST, Arc::new(FakeClock::at(clock.now())));
        vault.init(conn, "master password").unwrap();
        let client = ClientRepo::new(conn, clock)
            .create(ClientInput {
                kind: ClientKind::Company,
                last_name: None,
                first_name: None,
                middle_name: None,
                company_name: Some("ООО Ромашка".into()),
                legal_name: Some("Общество «Ромашка»".into()),
                status: ClientStatus::Active,
                default_currency: Some("RUB".parse().unwrap()),
                default_rate_minor: Some(150_000),
            })
            .unwrap()
            .id;
        crate::contacts::ContactRepo::new(conn, clock)
            .create(
                client,
                crate::contacts::ContactInput {
                    last_name: Some("Петров".into()),
                    first_name: Some("Иван".into()),
                    middle_name: None,
                    position: Some("Директор".into()),
                    note: None,
                    is_primary: true,
                },
            )
            .unwrap();
        let project = crate::projects::ProjectRepo::new(conn, clock)
            .create(crate::projects::ProjectInput {
                client_id: client,
                name: "Сайт".into(),
                description_md: "Лендинг".into(),
                status: crate::projects::ProjectStatus::Active,
                billing_type: crate::projects::BillingType::Hourly,
                currency: Some("RUB".parse().unwrap()),
                rate_minor: Some(200_000),
                budget_minor: None,
                start_date: Some("2026-03-01".into()),
                deadline: Some("2026-04-01".into()),
            })
            .unwrap()
            .id;
        let task = crate::tasks::TaskRepo::new(conn, clock)
            .create(crate::tasks::TaskInput {
                title: "Макет".into(),
                description_md: "Главная и каталог".into(),
                status_id: None,
                priority: crate::tasks::Priority::High,
                project_id: Some(project),
                client_id: None,
                due_date: Some("2026-03-12".into()),
                due_time: Some("18:00".into()),
                estimate_minutes: Some(240),
            })
            .unwrap()
            .id;
        let tags = crate::tags::TagService::new(conn, clock);
        let tag = tags
            .create(crate::tags::TagInput {
                name: "Срочно".into(),
                color: "#ff0000".into(),
            })
            .unwrap()
            .id;
        tags.attach(tag, EntityRef::Task(task)).unwrap();
        let event = crate::calendar::events::EventRepo::new(conn, clock)
            .create(crate::calendar::events::EventInput {
                title: "Планёрка".into(),
                description_md: String::new(),
                location: Some("Zoom".into()),
                all_day: false,
                start: "2026-03-10T10:00".into(),
                end: "2026-03-10T10:30".into(),
                tzid: "Europe/Moscow".into(),
                client_id: Some(client),
                project_id: None,
                task_id: None,
                contact_id: None,
                rrule: Some("FREQ=WEEKLY;BYDAY=TU".into()),
            })
            .unwrap()
            .id;
        crate::calendar::reminders::ReminderRepo::new(conn, clock)
            .set(EntityRef::Event(event), &[15])
            .unwrap();
        crate::notes::NoteRepo::new(conn, clock)
            .create(
                EntityRef::Client(client),
                crate::notes::NoteInput {
                    body_md: "Скидка 10 %".into(),
                    pinned: true,
                },
            )
            .unwrap();
        crate::custom_fields::CustomFieldRepo::new(conn, clock)
            .create(
                EntityRef::Project(project),
                crate::custom_fields::CustomFieldInput {
                    key: "Репозиторий".into(),
                    value: "https://example.com/site".into(),
                    field_type: crate::custom_fields::FieldType::Url,
                },
            )
            .unwrap();
        crate::time_entries::TimeRepo::new(conn, clock)
            .create(crate::time_entries::TimeEntryInput {
                target: crate::time_entries::TimeTarget {
                    task_id: Some(task),
                    project_id: None,
                    client_id: None,
                },
                description: "Вёрстка".into(),
                started_at: Timestamp::from_millis(1_773_100_000_000),
                ended_at: Timestamp::from_millis(1_773_107_200_000),
                billable: true,
            })
            .unwrap();
        let rub = |minor| Money::new(minor, "RUB".parse().unwrap());
        let invoice = crate::finance::invoices::InvoiceRepo::new(conn, clock)
            .create(crate::finance::invoices::InvoiceInput {
                number: String::new(),
                client_id: client,
                project_id: Some(project),
                currency: "RUB".parse().unwrap(),
                issue_date: "2026-03-10".into(),
                due_date: Some("2026-03-20".into()),
                status: crate::finance::invoices::StoredStatus::Sent,
                items: vec![crate::finance::invoices::InvoiceItemInput {
                    description: "Вёрстка".into(),
                    quantity_milli: 2000,
                    unit: "ч".into(),
                    price: rub(200_000),
                }],
                discount: crate::finance::invoices::Discount::Percent(1000),
                tax_bp: 0,
                notes_md: String::new(),
                language: "ru".into(),
                show_signature: None,
            })
            .unwrap()
            .id;
        crate::finance::payments::PaymentRepo::new(conn, clock)
            .create(crate::finance::payments::PaymentInput {
                invoice_id: Some(invoice),
                project_id: None,
                amount: rub(100_000),
                paid_on: "2026-03-15".into(),
                method: "Карта".into(),
                note: String::new(),
            })
            .unwrap();
        crate::finance::expenses::ExpenseRepo::new(conn, clock)
            .create(crate::finance::expenses::ExpenseInput {
                project_id: Some(project),
                category: "Софт".into(),
                amount: rub(99_000),
                spent_on: "2026-03-05".into(),
                description: "Figma".into(),
            })
            .unwrap();
        crate::finance::taxes::TaxRepo::new(conn, clock)
            .save(
                None,
                crate::finance::taxes::preset("npd", "2026-01-01").unwrap(),
            )
            .unwrap();
        let files = crate::files::attachments::Attachments::new(dir, conn, clock, &vault);
        files
            .attach(
                EntityRef::Client(client),
                "договор.pdf",
                &b"%PDF-1.7"[..],
                false,
            )
            .unwrap();
        files
            .attach(
                EntityRef::Project(project),
                "доступы.txt",
                &b"secret"[..],
                true,
            )
            .unwrap();
        crate::vault::secrets::SecretRepo::new(conn, clock, &vault)
            .create(crate::vault::secrets::SecretInput {
                title: "Хостинг".into(),
                kind: crate::vault::templates::SecretKind::Login,
                client_id: Some(client),
                project_id: None,
                expires_on: Some("2026-06-01".into()),
                payload: crate::vault::secrets::SecretPayload {
                    fields: [("password".to_owned(), "hunter2".to_owned())]
                        .into_iter()
                        .collect(),
                    custom: vec![],
                },
            })
            .unwrap();
    }

    /// Every row of every table, as text, by table.
    fn dump(conn: &Connection) -> Vec<(String, Vec<String>)> {
        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table'
                   AND name NOT LIKE 'sqlite_%' AND sql NOT LIKE 'CREATE VIRTUAL%'
                 ORDER BY name",
            )
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        tables
            .into_iter()
            .map(|table| {
                let mut stmt = conn.prepare(&format!("SELECT * FROM \"{table}\"")).unwrap();
                let n = stmt.column_count();
                let mut rows: Vec<String> = stmt
                    .query_map([], |r| {
                        Ok((0..n)
                            .map(|i| format!("{:?}", r.get_ref(i).unwrap()))
                            .collect::<Vec<_>>()
                            .join("|"))
                    })
                    .unwrap()
                    .collect::<Result<_, _>>()
                    .unwrap();
                rows.sort();
                (table, rows)
            })
            .collect()
    }

    /// Files under `dir/blobs`, with their content.
    fn files(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(root, &p, out);
                } else {
                    out.push((
                        p.strip_prefix(root).unwrap().into(),
                        std::fs::read(&p).unwrap(),
                    ));
                }
            }
        }
        let mut out = Vec::new();
        walk(dir, &dir.join("blobs"), &mut out);
        out.sort();
        out
    }

    /// A data folder with the sample data, and its backup.
    fn backed_up() -> (tempfile::TempDir, Db, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let clock = clock();
        let db = Db::open(&dir.path().join("crm.db")).unwrap();
        sample(&db, dir.path(), &clock);
        let backup = dir.path().join("all.fcrm-backup");
        export_with(db.conn(), dir.path(), &backup, PASS, &clock, LOG_N).unwrap();
        (dir, db, backup)
    }

    #[test]
    fn restoring_into_an_empty_folder_gives_identical_data() {
        let (source_dir, source, backup) = backed_up();
        let target = tempfile::tempdir().unwrap();
        let mut db = Db::open(&target.path().join("crm.db")).unwrap();
        let clock = clock();
        let restored = restore_with(&mut db, target.path(), &backup, PASS, &clock, LOG_N).unwrap();
        assert_eq!(restored.manifest.schema_version, crate::db::LATEST_VERSION);
        assert_eq!(dump(db.conn()), dump(source.conn()));
        assert_eq!(files(target.path()), files(source_dir.path()));
        // The restored data works: search and the secret's title.
        let hits = crate::search::search(db.conn(), "ромашка", 5).unwrap();
        assert!(!hits.is_empty());
    }

    #[test]
    fn the_current_data_is_backed_up_first() {
        let (_source_dir, _source, backup) = backed_up();
        let target = tempfile::tempdir().unwrap();
        let clock = clock();
        let mut db = Db::open(&target.path().join("crm.db")).unwrap();
        crate::clients::ClientRepo::new(db.conn(), &clock)
            .create(crate::clients::ClientInput {
                kind: crate::clients::ClientKind::Person,
                last_name: Some("Старый".into()),
                first_name: None,
                middle_name: None,
                company_name: None,
                legal_name: None,
                status: crate::clients::ClientStatus::Active,
                default_currency: None,
                default_rate_minor: None,
            })
            .unwrap();
        let before = dump(db.conn());
        let restored = restore_with(&mut db, target.path(), &backup, PASS, &clock, LOG_N).unwrap();
        assert!(
            restored
                .safety_backup
                .starts_with(target.path().join(SAFETY_DIR))
        );
        let out = tempfile::tempdir().unwrap();
        crate::backup::unpack(&restored.safety_backup, PASS, out.path()).unwrap();
        let old = Connection::open(out.path().join(crate::backup::DB_ENTRY)).unwrap();
        assert_eq!(dump(&old), before);
    }

    #[test]
    fn a_wrong_password_or_a_damaged_file_changes_nothing() {
        let (_source_dir, _source, backup) = backed_up();
        let target = tempfile::tempdir().unwrap();
        let clock = clock();
        let mut db = Db::open(&target.path().join("crm.db")).unwrap();
        sample(&db, target.path(), &clock);
        let (data, stored) = (dump(db.conn()), files(target.path()));
        let listing = || {
            let mut names: Vec<_> = std::fs::read_dir(target.path())
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect();
            names.sort();
            names
        };
        let entries = listing();

        assert_eq!(
            restore_with(&mut db, target.path(), &backup, "not it", &clock, LOG_N).unwrap_err(),
            CoreError::new("backup.wrong_password")
        );
        let mut raw = std::fs::read(&backup).unwrap();
        let at = raw.len() / 2;
        raw[at] ^= 0xff;
        std::fs::write(&backup, raw).unwrap();
        assert_eq!(
            restore_with(&mut db, target.path(), &backup, PASS, &clock, LOG_N).unwrap_err(),
            CoreError::new("backup.corrupt")
        );
        assert_eq!(dump(db.conn()), data);
        assert_eq!(files(target.path()), stored);
        // No safety backup and no leftovers either.
        assert_eq!(listing(), entries);
        assert!(!target.path().join(SAFETY_DIR).exists());
    }

    #[test]
    fn backups_of_a_newer_schema_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let clock = clock();
        let newer = Db::open(&dir.path().join("newer.db")).unwrap();
        newer
            .conn()
            .pragma_update(None, "user_version", crate::db::LATEST_VERSION + 1)
            .unwrap();
        let backup = dir.path().join("newer.fcrm-backup");
        export_with(newer.conn(), dir.path(), &backup, PASS, &clock, LOG_N).unwrap();

        let target = tempfile::tempdir().unwrap();
        let mut db = Db::open(&target.path().join("crm.db")).unwrap();
        assert_eq!(
            restore_with(&mut db, target.path(), &backup, PASS, &clock, LOG_N).unwrap_err(),
            CoreError::new("backup.newer_schema")
        );
    }

    #[test]
    fn backups_of_an_older_schema_are_migrated() {
        let dir = tempfile::tempdir().unwrap();
        let clock = clock();
        // Before global search (migration 34).
        let mut old = Connection::open(dir.path().join("old.db")).unwrap();
        crate::db::migrations().to_version(&mut old, 33).unwrap();
        old.execute(
            "INSERT INTO clients (id, kind, company_name, display_name, sort_key, status,
                 created_at, updated_at)
             VALUES (?1, 'company', 'Ромашка', 'Ромашка', 'ромашка', 'active', 1, 1)",
            [Id::new()],
        )
        .unwrap();
        let backup = dir.path().join("old.fcrm-backup");
        let m = export_with(&old, dir.path(), &backup, PASS, &clock, LOG_N).unwrap();
        assert_eq!(m.schema_version, 33);

        let target = tempfile::tempdir().unwrap();
        let mut db = Db::open(&target.path().join("crm.db")).unwrap();
        restore_with(&mut db, target.path(), &backup, PASS, &clock, LOG_N).unwrap();
        assert_eq!(db.schema_version().unwrap(), crate::db::LATEST_VERSION);
        assert_eq!(
            crate::search::search(db.conn(), "ромашка", 5)
                .unwrap()
                .len(),
            1
        );
    }
}
