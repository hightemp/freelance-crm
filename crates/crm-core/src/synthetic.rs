//! Synthetic data for benchmarks and performance budgets (NFR-1):
//! clients with projects, tasks, time entries and payments, inserted in one
//! transaction straight through SQL (the search triggers still run).

use rusqlite::Connection;

use crate::CoreResult;
use crate::domain::Id;

#[derive(Debug, Clone, Copy)]
pub struct Sizes {
    pub clients: usize,
    pub projects_per_client: usize,
    pub tasks: usize,
    pub time_entries: usize,
    pub payments: usize,
}

impl Sizes {
    /// The NFR-1 scale: 10 000 tasks.
    pub const LARGE: Self = Self {
        clients: 500,
        projects_per_client: 4,
        tasks: 10_000,
        time_entries: 20_000,
        payments: 10_000,
    };
}

const WORDS: &[&str] = &[
    "макет",
    "вёрстка",
    "дизайн",
    "сайт",
    "лендинг",
    "отчёт",
    "созвон",
    "правки",
    "api",
    "сервер",
    "бэкап",
    "миграция",
    "тесты",
    "релиз",
    "логотип",
    "баннер",
    "каталог",
    "корзина",
    "оплата",
    "договор",
];

/// A deterministic phrase for `n`.
fn phrase(n: usize, words: usize) -> String {
    (0..words)
        .map(|i| WORDS[(n * 7 + i * 13) % WORDS.len()])
        .collect::<Vec<_>>()
        .join(" ")
}

/// Fills the database; returns nothing but rows. Times are spread over
/// 2025–2026, from `start_ms`.
pub fn fill(conn: &Connection, sizes: Sizes) -> CoreResult<()> {
    let tx = conn.unchecked_transaction()?;
    let start_ms: i64 = 1_735_689_600_000; // 2025-01-01
    let day = 86_400_000;
    let mut clients = Vec::with_capacity(sizes.clients);
    let mut projects: Vec<(Id, Id)> = Vec::new();
    {
        let mut client = tx.prepare(
            "INSERT INTO clients (id, kind, company_name, display_name, sort_key, status,
                 default_currency, default_rate_minor, created_at, updated_at)
             VALUES (?1, 'company', ?2, ?2, lower(?2), 'active', 'RUB', 150000, ?3, ?3)",
        )?;
        let mut project = tx.prepare(
            "INSERT INTO projects (id, client_id, name, sort_key, description_md, status,
                 billing_type, currency, rate_minor, created_at, updated_at)
             VALUES (?1, ?2, ?3, lower(?3), ?4, 'active', 'hourly', 'RUB', 200000, ?5, ?5)",
        )?;
        for c in 0..sizes.clients {
            let id = Id::new();
            let name = format!("ООО Клиент {c} {}", phrase(c, 1));
            client.execute(rusqlite::params![id, name, start_ms + c as i64 * 60_000])?;
            clients.push(id);
            for p in 0..sizes.projects_per_client {
                let pid = Id::new();
                project.execute(rusqlite::params![
                    pid,
                    id,
                    format!("Проект {c}-{p} {}", phrase(c + p, 2)),
                    phrase(c * 3 + p, 8),
                    start_ms + c as i64 * 60_000
                ])?;
                projects.push((pid, id));
            }
        }
    }
    {
        let mut task = tx.prepare(
            "INSERT INTO tasks (id, title, description_md, status_id, priority, project_id,
                 client_id, due_date, board_position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        )?;
        const STATUSES: [&str; 4] = [
            "00000000-0000-7000-8000-000000000001",
            "00000000-0000-7000-8000-000000000002",
            "00000000-0000-7000-8000-000000000003",
            "00000000-0000-7000-8000-000000000004",
        ];
        const PRIORITIES: [&str; 4] = ["low", "normal", "high", "urgent"];
        for t in 0..sizes.tasks {
            let (project, client) = projects[t % projects.len().max(1)];
            let due = jiff::civil::date(2025, 1, 1)
                .checked_add(jiff::Span::new().days((t % 700) as i64))
                .expect("in range")
                .to_string();
            task.execute(rusqlite::params![
                Id::new(),
                format!("Задача {t}: {}", phrase(t, 3)),
                phrase(t * 5, 12),
                STATUSES[t % 4],
                PRIORITIES[t % 4],
                project,
                client,
                due,
                format!("a{t:08}"),
                start_ms + t as i64 * 3_600_000
            ])?;
        }
    }
    {
        let mut entry = tx.prepare(
            "INSERT INTO time_entries (id, project_id, client_id, description, started_at,
                 ended_at, billable, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?5, ?5)",
        )?;
        for e in 0..sizes.time_entries {
            let (project, client) = projects[e % projects.len().max(1)];
            let started = start_ms + (e as i64 % 700) * day + (e as i64 % 8) * 3_600_000;
            entry.execute(rusqlite::params![
                Id::new(),
                project,
                client,
                phrase(e, 2),
                started,
                started + 90 * 60_000
            ])?;
        }
    }
    {
        let mut payment = tx.prepare(
            "INSERT INTO payments (id, project_id, client_id, amount_minor, currency, paid_on,
                 created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        )?;
        for p in 0..sizes.payments {
            let (project, client) = projects[p % projects.len().max(1)];
            let paid_on = jiff::civil::date(2025, 1, 1)
                .checked_add(jiff::Span::new().days((p % 700) as i64))
                .expect("in range")
                .to_string();
            payment.execute(rusqlite::params![
                Id::new(),
                project,
                client,
                10_000 + (p as i64 % 50) * 1_000,
                if p % 5 == 0 { "USD" } else { "RUB" },
                paid_on,
                start_ms
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Performance budgets of NFR-1 on the backend side, at the large scale.
/// Run in release: `cargo test --release -p crm-core -- --ignored budget`.
#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use jiff::tz::TimeZone;

    use super::*;
    use crate::db::Db;
    use crate::domain::{FakeClock, Timestamp};

    fn large() -> (Db, FakeClock) {
        let db = Db::open_in_memory().unwrap();
        fill(db.conn(), Sizes::LARGE).unwrap();
        (db, FakeClock::at(Timestamp::from_millis(1_773_000_000_000)))
    }

    /// The best of a few runs, so that a cold cache does not decide.
    fn timed<T>(mut f: impl FnMut() -> T) -> Duration {
        (0..3)
            .map(|_| {
                let started = Instant::now();
                std::hint::black_box(f());
                started.elapsed()
            })
            .min()
            .unwrap()
    }

    #[test]
    fn fills_what_it_says() {
        let db = Db::open_in_memory().unwrap();
        fill(
            db.conn(),
            Sizes {
                clients: 3,
                projects_per_client: 2,
                tasks: 10,
                time_entries: 7,
                payments: 5,
            },
        )
        .unwrap();
        let count = |t: &str| -> i64 {
            db.conn()
                .query_row(&format!("SELECT count(*) FROM {t}"), [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(
            [count("clients"), count("projects"), count("tasks")],
            [3, 6, 10]
        );
        assert_eq!([count("time_entries"), count("payments")], [7, 5]);
        assert_eq!(count("search_docs"), 3 + 6 + 10);
    }

    #[test]
    #[ignore]
    fn budget_task_list_of_10_000() {
        let (db, clock) = large();
        let repo = crate::tasks::TaskRepo::new(db.conn(), &clock);
        let took = timed(|| {
            repo.list(&crate::tasks::TaskFilter::default(), &TimeZone::UTC)
                .unwrap()
        });
        assert!(took < Duration::from_millis(300), "{took:?}");
    }

    #[test]
    #[ignore]
    fn budget_client_list() {
        let (db, clock) = large();
        let repo = crate::clients::ClientRepo::new(db.conn(), &clock);
        let took = timed(|| repo.list(&crate::clients::ClientFilter::default()).unwrap());
        assert!(took < Duration::from_millis(100), "{took:?}");
    }

    #[test]
    #[ignore]
    fn budget_dashboard_of_a_year() {
        let (db, clock) = large();
        let period = crate::finance::dashboard::Period {
            from: "2025-01-01".into(),
            to: "2025-12-31".into(),
        };
        let took = timed(|| {
            crate::finance::dashboard::dashboard(db.conn(), &clock, TimeZone::UTC, &period).unwrap()
        });
        assert!(took < Duration::from_millis(300), "{took:?}");
    }

    #[test]
    #[ignore]
    fn budget_search() {
        let (db, _) = large();
        let took = timed(|| crate::search::search(db.conn(), "макет вёрстка", 20).unwrap());
        assert!(took < Duration::from_millis(100), "{took:?}");
    }
}
