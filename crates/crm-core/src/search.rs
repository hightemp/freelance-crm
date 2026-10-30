//! Global search (FR-SR-1, 2): one FTS5 index over clients, contacts,
//! projects, tasks, events, notes, interactions, custom fields, file names,
//! invoices and the titles of secrets (never their values). Triggers keep
//! it current (migration 34); `unicode61` makes it case-insensitive for
//! Cyrillic too, and "ё" is indexed and looked for as "е".

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::CoreResult;
use crate::domain::{EntityRef, Id};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SearchKind {
    Client,
    Contact,
    Project,
    Task,
    Event,
    Note,
    Interaction,
    Field,
    File,
    Secret,
    Invoice,
}

crate::db::text_enum!(SearchKind {
    Client => "client",
    Contact => "contact",
    Project => "project",
    Task => "task",
    Event => "event",
    Note => "note",
    Interaction => "interaction",
    Field => "field",
    File => "file",
    Secret => "secret",
    Invoice => "invoice",
});

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub kind: SearchKind,
    pub id: Id,
    /// Where the hit is shown: the record a note, field or file belongs
    /// to, the client of a contact or an interaction.
    pub owner: Option<EntityRef>,
    pub title: String,
    /// The matching part of the text with the match in `[` `]`, if the
    /// text is not just the title.
    pub snippet: Option<String>,
}

/// The FTS5 query for what the user typed: every word as a prefix, all
/// of them required. `None` when there is nothing to look for.
fn fts_query(input: &str) -> Option<String> {
    let words: Vec<String> = input
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| format!("\"{}\"*", w.replace('ё', "е").replace('Ё', "Е")))
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

/// The best `limit` matches: exact titles first, then by relevance with
/// titles weighing more than the rest of the text.
pub fn search(conn: &Connection, query: &str, limit: u32) -> CoreResult<Vec<SearchHit>> {
    let Some(fts) = fts_query(query) else {
        return Ok(Vec::new());
    };
    // Rare characters mark the match; brackets may be in the text itself.
    let mut stmt = conn.prepare(
        "SELECT d.kind, d.id, d.owner_type, d.owner_id, d.title,
                snippet(search_index, 1, char(2), char(3), '…', 16)
         FROM search_index JOIN search_docs d ON d.rowid = search_index.rowid
         WHERE search_index MATCH ?1
         ORDER BY bm25(search_index, 10.0, 1.0)
         LIMIT ?2",
    )?;
    // A few more than asked, so that exact titles further down move up.
    let rows = stmt.query_map((&fts, limit.saturating_mul(3).max(30)), |r| {
        Ok((
            r.get::<_, SearchKind>(0)?,
            r.get::<_, Id>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<Id>>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
        ))
    })?;
    let wanted = crate::domain::text::fold(query.trim());
    let mut hits = Vec::new();
    for row in rows {
        let (kind, id, owner_type, owner_id, title, snippet) = row?;
        let owner = owner_type
            .zip(owner_id)
            .and_then(|(t, id)| EntityRef::from_parts(&t, id));
        let snippet = snippet
            .contains('\u{2}')
            .then(|| snippet.replace('\u{2}', "[").replace('\u{3}', "]"));
        hits.push(SearchHit {
            kind,
            id,
            owner,
            title,
            snippet,
        });
    }
    // Stable: relevance order stays among the rest.
    hits.sort_by_key(|h| crate::domain::text::fold(&h.title) != wanted);
    hits.truncate(limit as usize);
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::{FakeClock, Timestamp};
    use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};
    use crate::tasks::{Priority, TaskInput, TaskRepo};

    struct Fixture {
        db: Db,
        clock: FakeClock,
        client: Id,
    }

    impl Fixture {
        /// A client "ООО Ромашка".
        fn new() -> Self {
            let db = Db::open_in_memory().unwrap();
            let clock = FakeClock::at(Timestamp::from_millis(1_773_000_000_000));
            let client = ClientRepo::new(db.conn(), &clock)
                .create(ClientInput {
                    kind: ClientKind::Company,
                    last_name: None,
                    first_name: None,
                    middle_name: None,
                    company_name: Some("ООО Ромашка".into()),
                    legal_name: Some("Общество с ограниченной ответственностью".into()),
                    status: ClientStatus::Active,
                    default_currency: None,
                    default_rate_minor: None,
                })
                .unwrap()
                .id;
            Self { db, clock, client }
        }

        fn project(&self, name: &str, description: &str) -> Id {
            ProjectRepo::new(self.db.conn(), &self.clock)
                .create(ProjectInput {
                    client_id: self.client,
                    name: name.into(),
                    description_md: description.into(),
                    status: ProjectStatus::Active,
                    billing_type: BillingType::Fixed,
                    currency: None,
                    rate_minor: None,
                    budget_minor: Some(1),
                    start_date: None,
                    deadline: None,
                })
                .unwrap()
                .id
        }

        fn task(&self, title: &str, description: &str) -> Id {
            TaskRepo::new(self.db.conn(), &self.clock)
                .create(TaskInput {
                    title: title.into(),
                    description_md: description.into(),
                    status_id: None,
                    priority: Priority::Normal,
                    project_id: None,
                    client_id: Some(self.client),
                    due_date: None,
                    due_time: None,
                    estimate_minutes: None,
                })
                .unwrap()
                .id
        }

        fn hits(&self, query: &str) -> Vec<(SearchKind, String)> {
            search(self.db.conn(), query, 20)
                .unwrap()
                .into_iter()
                .map(|h| (h.kind, h.title))
                .collect()
        }
    }

    #[test]
    fn finds_cyrillic_regardless_of_case() {
        let f = Fixture::new();
        assert_eq!(
            f.hits("ооо ромашка"),
            [(SearchKind::Client, "ООО Ромашка".to_owned())]
        );
        assert_eq!(f.hits("ООО РОМАШКА").len(), 1);
        // The legal name is searched too.
        assert_eq!(f.hits("ограниченной").len(), 1);
    }

    #[test]
    fn finds_by_prefix_and_ignores_yo() {
        let f = Fixture::new();
        f.project("Сайт ромашки", "Лендинг");
        let kinds: Vec<_> = f.hits("ром").into_iter().map(|h| h.0).collect();
        assert_eq!(kinds.len(), 2);
        assert!(kinds.contains(&SearchKind::Project));
        assert_eq!(
            f.hits("ленд"),
            [(SearchKind::Project, "Сайт ромашки".to_owned())]
        );

        crate::calendar::events::EventRepo::new(f.db.conn(), &f.clock)
            .create(crate::calendar::events::EventInput {
                title: "Встреча с Ёлкиной".into(),
                description_md: String::new(),
                location: Some("Офис на Арбате".into()),
                all_day: false,
                start: "2026-03-10T10:00".into(),
                end: "2026-03-10T11:00".into(),
                tzid: "UTC".into(),
                client_id: None,
                project_id: None,
                task_id: None,
                contact_id: None,
                rrule: None,
            })
            .unwrap();
        assert_eq!(f.hits("елкиной").len(), 1);
        assert_eq!(f.hits("арбат").len(), 1);
    }

    #[test]
    fn updates_and_deletions_reach_the_index() {
        let f = Fixture::new();
        let task = f.task("Сверстать макет", "");
        assert_eq!(f.hits("макет").len(), 1);
        let tasks = TaskRepo::new(f.db.conn(), &f.clock);
        let mut input = TaskInput {
            title: "Нарисовать иконки".into(),
            description_md: String::new(),
            status_id: None,
            priority: Priority::Normal,
            project_id: None,
            client_id: Some(f.client),
            due_date: None,
            due_time: None,
            estimate_minutes: None,
        };
        tasks.update(task, input.clone()).unwrap();
        assert!(f.hits("макет").is_empty());
        assert_eq!(f.hits("иконки").len(), 1);
        input.title = "Иконки".into();
        tasks.delete(task).unwrap();
        assert!(f.hits("иконки").is_empty());

        // Deleting a client takes its contacts, notes and fields along.
        let contact = crate::contacts::ContactRepo::new(f.db.conn(), &f.clock)
            .create(
                f.client,
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
        crate::notes::NoteRepo::new(f.db.conn(), &f.clock)
            .create(
                EntityRef::Client(f.client),
                crate::notes::NoteInput {
                    body_md: "Позвонить в понедельник".into(),
                    pinned: false,
                },
            )
            .unwrap();
        assert_eq!(f.hits("директор").len(), 1);
        assert_eq!(f.hits("понедельник").len(), 1);
        let _ = contact;
        f.db.conn()
            .execute("DELETE FROM clients WHERE id = ?1", [f.client])
            .unwrap();
        assert!(f.hits("директор").is_empty());
        assert!(f.hits("понедельник").is_empty());
        assert!(f.hits("ромашка").is_empty());
    }

    #[test]
    fn exact_titles_and_title_matches_rank_first() {
        let f = Fixture::new();
        f.task("Обсудить смету", "Подготовить макет к пятнице");
        f.task("Макет", "");
        f.task("Новый макет главной", "");
        let titles: Vec<_> = f.hits("макет").into_iter().map(|h| h.1).collect();
        assert_eq!(titles[0], "Макет");
        assert_eq!(titles[2], "Обсудить смету", "a body match ranks last");
    }

    #[test]
    fn hits_lead_to_their_owner() {
        let f = Fixture::new();
        let conn = f.db.conn();
        crate::notes::NoteRepo::new(conn, &f.clock)
            .create(
                EntityRef::Client(f.client),
                crate::notes::NoteInput {
                    body_md: "Скидка 10 %\nдо конца года".into(),
                    pinned: false,
                },
            )
            .unwrap();
        crate::custom_fields::CustomFieldRepo::new(conn, &f.clock)
            .create(
                EntityRef::Client(f.client),
                crate::custom_fields::CustomFieldInput {
                    key: "Менеджер".into(),
                    value: "Светлана".into(),
                    field_type: crate::custom_fields::FieldType::Text,
                },
            )
            .unwrap();
        crate::interactions::InteractionRepo::new(conn, &f.clock)
            .create(crate::interactions::InteractionInput {
                client_id: f.client,
                contact_id: None,
                project_id: None,
                kind: crate::interactions::InteractionKind::Call,
                occurred_at: Timestamp::from_millis(1),
                summary_md: "Обсудили бюджет".into(),
            })
            .unwrap();
        conn.execute_batch("INSERT INTO blobs VALUES ('b1', 'x', 1, 'application/pdf', 0, 1, 1);")
            .unwrap();
        conn.execute(
            "INSERT INTO attachments (id, owner_type, owner_id, blob_id, file_name, mime,
                 created_at)
             VALUES (?2, 'client', ?1, 'b1', 'договор.pdf', 'application/pdf', 1)",
            (f.client, Id::new()),
        )
        .unwrap();

        let owner = Some(EntityRef::Client(f.client));
        let hit = |q: &str| search(conn, q, 5).unwrap().remove(0);
        let note = hit("скидка");
        assert_eq!(
            (note.kind, note.owner, note.title.as_str()),
            (SearchKind::Note, owner, "Скидка 10 %")
        );
        assert_eq!(
            note.snippet.as_deref(),
            Some("[Скидка] 10 %\nдо конца года")
        );
        let field = hit("светлана");
        assert_eq!((field.kind, field.owner), (SearchKind::Field, owner));
        assert_eq!(field.title, "Менеджер");
        let call = hit("бюджет");
        assert_eq!((call.kind, call.owner), (SearchKind::Interaction, owner));
        let file = hit("договор");
        assert_eq!(
            (file.kind, file.owner, file.title.as_str()),
            (SearchKind::File, owner, "договор.pdf")
        );
        let client = hit("ромашка");
        assert_eq!(
            (client.kind, client.id, client.owner),
            (SearchKind::Client, f.client, None)
        );
    }

    #[test]
    fn odd_input_is_safe() {
        let f = Fixture::new();
        for q in ["", "   ", "\"", "ром*", "AND", "(", "-", "NEAR(ром"] {
            assert!(search(f.db.conn(), q, 10).is_ok(), "{q}");
        }
        assert!(f.hits("").is_empty());
        assert_eq!(f.hits("\"ромашка\"").len(), 1);
        assert_eq!(
            fts_query("ООО «Ромашка»"),
            Some("\"ООО\"* \"Ромашка\"*".into())
        );
    }

    /// 100 000 records answer in under 100 ms (`cargo test -- --ignored`).
    #[test]
    #[ignore]
    fn searches_a_large_index_fast() {
        let f = Fixture::new();
        let conn = f.db.conn();
        conn.execute_batch("BEGIN").unwrap();
        for i in 0..100_000 {
            conn.execute(
                "INSERT INTO tasks (id, title, description_md, status_id, priority, client_id,
                     board_position, created_at, updated_at)
                 SELECT ?1, ?2, ?3, id, 'normal', ?4, 'a', 1, 1 FROM task_statuses LIMIT 1",
                rusqlite::params![
                    Id::new(),
                    format!("Задача номер {i}"),
                    format!("Описание работы {} для клиента", i % 977),
                    f.client
                ],
            )
            .unwrap();
        }
        conn.execute_batch("COMMIT").unwrap();
        let started = std::time::Instant::now();
        let hits = search(conn, "описание работы 42", 20).unwrap();
        let took = started.elapsed();
        assert!(!hits.is_empty());
        assert!(took.as_millis() < 100, "{took:?}");
    }
}
