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
