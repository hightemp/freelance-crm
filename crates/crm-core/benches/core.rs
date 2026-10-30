//! Benchmarks of NFR-1 on synthetic data: lists, search, aggregates.
//! `cargo bench -p crm-core --features synthetic`

use criterion::{Criterion, criterion_group, criterion_main};
use crm_core::db::Db;
use crm_core::domain::{FakeClock, Timestamp};
use crm_core::synthetic::{Sizes, fill};
use jiff::tz::TimeZone;

fn benches(c: &mut Criterion) {
    let db = Db::open_in_memory().unwrap();
    fill(db.conn(), Sizes::LARGE).unwrap();
    let clock = FakeClock::at(Timestamp::from_millis(1_773_000_000_000));
    let conn = db.conn();

    c.bench_function("tasks: list 10 000", |b| {
        let repo = crm_core::tasks::TaskRepo::new(conn, &clock);
        b.iter(|| repo.list(&Default::default(), &TimeZone::UTC).unwrap());
    });
    c.bench_function("clients: list", |b| {
        let repo = crm_core::clients::ClientRepo::new(conn, &clock);
        b.iter(|| repo.list(&Default::default()).unwrap());
    });
    c.bench_function("search: two words", |b| {
        b.iter(|| crm_core::search::search(conn, "макет вёрстка", 20).unwrap());
    });
    c.bench_function("finance: dashboard of a year", |b| {
        let period = crm_core::finance::dashboard::Period {
            from: "2025-01-01".into(),
            to: "2025-12-31".into(),
        };
        b.iter(|| {
            crm_core::finance::dashboard::dashboard(conn, &clock, TimeZone::UTC, &period).unwrap()
        });
    });
    c.bench_function("time: report by project", |b| {
        b.iter(|| {
            crm_core::time_report::report(
                conn,
                Default::default(),
                crm_core::time_report::GroupBy::Project,
                &TimeZone::UTC,
                0,
            )
            .unwrap()
        });
    });
}

criterion_group!(core, benches);
criterion_main!(core);
