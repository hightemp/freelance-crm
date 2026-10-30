//! Export of the non-secret data (FR-BK-5), for use elsewhere: every
//! table as JSON, and CSV of clients, projects, tasks and time. Secrets
//! are left out entirely, with the notes, fields, tags and files attached
//! to them; stored files are listed, not included.

use std::path::Path;

use rusqlite::Connection;
use rusqlite::types::ValueRef;
use serde_json::{Map, Value, json};

use crate::domain::Clock;
use crate::{CoreError, CoreResult};

/// The layout version of the JSON export.
pub const FORMAT: u32 = 1;

/// Tables exported as JSON, with a condition leaving out what belongs to
/// secrets.
const TABLES: &[(&str, &str)] = &[
    ("clients", "1"),
    ("contacts", "1"),
    ("contact_methods", "1"),
    ("requisites", "1"),
    ("custom_fields", "1"),
    ("notes", "owner_type <> 'secret'"),
    ("tags", "1"),
    ("taggings", "owner_type <> 'secret'"),
    ("attachments", "owner_type <> 'secret'"),
    ("projects", "1"),
    ("task_statuses", "1"),
    ("tasks", "1"),
    ("checklist_items", "1"),
    ("events", "1"),
    ("reminders", "1"),
    ("time_entries", "1"),
    ("interactions", "1"),
    ("interaction_follow_ups", "1"),
    ("invoices", "1"),
    ("invoice_items", "1"),
    ("payments", "1"),
    ("expenses", "1"),
    ("tax_regimes", "1"),
];

/// Every exported table's rows as objects of their columns.
pub fn json(conn: &Connection, clock: &dyn Clock) -> CoreResult<Value> {
    let mut tables = Map::new();
    for (table, condition) in TABLES {
        let mut stmt = conn.prepare(&format!(
            "SELECT * FROM {table} WHERE {condition} ORDER BY 1"
        ))?;
        let columns: Vec<String> = stmt.column_names().into_iter().map(str::to_owned).collect();
        let rows = stmt
            .query_map([], |r| {
                let mut row = Map::new();
                for (i, column) in columns.iter().enumerate() {
                    row.insert(column.clone(), value(r.get_ref(i)?));
                }
                Ok(Value::Object(row))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        tables.insert((*table).to_owned(), Value::Array(rows));
    }
    let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    Ok(json!({
        "format": FORMAT,
        "schemaVersion": version,
        "exportedAt": iso(clock.now().as_millis())?,
        "tables": tables,
    }))
}

/// A column value as JSON; blobs as hex.
fn value(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(n) => json!(n),
        ValueRef::Real(x) => json!(x),
        ValueRef::Text(t) => Value::String(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Value::String(crate::backup::hex(b)),
    }
}

/// An instant as RFC 3339 in UTC, to the second.
fn iso(ms: i64) -> CoreResult<String> {
    Ok(jiff::Timestamp::from_millisecond(ms)
        .map_err(|_| CoreError::new("internal.clock"))?
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string())
}

/// RFC 4180 CSV of the header and the rows of a query; each row gives
/// its cells as text.
fn csv(
    conn: &Connection,
    header: &[&str],
    sql: &str,
    row: impl Fn(&rusqlite::Row<'_>) -> rusqlite::Result<Vec<String>>,
) -> CoreResult<String> {
    use crate::time_report::field;
    let mut out = header.join(",");
    out.push_str("\r\n");
    let mut stmt = conn.prepare(sql)?;
    for cells in stmt.query_map([], |r| row(r))? {
        out.push_str(
            &cells?
                .iter()
                .map(|c| field(c))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("\r\n");
    }
    Ok(out)
}

fn text(v: Option<String>) -> String {
    v.unwrap_or_default()
}

/// Minor units as a decimal of the currency (`1500.50`).
fn amount(minor: Option<i64>, currency: Option<crate::domain::Currency>) -> String {
    match (minor, currency) {
        (Some(m), Some(c)) => crate::time_report::decimal(m, c.minor_units()),
        (Some(m), None) => m.to_string(),
        _ => String::new(),
    }
}

fn instant(ms: Option<i64>) -> rusqlite::Result<String> {
    Ok(match ms {
        Some(ms) => iso(ms).unwrap_or_default(),
        None => String::new(),
    })
}

pub fn clients_csv(conn: &Connection) -> CoreResult<String> {
    csv(
        conn,
        &[
            "id",
            "name",
            "kind",
            "legal_name",
            "status",
            "currency",
            "rate",
            "created_at",
        ],
        "SELECT id, display_name, kind, legal_name, status, default_currency,
                default_rate_minor, created_at
         FROM clients ORDER BY sort_key",
        |r| {
            Ok(vec![
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                text(r.get(3)?),
                r.get(4)?,
                text(r.get(5)?),
                amount(r.get(6)?, r.get(5)?),
                instant(r.get(7)?)?,
            ])
        },
    )
}

pub fn projects_csv(conn: &Connection) -> CoreResult<String> {
    csv(
        conn,
        &[
            "id",
            "name",
            "client",
            "status",
            "billing_type",
            "currency",
            "rate",
            "budget",
            "start_date",
            "deadline",
        ],
        "SELECT p.id, p.name, c.display_name, p.status, p.billing_type, p.currency,
                p.rate_minor, p.budget_minor, p.start_date, p.deadline
         FROM projects p JOIN clients c ON c.id = p.client_id
         ORDER BY c.sort_key, p.sort_key",
        |r| {
            Ok(vec![
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                amount(r.get(6)?, r.get(5)?),
                amount(r.get(7)?, r.get(5)?),
                text(r.get(8)?),
                text(r.get(9)?),
            ])
        },
    )
}

pub fn tasks_csv(conn: &Connection) -> CoreResult<String> {
    csv(
        conn,
        &[
            "id",
            "title",
            "status",
            "priority",
            "project",
            "client",
            "due_date",
            "due_time",
            "completed_at",
        ],
        "SELECT t.id, t.title, coalesce(s.name, s.name_key, s.category), t.priority, p.name,
                coalesce(c.display_name, pc.display_name), t.due_date, t.due_time,
                t.completed_at
         FROM tasks t
         JOIN task_statuses s ON s.id = t.status_id
         LEFT JOIN projects p ON p.id = t.project_id
         LEFT JOIN clients c ON c.id = t.client_id
         LEFT JOIN clients pc ON pc.id = p.client_id
         ORDER BY t.created_at, t.id",
        |r| {
            Ok(vec![
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                text(r.get(4)?),
                text(r.get(5)?),
                text(r.get(6)?),
                text(r.get(7)?),
                instant(r.get(8)?)?,
            ])
        },
    )
}

pub fn time_csv(conn: &Connection) -> CoreResult<String> {
    csv(
        conn,
        &[
            "id",
            "started_at",
            "ended_at",
            "minutes",
            "description",
            "client",
            "project",
            "task",
            "billable",
            "invoiced",
        ],
        "SELECT e.id, e.started_at, e.ended_at, e.description,
                coalesce(c.display_name, pc.display_name), p.name, t.title, e.billable,
                e.invoice_id IS NOT NULL
         FROM time_entries e
         LEFT JOIN tasks t ON t.id = e.task_id
         LEFT JOIN projects p ON p.id = coalesce(e.project_id, t.project_id)
         LEFT JOIN clients c ON c.id = e.client_id
         LEFT JOIN clients pc ON pc.id = p.client_id
         ORDER BY e.started_at, e.id",
        |r| {
            let started: i64 = r.get(1)?;
            let ended: Option<i64> = r.get(2)?;
            Ok(vec![
                r.get(0)?,
                instant(Some(started))?,
                instant(ended)?,
                ended
                    .map(|e| ((e - started) / 60_000).to_string())
                    .unwrap_or_default(),
                r.get(3)?,
                text(r.get(4)?),
                text(r.get(5)?),
                text(r.get(6)?),
                r.get::<_, bool>(7)?.to_string(),
                r.get::<_, bool>(8)?.to_string(),
            ])
        },
    )
}

/// Writes `freelance-crm.json` and the CSV files into `dir`; returns their
/// names.
pub fn write_all(conn: &Connection, clock: &dyn Clock, dir: &Path) -> CoreResult<Vec<String>> {
    let files = [
        (
            "freelance-crm.json",
            serde_json::to_string_pretty(&json(conn, clock)?)
                .map_err(|_| CoreError::new("file.write_failed"))?,
        ),
        ("clients.csv", clients_csv(conn)?),
        ("projects.csv", projects_csv(conn)?),
        ("tasks.csv", tasks_csv(conn)?),
        ("time.csv", time_csv(conn)?),
    ];
    let mut names = Vec::new();
    for (name, content) in files {
        std::fs::write(dir.join(name), content).map_err(|_| CoreError::new("file.write_failed"))?;
        names.push(name.to_owned());
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::domain::{EntityRef, FakeClock, Timestamp};
    use crate::vault::Vault;
    use crate::vault::crypto::KdfParams;
    use std::sync::Arc;

    const MARKER: &str = "СЕКРЕТНЫЙ_МАРКЕР_42";

    struct Fixture {
        db: Db,
        clock: FakeClock,
    }

    /// "Ромашка" with project "Сайт", a task, an hour of time and a secret
    /// (with a note and a tag) that must not appear.
    fn fixture() -> Fixture {
        let db = Db::open_in_memory().unwrap();
        let clock = FakeClock::at(Timestamp::from_millis(1_773_000_000_000));
        let conn = db.conn();
        let client = crate::clients::ClientRepo::new(conn, &clock)
            .create(crate::clients::ClientInput {
                kind: crate::clients::ClientKind::Company,
                last_name: None,
                first_name: None,
                middle_name: None,
                company_name: Some("Ромашка, ООО".into()),
                legal_name: Some("ООО \"Ромашка\"".into()),
                status: crate::clients::ClientStatus::Active,
                default_currency: Some("RUB".parse().unwrap()),
                default_rate_minor: Some(150_050),
            })
            .unwrap()
            .id;
        let project = crate::projects::ProjectRepo::new(conn, &clock)
            .create(crate::projects::ProjectInput {
                client_id: client,
                name: "Сайт".into(),
                description_md: String::new(),
                status: crate::projects::ProjectStatus::Active,
                billing_type: crate::projects::BillingType::Hourly,
                currency: Some("RUB".parse().unwrap()),
                rate_minor: Some(200_000),
                budget_minor: None,
                start_date: Some("2026-03-01".into()),
                deadline: None,
            })
            .unwrap()
            .id;
        let task = crate::tasks::TaskRepo::new(conn, &clock)
            .create(crate::tasks::TaskInput {
                title: "Макет\nглавной".into(),
                description_md: String::new(),
                status_id: None,
                priority: crate::tasks::Priority::High,
                project_id: Some(project),
                client_id: None,
                due_date: Some("2026-03-12".into()),
                due_time: Some("18:00".into()),
                estimate_minutes: None,
            })
            .unwrap()
            .id;
        crate::time_entries::TimeRepo::new(conn, &clock)
            .create(crate::time_entries::TimeEntryInput {
                target: crate::time_entries::TimeTarget {
                    task_id: Some(task),
                    project_id: None,
                    client_id: None,
                },
                description: "Вёрстка".into(),
                started_at: Timestamp::from_millis(1_773_100_000_000),
                ended_at: Timestamp::from_millis(1_773_105_400_000),
                billable: true,
            })
            .unwrap();
        let vault = Vault::new(
            KdfParams {
                m_kib: 64,
                t: 1,
                p: 1,
            },
            Arc::new(FakeClock::at(Timestamp::from_millis(1_773_000_000_000))),
        );
        vault.init(conn, "master password").unwrap();
        let secret = crate::vault::secrets::SecretRepo::new(conn, &clock, &vault)
            .create(crate::vault::secrets::SecretInput {
                title: MARKER.into(),
                kind: crate::vault::templates::SecretKind::Login,
                client_id: Some(client),
                project_id: None,
                expires_on: None,
                payload: crate::vault::secrets::SecretPayload {
                    fields: [("password".to_owned(), MARKER.to_owned())]
                        .into_iter()
                        .collect(),
                    custom: vec![],
                },
            })
            .unwrap()
            .id;
        crate::notes::NoteRepo::new(conn, &clock)
            .create(
                EntityRef::Secret(secret),
                crate::notes::NoteInput {
                    body_md: MARKER.into(),
                    pinned: false,
                },
            )
            .unwrap();
        Fixture { db, clock }
    }

    #[test]
    fn json_lists_every_table_but_secrets() {
        let f = fixture();
        let data = json(f.db.conn(), &f.clock).unwrap();
        assert_eq!(data["format"], FORMAT);
        assert_eq!(data["schemaVersion"], crate::db::LATEST_VERSION);
        assert_eq!(data["exportedAt"], "2026-03-08T20:00:00Z");
        let tables: Vec<&str> = data["tables"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let mut expected: Vec<&str> = TABLES.iter().map(|(t, _)| *t).collect();
        expected.sort();
        assert_eq!(tables, expected);
        let client = &data["tables"]["clients"][0];
        assert_eq!(client["display_name"], "Ромашка, ООО");
        assert_eq!(client["default_rate_minor"], 150_050);
        assert_eq!(client["archived_at"], Value::Null);
        assert_eq!(data["tables"]["notes"].as_array().unwrap().len(), 0);
        assert!(!data.to_string().contains(MARKER));
    }

    #[test]
    fn csv_of_clients_projects_tasks_and_time() {
        let f = fixture();
        let conn = f.db.conn();
        let rows = |csv: String| {
            csv.split("\r\n")
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };

        let clients = rows(clients_csv(conn).unwrap());
        assert_eq!(
            clients[0],
            "id,name,kind,legal_name,status,currency,rate,created_at"
        );
        assert!(clients[1].ends_with(
            ",\"Ромашка, ООО\",company,\"ООО \"\"Ромашка\"\"\",active,RUB,1500.50,2026-03-08T20:00:00Z"
        ));

        let projects = rows(projects_csv(conn).unwrap());
        assert_eq!(
            projects[0],
            "id,name,client,status,billing_type,currency,rate,budget,start_date,deadline"
        );
        assert!(
            projects[1].ends_with(",Сайт,\"Ромашка, ООО\",active,hourly,RUB,2000.00,,2026-03-01,")
        );

        let tasks = csv_records(&tasks_csv(conn).unwrap());
        assert_eq!(
            tasks[0],
            [
                "id",
                "title",
                "status",
                "priority",
                "project",
                "client",
                "due_date",
                "due_time",
                "completed_at"
            ]
        );
        assert_eq!(tasks[1][1], "Макет\nглавной");
        assert_eq!(
            tasks[1][3..8],
            ["high", "Сайт", "Ромашка, ООО", "2026-03-12", "18:00"]
        );

        let time = csv_records(&time_csv(conn).unwrap());
        assert_eq!(
            time[0],
            [
                "id",
                "started_at",
                "ended_at",
                "minutes",
                "description",
                "client",
                "project",
                "task",
                "billable",
                "invoiced"
            ]
        );
        assert_eq!(
            time[1][1..10],
            [
                "2026-03-09T23:46:40Z",
                "2026-03-10T01:16:40Z",
                "90",
                "Вёрстка",
                "Ромашка, ООО",
                "Сайт",
                "Макет\nглавной",
                "true",
                "false"
            ]
        );
    }

    #[test]
    fn writes_the_files_into_a_folder() {
        let f = fixture();
        let dir = tempfile::tempdir().unwrap();
        let names = write_all(f.db.conn(), &f.clock, dir.path()).unwrap();
        assert_eq!(
            names,
            [
                "freelance-crm.json",
                "clients.csv",
                "projects.csv",
                "tasks.csv",
                "time.csv"
            ]
        );
        for name in &names {
            let text = std::fs::read_to_string(dir.path().join(name)).unwrap();
            assert!(!text.contains(MARKER), "{name}");
        }
        let data: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.path().join(&names[0])).unwrap())
                .unwrap();
        assert_eq!(data["format"], FORMAT);
    }

    /// Records of an RFC 4180 text, for checking fields with line breaks.
    fn csv_records(text: &str) -> Vec<Vec<String>> {
        let mut records = Vec::new();
        let (mut record, mut field, mut quoted) = (Vec::new(), String::new(), false);
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match (c, quoted) {
                ('"', true) if chars.peek() == Some(&'"') => {
                    field.push('"');
                    chars.next();
                }
                ('"', _) => quoted = !quoted,
                (',', false) => record.push(std::mem::take(&mut field)),
                ('\r', false) => {}
                ('\n', false) => {
                    record.push(std::mem::take(&mut field));
                    records.push(std::mem::take(&mut record));
                }
                _ => field.push(c),
            }
        }
        records
    }
}
