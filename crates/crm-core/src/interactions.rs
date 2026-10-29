//! History of interactions with clients (FR-IN-1): calls, meetings,
//! e-mails and messages, newest first with cursor pagination.

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::domain::{Clock, Id, Timestamp};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum InteractionKind {
    Call,
    Meeting,
    Email,
    Message,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InteractionInput {
    pub client_id: Id,
    /// Must belong to the client.
    pub contact_id: Option<Id>,
    /// Must belong to the client.
    pub project_id: Option<Id>,
    pub kind: InteractionKind,
    pub occurred_at: Timestamp,
    pub summary_md: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Interaction {
    pub id: Id,
    pub client_id: Id,
    pub contact_id: Option<Id>,
    pub contact_name: Option<String>,
    pub project_id: Option<Id>,
    pub project_name: Option<String>,
    pub kind: InteractionKind,
    pub occurred_at: Timestamp,
    pub summary_md: String,
    /// Tasks and events created from it, oldest first.
    pub follow_ups: Vec<FollowUp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Where the next page starts: after this item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FeedCursor {
    pub occurred_at: Timestamp,
    pub id: Id,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FeedPage {
    pub items: Vec<Interaction>,
    /// `None` on the last page.
    pub next: Option<FeedCursor>,
}

/// A task or event created from an interaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FollowUp {
    pub kind: FollowUpKind,
    pub id: Id,
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum FollowUpKind {
    Task,
    Event,
}

pub struct InteractionRepo<'a> {
    conn: &'a Connection,
    clock: &'a dyn Clock,
}

impl<'a> InteractionRepo<'a> {
    pub fn new(conn: &'a Connection, clock: &'a dyn Clock) -> Self {
        Self { conn, clock }
    }

    fn validate(&self, input: InteractionInput) -> CoreResult<InteractionInput> {
        let summary_md = input.summary_md.trim().to_owned();
        if summary_md.is_empty() {
            return Err(CoreError::validation("summaryMd", "required"));
        }
        crate::db::ensure_exists(self.conn, "clients", "client", input.client_id)?;
        let owner = |table: &str, entity: &str, id: Id| -> CoreResult<Id> {
            self.conn
                .query_row(
                    &format!("SELECT client_id FROM {table} WHERE id = ?1"),
                    [id],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or_else(|| CoreError::not_found(entity, id))
        };
        if let Some(contact) = input.contact_id
            && owner("contacts", "contact", contact)? != input.client_id
        {
            return Err(CoreError::new("interaction.contact_mismatch"));
        }
        if let Some(project) = input.project_id
            && owner("projects", "project", project)? != input.client_id
        {
            return Err(CoreError::new("interaction.project_mismatch"));
        }
        Ok(InteractionInput {
            summary_md,
            ..input
        })
    }

    pub fn create(&self, input: InteractionInput) -> CoreResult<Interaction> {
        let i = self.validate(input)?;
        let id = Id::new();
        self.conn.execute(
            "INSERT INTO interactions (id, client_id, contact_id, project_id, kind, occurred_at,
                 summary_md, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            rusqlite::params![
                id,
                i.client_id,
                i.contact_id,
                i.project_id,
                i.kind,
                i.occurred_at,
                i.summary_md,
                self.clock.now(),
            ],
        )?;
        self.get(id)
    }

    pub fn update(&self, id: Id, input: InteractionInput) -> CoreResult<Interaction> {
        self.get(id)?;
        let i = self.validate(input)?;
        self.conn.execute(
            "UPDATE interactions SET client_id = ?2, contact_id = ?3, project_id = ?4, kind = ?5,
                 occurred_at = ?6, summary_md = ?7, updated_at = ?8
             WHERE id = ?1",
            rusqlite::params![
                id,
                i.client_id,
                i.contact_id,
                i.project_id,
                i.kind,
                i.occurred_at,
                i.summary_md,
                self.clock.now(),
            ],
        )?;
        self.get(id)
    }

    pub fn get(&self, id: Id) -> CoreResult<Interaction> {
        self.conn
            .query_row(&format!("{SELECT} WHERE i.id = ?1"), [id], from_row)
            .optional()?
            .ok_or_else(|| CoreError::not_found("interaction", id))
            .and_then(|i| self.with_follow_ups(i))
    }

    pub fn delete(&self, id: Id) -> CoreResult<()> {
        if self
            .conn
            .execute("DELETE FROM interactions WHERE id = ?1", [id])?
            == 0
        {
            return Err(CoreError::not_found("interaction", id));
        }
        Ok(())
    }

    /// Creates a task from the interaction, for its client and project.
    pub fn follow_up_task(
        &self,
        id: Id,
        title: &str,
        due_date: Option<String>,
    ) -> CoreResult<FollowUp> {
        let source = self.get(id)?;
        let task =
            crate::tasks::TaskRepo::new(self.conn, self.clock).create(crate::tasks::TaskInput {
                title: title.into(),
                description_md: quote(&source.summary_md),
                status_id: None,
                priority: crate::tasks::Priority::Normal,
                project_id: source.project_id,
                client_id: Some(source.client_id),
                due_date,
                due_time: None,
                estimate_minutes: None,
            })?;
        self.link(id, FollowUpKind::Task, task.id)?;
        Ok(FollowUp {
            kind: FollowUpKind::Task,
            id: task.id,
            title: task.title,
        })
    }

    /// Creates an event from the interaction, for its client and project.
    pub fn follow_up_event(
        &self,
        id: Id,
        input: crate::calendar::events::EventInput,
    ) -> CoreResult<FollowUp> {
        let source = self.get(id)?;
        let description_md = if input.description_md.trim().is_empty() {
            quote(&source.summary_md)
        } else {
            input.description_md
        };
        let event = crate::calendar::events::EventRepo::new(self.conn, self.clock).create(
            crate::calendar::events::EventInput {
                client_id: Some(source.client_id),
                project_id: source.project_id,
                contact_id: source.contact_id,
                description_md,
                ..input
            },
        )?;
        self.link(id, FollowUpKind::Event, event.id)?;
        Ok(FollowUp {
            kind: FollowUpKind::Event,
            id: event.id,
            title: event.title,
        })
    }

    fn link(&self, id: Id, kind: FollowUpKind, owner: Id) -> CoreResult<()> {
        self.conn.execute(
            "INSERT INTO interaction_follow_ups (interaction_id, kind, owner_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            (id, kind, owner, self.clock.now()),
        )?;
        Ok(())
    }

    fn with_follow_ups(&self, mut interaction: Interaction) -> CoreResult<Interaction> {
        let mut stmt = self.conn.prepare(
            "SELECT f.kind, f.owner_id, coalesce(t.title, e.title, '')
             FROM interaction_follow_ups f
             LEFT JOIN tasks t ON f.kind = 'task' AND t.id = f.owner_id
             LEFT JOIN events e ON f.kind = 'event' AND e.id = f.owner_id
             WHERE f.interaction_id = ?1
             ORDER BY f.created_at, f.rowid",
        )?;
        interaction.follow_ups = stmt
            .query_map([interaction.id], |r| {
                Ok(FollowUp {
                    kind: r.get(0)?,
                    id: r.get(1)?,
                    title: r.get(2)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(interaction)
    }

    /// A client's interactions, newest first, `limit` at a time.
    pub fn feed(
        &self,
        client_id: Id,
        kind: Option<InteractionKind>,
        after: Option<FeedCursor>,
        limit: u32,
    ) -> CoreResult<FeedPage> {
        let limit = limit.clamp(1, 200);
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT} WHERE i.client_id = ?1 AND (?2 IS NULL OR i.kind = ?2)
               AND (?3 IS NULL OR (i.occurred_at, i.id) < (?3, ?4))
             ORDER BY i.occurred_at DESC, i.id DESC
             LIMIT ?5"
        ))?;
        // One more than asked tells whether another page follows.
        let mut items = stmt
            .query_map(
                rusqlite::params![
                    client_id,
                    kind,
                    after.map(|c| c.occurred_at),
                    after.map(|c| c.id),
                    limit + 1
                ],
                from_row,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let next = if items.len() > limit as usize {
            items.truncate(limit as usize);
            items.last().map(|i| FeedCursor {
                occurred_at: i.occurred_at,
                id: i.id,
            })
        } else {
            None
        };
        let items = items
            .into_iter()
            .map(|i| self.with_follow_ups(i))
            .collect::<CoreResult<_>>()?;
        Ok(FeedPage { items, next })
    }
}

crate::db::text_enum!(InteractionKind {
    Call => "call",
    Meeting => "meeting",
    Email => "email",
    Message => "message",
    Other => "other",
});

crate::db::text_enum!(FollowUpKind { Task => "task", Event => "event" });

/// The interaction's summary as a Markdown quote, for the follow-up.
fn quote(summary: &str) -> String {
    summary
        .lines()
        .map(|l| format!("> {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

const SELECT: &str = "SELECT i.id, i.client_id, i.contact_id, ct.display_name AS contact_name,
        i.project_id, p.name AS project_name, i.kind, i.occurred_at, i.summary_md,
        i.created_at, i.updated_at
     FROM interactions i
     LEFT JOIN contacts ct ON ct.id = i.contact_id
     LEFT JOIN projects p ON p.id = i.project_id";

fn from_row(r: &Row) -> rusqlite::Result<Interaction> {
    Ok(Interaction {
        id: r.get("id")?,
        client_id: r.get("client_id")?,
        contact_id: r.get("contact_id")?,
        contact_name: r.get("contact_name")?,
        project_id: r.get("project_id")?,
        project_name: r.get("project_name")?,
        kind: r.get("kind")?,
        occurred_at: r.get("occurred_at")?,
        summary_md: r.get("summary_md")?,
        follow_ups: Vec::new(),
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::contacts::{ContactInput, ContactRepo};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};

    const HOUR: i64 = 3_600_000;

    struct Fixture {
        db: Db,
        clock: FakeClock,
        client: Id,
        contact: Id,
        project: Id,
        other: Id,
        others_contact: Id,
        others_project: Id,
    }

    impl Fixture {
        fn new() -> Self {
            let db = Db::open_in_memory().unwrap();
            let clock = FakeClock::at(Timestamp::from_millis(1_700_000_000_000));
            let client = |name: &str| {
                ClientRepo::new(db.conn(), &clock)
                    .create(ClientInput {
                        kind: ClientKind::Company,
                        last_name: None,
                        first_name: None,
                        middle_name: None,
                        company_name: Some(name.into()),
                        legal_name: None,
                        status: ClientStatus::Active,
                        default_currency: None,
                        default_rate_minor: None,
                    })
                    .unwrap()
                    .id
            };
            let contact = |client: Id, name: &str| {
                ContactRepo::new(db.conn(), &clock)
                    .create(
                        client,
                        ContactInput {
                            last_name: Some(name.into()),
                            first_name: None,
                            middle_name: None,
                            position: None,
                            note: None,
                            is_primary: false,
                        },
                    )
                    .unwrap()
                    .id
            };
            let project = |client: Id, name: &str| {
                ProjectRepo::new(db.conn(), &clock)
                    .create(ProjectInput {
                        client_id: client,
                        name: name.into(),
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
                    .id
            };
            let (romashka, acme) = (client("Ромашка"), client("Acme"));
            Self {
                contact: contact(romashka, "Петрова"),
                project: project(romashka, "Сайт"),
                others_contact: contact(acme, "Smith"),
                others_project: project(acme, "App"),
                client: romashka,
                other: acme,
                clock,
                db,
            }
        }

        fn repo(&self) -> InteractionRepo<'_> {
            InteractionRepo::new(self.db.conn(), &self.clock)
        }

        fn input(&self, kind: InteractionKind, hours_ago: i64, text: &str) -> InteractionInput {
            InteractionInput {
                client_id: self.client,
                contact_id: None,
                project_id: None,
                kind,
                occurred_at: Timestamp::from_millis(1_700_000_000_000 - hours_ago * HOUR),
                summary_md: text.into(),
            }
        }
    }

    #[test]
    fn records_an_interaction_with_its_links() {
        let f = Fixture::new();
        let i = f
            .repo()
            .create(InteractionInput {
                contact_id: Some(f.contact),
                project_id: Some(f.project),
                ..f.input(InteractionKind::Call, 2, "  Обсудили **сроки**  ")
            })
            .unwrap();
        assert_eq!(i.summary_md, "Обсудили **сроки**");
        assert_eq!(i.contact_name.as_deref(), Some("Петрова"));
        assert_eq!(i.project_name.as_deref(), Some("Сайт"));
        assert_eq!(f.repo().get(i.id).unwrap(), i);

        f.clock.advance_millis(1000);
        let edited = f
            .repo()
            .update(i.id, f.input(InteractionKind::Meeting, 3, "Встреча"))
            .unwrap();
        assert_eq!(
            (edited.kind, edited.contact_id),
            (InteractionKind::Meeting, None)
        );
        assert!(edited.updated_at > i.updated_at);
        f.repo().delete(i.id).unwrap();
        assert_eq!(
            f.repo().get(i.id).unwrap_err(),
            CoreError::not_found("interaction", i.id)
        );
    }

    #[test]
    fn contacts_and_projects_must_be_the_clients() {
        let f = Fixture::new();
        let base = f.input(InteractionKind::Email, 1, "x");
        assert_eq!(
            f.repo()
                .create(InteractionInput {
                    contact_id: Some(f.others_contact),
                    ..base.clone()
                })
                .unwrap_err(),
            CoreError::new("interaction.contact_mismatch")
        );
        assert_eq!(
            f.repo()
                .create(InteractionInput {
                    project_id: Some(f.others_project),
                    ..base.clone()
                })
                .unwrap_err(),
            CoreError::new("interaction.project_mismatch")
        );
        let missing = Id::new();
        assert_eq!(
            f.repo()
                .create(InteractionInput {
                    client_id: missing,
                    ..base.clone()
                })
                .unwrap_err(),
            CoreError::not_found("client", missing)
        );
        assert_eq!(
            f.repo()
                .create(InteractionInput {
                    summary_md: "  ".into(),
                    ..base
                })
                .unwrap_err(),
            CoreError::validation("summaryMd", "required")
        );
    }

    #[test]
    fn the_feed_is_newest_first_in_pages() {
        let f = Fixture::new();
        // Two at the same moment: the id breaks the tie.
        for (hours, kind) in [
            (5, InteractionKind::Call),
            (1, InteractionKind::Email),
            (3, InteractionKind::Call),
            (3, InteractionKind::Message),
            (9, InteractionKind::Other),
        ] {
            f.repo()
                .create(f.input(kind, hours, &format!("{hours}h")))
                .unwrap();
        }
        f.repo()
            .create(InteractionInput {
                client_id: f.other,
                ..f.input(InteractionKind::Call, 0, "чужое")
            })
            .unwrap();

        let mut seen = Vec::new();
        let mut after = None;
        loop {
            let page = f.repo().feed(f.client, None, after, 2).unwrap();
            assert!(page.items.len() <= 2);
            seen.extend(page.items.iter().map(|i| i.summary_md.clone()));
            match page.next {
                Some(next) => after = Some(next),
                None => break,
            }
        }
        assert_eq!(seen, ["1h", "3h", "3h", "5h", "9h"]);

        let calls = f
            .repo()
            .feed(f.client, Some(InteractionKind::Call), None, 10)
            .unwrap();
        assert_eq!(
            calls
                .items
                .iter()
                .map(|i| i.summary_md.as_str())
                .collect::<Vec<_>>(),
            ["3h", "5h"]
        );
        assert_eq!(calls.next, None);
    }

    #[test]
    fn follow_ups_carry_the_client_project_and_source() {
        use crate::calendar::events::{EventInput, EventRepo};
        use crate::tasks::TaskRepo;
        let f = Fixture::new();
        let call = f
            .repo()
            .create(InteractionInput {
                project_id: Some(f.project),
                ..f.input(InteractionKind::Call, 1, "Договорились прислать смету")
            })
            .unwrap();

        let task = f
            .repo()
            .follow_up_task(call.id, "Прислать смету", Some("2026-03-20".into()))
            .unwrap();
        let created = TaskRepo::new(f.db.conn(), &f.clock).get(task.id).unwrap();
        assert_eq!(
            (
                created.client_id,
                created.project_id,
                created.due_date.as_deref()
            ),
            (Some(f.client), Some(f.project), Some("2026-03-20"))
        );
        assert_eq!(created.title, "Прислать смету");
        // The task links back to the call in its description.
        assert!(
            created
                .description_md
                .contains("Договорились прислать смету")
        );

        let event = f
            .repo()
            .follow_up_event(
                call.id,
                EventInput {
                    title: "Обсудить смету".into(),
                    description_md: String::new(),
                    location: None,
                    all_day: false,
                    start: "2026-03-21T10:00".into(),
                    end: "2026-03-21T11:00".into(),
                    tzid: "UTC".into(),
                    client_id: None,
                    project_id: None,
                    task_id: None,
                    contact_id: None,
                    rrule: None,
                },
            )
            .unwrap();
        let created = EventRepo::new(f.db.conn(), &f.clock).get(event.id).unwrap();
        assert_eq!(
            (created.client_id, created.project_id),
            (Some(f.client), Some(f.project))
        );

        assert_eq!(
            f.repo().get(call.id).unwrap().follow_ups,
            [
                FollowUp {
                    kind: FollowUpKind::Task,
                    id: task.id,
                    title: "Прислать смету".into()
                },
                FollowUp {
                    kind: FollowUpKind::Event,
                    id: event.id,
                    title: "Обсудить смету".into()
                },
            ]
        );
        // Deleting a follow-up removes the link.
        TaskRepo::new(f.db.conn(), &f.clock)
            .delete(task.id)
            .unwrap();
        assert_eq!(f.repo().get(call.id).unwrap().follow_ups.len(), 1);
        assert_eq!(
            f.repo().follow_up_task(call.id, "  ", None).unwrap_err(),
            CoreError::validation("title", "required")
        );
    }

    #[test]
    fn deleting_the_client_deletes_its_history() {
        let f = Fixture::new();
        f.repo()
            .create(f.input(InteractionKind::Call, 1, "x"))
            .unwrap();
        assert_eq!(
            ClientRepo::new(f.db.conn(), &f.clock)
                .delete_impact(f.client)
                .unwrap()
                .interactions,
            1
        );
        ClientRepo::new(f.db.conn(), &f.clock)
            .delete(f.client)
            .unwrap();
        assert!(
            f.repo()
                .feed(f.client, None, None, 10)
                .unwrap()
                .items
                .is_empty()
        );
        let _ = f.other;
    }
}
