//! Time reports (FR-TT-4, 6): the journal of entries for a filter, totals
//! grouped by day (in the user's zone), project or task with money at the
//! project's or else the client's hourly rate, optional rounding up of
//! each entry, and CSV export.

use jiff::tz::TimeZone;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::domain::{Currency, Id, Money, Timestamp};
use crate::time_entries::{SELECT, TimeEntry, from_row};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TimeFilter {
    /// Entries starting in `[from, to)`.
    pub from: Option<Timestamp>,
    pub to: Option<Timestamp>,
    pub client_id: Option<Id>,
    pub project_id: Option<Id>,
    pub billable: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum GroupBy {
    Day,
    Project,
    Task,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReportRow {
    /// `YYYY-MM-DD`, or the project's or task's id; empty for "none".
    pub key: String,
    /// The project's or task's name; empty for days and "none".
    pub label: String,
    #[specta(type = i32)]
    pub seconds: i64,
    #[specta(type = i32)]
    pub billable_seconds: i64,
    /// Per currency, for billable entries with a rate.
    pub amounts: Vec<Money>,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TimeReport {
    pub rows: Vec<ReportRow>,
    pub total: ReportRow,
}

/// Finished entries matching the filter, newest first.
pub fn entries(conn: &Connection, filter: TimeFilter) -> CoreResult<Vec<TimeEntry>> {
    Ok(rated(conn, filter)?.into_iter().map(|r| r.entry).collect())
}

/// An entry with the hourly rate that applies to it.
struct Rated {
    entry: TimeEntry,
    rate: Option<Money>,
}

impl Rated {
    /// Duration in seconds, rounded up to whole `rounding_minutes`.
    fn seconds(&self, rounding_minutes: u32) -> i64 {
        let ended = self.entry.ended_at.expect("finished entries only");
        let exact = (ended.as_millis() - self.entry.started_at.as_millis()) / 1000;
        let step = i64::from(rounding_minutes) * 60;
        if step == 0 {
            exact
        } else {
            (exact + step - 1) / step * step
        }
    }

    fn amount(&self, seconds: i64) -> CoreResult<Option<Money>> {
        match (self.entry.billable, self.rate) {
            (true, Some(rate)) => rate
                .for_seconds(seconds)
                .map(Some)
                .map_err(|_| CoreError::new("money.overflow")),
            _ => Ok(None),
        }
    }
}

/// The project's hourly rate, else the client's default one.
fn rated(conn: &Connection, filter: TimeFilter) -> CoreResult<Vec<Rated>> {
    let select = SELECT.replacen(
        " FROM time_entries e",
        ", p.rate_minor AS project_rate, p.currency AS project_currency,
           c.default_rate_minor AS client_rate, c.default_currency AS client_currency
         FROM time_entries e",
        1,
    );
    let mut stmt = conn.prepare(&format!(
        "{select}
         WHERE e.ended_at IS NOT NULL
           AND (?1 IS NULL OR e.started_at >= ?1)
           AND (?2 IS NULL OR e.started_at < ?2)
           AND (?3 IS NULL OR e.client_id = ?3)
           AND (?4 IS NULL OR e.project_id = ?4)
           AND (?5 IS NULL OR e.billable = ?5)
         ORDER BY e.started_at DESC, e.id"
    ))?;
    let rows = stmt.query_map(
        rusqlite::params![
            filter.from,
            filter.to,
            filter.client_id,
            filter.project_id,
            filter.billable
        ],
        |r| {
            let project: (Option<i64>, Option<Currency>) =
                (r.get("project_rate")?, r.get("project_currency")?);
            let client: (Option<i64>, Option<Currency>) =
                (r.get("client_rate")?, r.get("client_currency")?);
            let rate = match (project, client) {
                ((Some(rate), Some(currency)), _) => Some(Money::new(rate, currency)),
                (_, (Some(rate), Some(currency))) => Some(Money::new(rate, currency)),
                _ => None,
            };
            Ok(Rated {
                entry: from_row(r)?,
                rate,
            })
        },
    )?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Adds `amount` to the per-currency sums, kept sorted by currency.
fn add_amount(sums: &mut Vec<Money>, amount: Money) -> CoreResult<()> {
    match sums.iter_mut().find(|m| m.currency == amount.currency) {
        Some(sum) => {
            *sum = sum
                .checked_add(amount)
                .map_err(|_| CoreError::new("money.overflow"))?
        }
        None => {
            sums.push(amount);
            sums.sort_by(|a, b| a.currency.code().cmp(b.currency.code()));
        }
    }
    Ok(())
}

fn row(key: String, label: String) -> ReportRow {
    ReportRow {
        key,
        label,
        seconds: 0,
        billable_seconds: 0,
        amounts: Vec::new(),
    }
}

fn local_date(at: Timestamp, tz: &TimeZone) -> String {
    jiff::Timestamp::from_millisecond(at.as_millis())
        .unwrap_or_default()
        .to_zoned(tz.clone())
        .date()
        .to_string()
}

/// Totals by `group`; durations are rounded up to `rounding_minutes`
/// per entry (0: exact).
pub fn report(
    conn: &Connection,
    filter: TimeFilter,
    group: GroupBy,
    tz: &TimeZone,
    rounding_minutes: u32,
) -> CoreResult<TimeReport> {
    let mut rows: Vec<ReportRow> = Vec::new();
    let mut total = row(String::new(), String::new());
    for r in rated(conn, filter)? {
        let (key, label) = match group {
            GroupBy::Day => (local_date(r.entry.started_at, tz), String::new()),
            GroupBy::Project => (
                r.entry
                    .project_id
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
                r.entry.project_name.clone().unwrap_or_default(),
            ),
            GroupBy::Task => (
                r.entry.task_id.map(|id| id.to_string()).unwrap_or_default(),
                r.entry.task_title.clone().unwrap_or_default(),
            ),
        };
        let index = match rows.iter().position(|x| x.key == key) {
            Some(i) => i,
            None => {
                rows.push(row(key, label));
                rows.len() - 1
            }
        };
        let seconds = r.seconds(rounding_minutes);
        let amount = r.amount(seconds)?;
        for target in [&mut rows[index], &mut total] {
            target.seconds += seconds;
            if r.entry.billable {
                target.billable_seconds += seconds;
            }
            if let Some(amount) = amount {
                add_amount(&mut target.amounts, amount)?;
            }
        }
    }
    match group {
        GroupBy::Day => rows.sort_by(|a, b| a.key.cmp(&b.key)),
        // By name, entries without a project or task last.
        _ => rows.sort_by(|a, b| {
            (a.key.is_empty(), crate::domain::text::fold(&a.label))
                .cmp(&(b.key.is_empty(), crate::domain::text::fold(&b.label)))
        }),
    }
    Ok(TimeReport { rows, total })
}

/// Quotes a CSV field when needed (RFC 4180).
fn field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn decimal(minor: i64, units: u8) -> String {
    let scale = 10i64.pow(u32::from(units));
    let sign = if minor < 0 { "-" } else { "" };
    let (whole, part) = (minor.abs() / scale, minor.abs() % scale);
    if units == 0 {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{part:0width$}", width = usize::from(units))
    }
}

/// RFC 4180 CSV of the entries, oldest first, with the given 11 column
/// headers: date, start, end, hours, client, project, task, description,
/// billable, amount, currency.
pub fn csv(
    conn: &Connection,
    filter: TimeFilter,
    tz: &TimeZone,
    rounding_minutes: u32,
    headers: &[String],
) -> CoreResult<String> {
    if headers.len() != 11 {
        return Err(CoreError::validation("headers", "count"));
    }
    let mut out = String::new();
    let mut line = |cells: Vec<String>| {
        out.push_str(&cells.iter().map(|c| field(c)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    };
    line(headers.to_vec());
    let mut entries = rated(conn, filter)?;
    entries.reverse();
    for r in entries {
        let at = |t: Timestamp| {
            jiff::Timestamp::from_millisecond(t.as_millis())
                .unwrap_or_default()
                .to_zoned(tz.clone())
        };
        let (start, end) = (
            at(r.entry.started_at),
            at(r.entry.ended_at.expect("finished")),
        );
        let seconds = r.seconds(rounding_minutes);
        let amount = r.amount(seconds)?;
        // Hundredths of an hour, rounded half up.
        let hours = (seconds * 100 + 1800) / 3600;
        let e = &r.entry;
        line(vec![
            start.date().to_string(),
            start.strftime("%H:%M").to_string(),
            end.strftime("%H:%M").to_string(),
            decimal(hours, 2),
            e.client_name.clone().unwrap_or_default(),
            e.project_name.clone().unwrap_or_default(),
            e.task_title.clone().unwrap_or_default(),
            e.description.clone(),
            if e.billable { "yes" } else { "no" }.to_owned(),
            amount
                .map(|m| decimal(m.amount_minor, m.currency.minor_units()))
                .unwrap_or_default(),
            amount
                .map(|m| m.currency.code().to_owned())
                .unwrap_or_default(),
        ]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{ClientInput, ClientKind, ClientRepo, ClientStatus};
    use crate::db::Db;
    use crate::domain::FakeClock;
    use crate::projects::{BillingType, ProjectInput, ProjectRepo, ProjectStatus};
    use crate::tasks::{Priority, TaskInput, TaskRepo};
    use crate::time_entries::{TimeEntryInput, TimeRepo, TimeTarget};

    fn utc(s: &str) -> Timestamp {
        Timestamp::from_millis(s.parse::<jiff::Timestamp>().unwrap().as_millisecond())
    }

    fn money(minor: i64, currency: &str) -> Money {
        Money::new(minor, currency.parse().unwrap())
    }

    struct Fixture {
        db: Db,
        clock: FakeClock,
        client: Id,
        site: Id,
        task: Id,
    }

    /// Ромашка (2 000 ₽/h by default) with project "Сайт" at 3 000 ₽/h and
    /// task "Макет"; project "Поддержка" without a rate; and a client in
    /// dollars without projects.
    impl Fixture {
        fn new() -> Self {
            let db = Db::open_in_memory().unwrap();
            let clock = FakeClock::at(utc("2026-03-31T00:00:00Z"));
            let clients = ClientRepo::new(db.conn(), &clock);
            let client = |name: &str, currency: &str, rate: i64| {
                clients
                    .create(ClientInput {
                        kind: ClientKind::Company,
                        last_name: None,
                        first_name: None,
                        middle_name: None,
                        company_name: Some(name.into()),
                        legal_name: None,
                        status: ClientStatus::Active,
                        default_currency: Some(currency.parse().unwrap()),
                        default_rate_minor: Some(rate),
                    })
                    .unwrap()
                    .id
            };
            let romashka = client("Ромашка", "RUB", 200_000);
            let acme = client("Acme", "USD", 5_000);
            let projects = ProjectRepo::new(db.conn(), &clock);
            let project = |name: &str, rate: Option<i64>| {
                projects
                    .create(ProjectInput {
                        client_id: romashka,
                        name: name.into(),
                        description_md: String::new(),
                        status: ProjectStatus::Active,
                        billing_type: if rate.is_some() {
                            BillingType::Hourly
                        } else {
                            BillingType::Fixed
                        },
                        currency: Some("RUB".parse().unwrap()),
                        rate_minor: rate,
                        budget_minor: if rate.is_some() { None } else { Some(1) },
                        start_date: None,
                        deadline: None,
                    })
                    .unwrap()
                    .id
            };
            let site = project("Сайт", Some(300_000));
            let support = project("Поддержка", None);
            let task = TaskRepo::new(db.conn(), &clock)
                .create(TaskInput {
                    title: "Макет".into(),
                    description_md: String::new(),
                    status_id: None,
                    priority: Priority::Normal,
                    project_id: Some(site),
                    client_id: None,
                    due_date: None,
                    due_time: None,
                    estimate_minutes: None,
                })
                .unwrap()
                .id;
            let f = Self {
                db,
                clock,
                client: romashka,
                site,
                task,
            };
            let add = |target: TimeTarget, from: &str, to: &str, billable: bool, text: &str| {
                TimeRepo::new(f.db.conn(), &f.clock)
                    .create(TimeEntryInput {
                        target,
                        description: text.into(),
                        started_at: utc(from),
                        ended_at: utc(to),
                        billable,
                    })
                    .unwrap();
            };
            let on_task = TimeTarget {
                task_id: Some(task),
                ..Default::default()
            };
            let on_site = TimeTarget {
                project_id: Some(site),
                ..Default::default()
            };
            let on_support = TimeTarget {
                project_id: Some(support),
                ..Default::default()
            };
            let on_acme = TimeTarget {
                client_id: Some(acme),
                ..Default::default()
            };
            // 22:30 UTC is already the next day in Moscow.
            add(
                on_task,
                "2026-03-09T22:30:00Z",
                "2026-03-09T23:30:00Z",
                true,
                "Вёрстка, \"шапка\"",
            );
            add(
                on_site,
                "2026-03-10T07:00:00Z",
                "2026-03-10T07:07:00Z",
                true,
                "Созвон",
            );
            add(
                on_support,
                "2026-03-10T08:00:00Z",
                "2026-03-10T09:30:00Z",
                true,
                "Правки\nпо почте",
            );
            add(
                on_support,
                "2026-03-11T08:00:00Z",
                "2026-03-11T08:30:00Z",
                false,
                "Внутреннее",
            );
            add(
                on_acme,
                "2026-03-11T10:00:00Z",
                "2026-03-11T12:00:00Z",
                true,
                "Consulting",
            );
            f
        }

        fn report(&self, filter: TimeFilter, group: GroupBy, rounding: u32) -> TimeReport {
            report(
                self.db.conn(),
                filter,
                group,
                &TimeZone::get("Europe/Moscow").unwrap(),
                rounding,
            )
            .unwrap()
        }
    }

    fn summary(rows: &[ReportRow]) -> Vec<(String, i64, Vec<Money>)> {
        rows.iter()
            .map(|r| {
                (
                    if r.label.is_empty() {
                        r.key.clone()
                    } else {
                        r.label.clone()
                    },
                    r.seconds / 60,
                    r.amounts.clone(),
                )
            })
            .collect()
    }

    #[test]
    fn groups_by_day_in_the_users_zone() {
        let f = Fixture::new();
        let r = f.report(TimeFilter::default(), GroupBy::Day, 0);
        assert_eq!(
            summary(&r.rows),
            [
                // Макет and the call at the project's 3 000 ₽/h, support at the
                // client's 2 000 ₽/h.
                (
                    "2026-03-10".into(),
                    60 + 7 + 90,
                    vec![money(300_000 + 35_000 + 300_000, "RUB")]
                ),
                ("2026-03-11".into(), 30 + 120, vec![money(10_000, "USD")]),
            ]
        );
        assert_eq!(r.total.seconds, (60 + 7 + 90 + 30 + 120) * 60);
        assert_eq!(r.total.billable_seconds, (60 + 7 + 90 + 120) * 60);
        assert_eq!(
            r.total.amounts,
            [money(635_000, "RUB"), money(10_000, "USD")]
        );
    }

    #[test]
    fn groups_by_project_and_task_with_the_rate_cascade() {
        let f = Fixture::new();
        let by_project = f.report(TimeFilter::default(), GroupBy::Project, 0);
        assert_eq!(
            summary(&by_project.rows),
            [
                // No project rate: the client's 2 000 ₽/h; billable time only.
                ("Поддержка".into(), 120, vec![money(300_000, "RUB")]),
                ("Сайт".into(), 67, vec![money(335_000, "RUB")]),
                // Time on the client itself.
                ("".into(), 120, vec![money(10_000, "USD")]),
            ]
        );
        let by_task = f.report(TimeFilter::default(), GroupBy::Task, 0);
        assert_eq!(
            summary(&by_task.rows),
            [
                ("Макет".into(), 60, vec![money(300_000, "RUB")]),
                (
                    "".into(),
                    7 + 90 + 30 + 120,
                    vec![money(35_000 + 300_000, "RUB"), money(10_000, "USD")]
                ),
            ]
        );
    }

    #[test]
    fn filters_and_rounds_up() {
        let f = Fixture::new();
        let r = f.report(
            TimeFilter {
                project_id: Some(f.site),
                ..Default::default()
            },
            GroupBy::Project,
            15,
        );
        // 60 min stays; 7 min becomes 15.
        assert_eq!(
            summary(&r.rows),
            [("Сайт".into(), 75, vec![money(375_000, "RUB")])]
        );
        let billable_only = f.report(
            TimeFilter {
                client_id: Some(f.client),
                billable: Some(false),
                ..Default::default()
            },
            GroupBy::Day,
            0,
        );
        assert_eq!(
            summary(&billable_only.rows),
            [("2026-03-11".into(), 30, vec![])]
        );
        let range = f.report(
            TimeFilter {
                from: Some(utc("2026-03-11T00:00:00Z")),
                to: Some(utc("2026-03-12T00:00:00Z")),
                ..Default::default()
            },
            GroupBy::Day,
            0,
        );
        assert_eq!(range.total.seconds, (30 + 120) * 60);
        let _ = f.task;
    }

    #[test]
    fn running_timers_are_left_out() {
        let f = Fixture::new();
        TimeRepo::new(f.db.conn(), &f.clock)
            .start(TimeTarget::default(), "идёт", true, true)
            .unwrap();
        assert_eq!(
            entries(f.db.conn(), TimeFilter::default()).unwrap().len(),
            5
        );
    }

    #[test]
    fn exports_csv() {
        let f = Fixture::new();
        let headers: Vec<String> = [
            "Date",
            "Start",
            "End",
            "Hours",
            "Client",
            "Project",
            "Task",
            "Description",
            "Billable",
            "Amount",
            "Currency",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let text = csv(
            f.db.conn(),
            TimeFilter {
                project_id: Some(f.site),
                ..Default::default()
            },
            &TimeZone::get("Europe/Moscow").unwrap(),
            0,
            &headers,
        )
        .unwrap();
        assert_eq!(
            text,
            "Date,Start,End,Hours,Client,Project,Task,Description,Billable,Amount,Currency\r\n\
             2026-03-10,01:30,02:30,1.00,Ромашка,Сайт,Макет,\"Вёрстка, \"\"шапка\"\"\",yes,3000.00,RUB\r\n\
             2026-03-10,10:00,10:07,0.12,Ромашка,Сайт,,Созвон,yes,350.00,RUB\r\n"
        );
        let all = csv(
            f.db.conn(),
            TimeFilter::default(),
            &TimeZone::UTC,
            0,
            &headers,
        )
        .unwrap();
        assert!(all.contains("\"Правки\nпо почте\""));
        assert!(all.contains(",no,,\r\n"));
        assert!(all.contains(",Acme,,,Consulting,yes,100.00,USD\r\n"));
    }
}
