//! Invoice numbers from a template (FR-FN-3) such as `INV-{YYYY}-{NNN}`:
//! `{YYYY}`, `{YY}` and `{MM}` come from the issue date and `{N…}` is a
//! counter as wide as the number of `N`s. The counter restarts each month
//! when the template has `{MM}`, else each year when it has a year, and
//! never otherwise. Increments are single atomic statements, so numbers
//! stay unique across connections.

use rusqlite::Connection;

use crate::{CoreError, CoreResult};

pub const DEFAULT_TEMPLATE: &str = "INV-{YYYY}-{NNN}";

enum Part {
    Text(String),
    Year4,
    Year2,
    Month,
    Counter(usize),
}

fn invalid() -> CoreError {
    CoreError::validation("invoiceNumberTemplate", "template")
}

fn parse(template: &str) -> CoreResult<Vec<Part>> {
    let mut parts = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        if open > 0 {
            parts.push(Part::Text(rest[..open].to_owned()));
        }
        let close = rest[open..].find('}').ok_or_else(invalid)? + open;
        let name = &rest[open + 1..close];
        parts.push(match name {
            "YYYY" => Part::Year4,
            "YY" => Part::Year2,
            "MM" => Part::Month,
            n if !n.is_empty() && n.chars().all(|c| c == 'N') => Part::Counter(n.len()),
            _ => return Err(invalid()),
        });
        rest = &rest[close + 1..];
    }
    if !rest.is_empty() {
        if rest.contains('}') {
            return Err(invalid());
        }
        parts.push(Part::Text(rest.to_owned()));
    }
    if parts
        .iter()
        .filter(|p| matches!(p, Part::Counter(_)))
        .count()
        != 1
    {
        return Err(invalid());
    }
    Ok(parts)
}

/// Checks that the template has exactly one counter and known parts.
pub fn validate_template(template: &str) -> CoreResult<()> {
    parse(template).map(|_| ())
}

/// The counter's key: the template and the period it restarts with.
fn key(parts: &[Part], template: &str, date: &str) -> String {
    let period = if parts.iter().any(|p| matches!(p, Part::Month)) {
        &date[..7]
    } else if parts.iter().any(|p| matches!(p, Part::Year4 | Part::Year2)) {
        &date[..4]
    } else {
        "*"
    };
    format!("{template}|{period}")
}

fn render(parts: &[Part], date: &str, n: i64) -> String {
    parts
        .iter()
        .map(|p| match p {
            Part::Text(t) => t.clone(),
            Part::Year4 => date[..4].to_owned(),
            Part::Year2 => date[2..4].to_owned(),
            Part::Month => date[5..7].to_owned(),
            Part::Counter(width) => format!("{n:0width$}"),
        })
        .collect()
}

fn checked(template: &str, date: &str) -> CoreResult<Vec<Part>> {
    if !crate::domain::date::is_iso_date(date) {
        return Err(CoreError::validation("issueDate", "date"));
    }
    parse(template)
}

fn taken(conn: &Connection, number: &str) -> CoreResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM invoices WHERE number = ?1)",
        [number],
        |r| r.get(0),
    )?)
}

/// The number the next invoice issued on `issue_date` would get, without
/// using it up.
pub fn preview(conn: &Connection, template: &str, issue_date: &str) -> CoreResult<String> {
    let parts = checked(template, issue_date)?;
    let last: i64 = conn.query_row(
        "SELECT coalesce((SELECT last FROM invoice_counters WHERE key = ?1), 0)",
        [key(&parts, template, issue_date)],
        |r| r.get(0),
    )?;
    let mut n = last + 1;
    loop {
        let number = render(&parts, issue_date, n);
        if !taken(conn, &number)? {
            return Ok(number);
        }
        n += 1;
    }
}

/// Takes the next free number for an invoice issued on `issue_date`.
pub fn next(conn: &Connection, template: &str, issue_date: &str) -> CoreResult<String> {
    let parts = checked(template, issue_date)?;
    let key = key(&parts, template, issue_date);
    loop {
        // One statement: atomic even with other connections writing.
        let n: i64 = conn.query_row(
            "INSERT INTO invoice_counters (key, last) VALUES (?1, 1)
             ON CONFLICT (key) DO UPDATE SET last = last + 1
             RETURNING last",
            [&key],
            |r| r.get(0),
        )?;
        let number = render(&parts, issue_date, n);
        if !taken(conn, &number)? {
            return Ok(number);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    #[test]
    fn validates_templates() {
        assert!(validate_template(DEFAULT_TEMPLATE).is_ok());
        assert!(validate_template("{YY}{MM}-{NN}").is_ok());
        assert!(validate_template("{N}").is_ok());
        let invalid = CoreError::validation("invoiceNumberTemplate", "template");
        assert_eq!(validate_template("INV-{YYYY}").unwrap_err(), invalid);
        assert_eq!(validate_template("{NN}-{NNN}").unwrap_err(), invalid);
        assert_eq!(validate_template("{DD}-{NNN}").unwrap_err(), invalid);
        assert_eq!(validate_template("INV-{NNN").unwrap_err(), invalid);
        assert_eq!(validate_template("").unwrap_err(), invalid);
    }

    #[test]
    fn numbers_count_up_and_restart_each_year() {
        let db = Db::open_in_memory().unwrap();
        let next = |date| next(db.conn(), DEFAULT_TEMPLATE, date).unwrap();
        assert_eq!(
            preview(db.conn(), DEFAULT_TEMPLATE, "2026-03-01").unwrap(),
            "INV-2026-001"
        );
        assert_eq!(next("2026-03-01"), "INV-2026-001");
        assert_eq!(next("2026-12-31"), "INV-2026-002");
        assert_eq!(next("2027-01-02"), "INV-2027-001");
        assert_eq!(next("2026-06-01"), "INV-2026-003");
        // The counter grows past its width rather than wrapping.
        for _ in 0..996 {
            next("2028-01-01");
        }
        assert_eq!(next("2028-01-01"), "INV-2028-997");
        next("2028-01-01");
        next("2028-01-01");
        assert_eq!(next("2028-01-01"), "INV-2028-1000");
    }

    #[test]
    fn monthly_and_endless_counters() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(
            next(db.conn(), "{YY}{MM}-{NN}", "2026-03-15").unwrap(),
            "2603-01"
        );
        assert_eq!(
            next(db.conn(), "{YY}{MM}-{NN}", "2026-03-20").unwrap(),
            "2603-02"
        );
        assert_eq!(
            next(db.conn(), "{YY}{MM}-{NN}", "2026-04-01").unwrap(),
            "2604-01"
        );
        assert_eq!(next(db.conn(), "№{N}", "2026-01-01").unwrap(), "№1");
        assert_eq!(next(db.conn(), "№{N}", "2030-01-01").unwrap(), "№2");
    }

    #[test]
    fn numbers_taken_by_hand_are_skipped() {
        let db = Db::open_in_memory().unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO clients (id, kind, display_name, sort_key, status, created_at, updated_at)
                 VALUES ('01900000-0000-7000-8000-000000000001', 'company', 'X', 'x', 'active', 0, 0);
                 INSERT INTO invoices (id, number, client_id, currency, issue_date, status,
                     discount_kind, language, created_at, updated_at)
                 VALUES ('01900000-0000-7000-8000-000000000002', 'INV-2026-002',
                     '01900000-0000-7000-8000-000000000001', 'RUB', '2026-01-01', 'draft', 'none',
                     'en', 0, 0);",
            )
            .unwrap();
        assert_eq!(
            next(db.conn(), DEFAULT_TEMPLATE, "2026-03-01").unwrap(),
            "INV-2026-001"
        );
        assert_eq!(
            preview(db.conn(), DEFAULT_TEMPLATE, "2026-03-01").unwrap(),
            "INV-2026-003"
        );
        assert_eq!(
            next(db.conn(), DEFAULT_TEMPLATE, "2026-03-01").unwrap(),
            "INV-2026-003"
        );
    }

    #[test]
    fn concurrent_connections_never_share_a_number() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("crm.db");
        drop(Db::open(&path).unwrap());
        let workers: Vec<_> = (0..4)
            .map(|_| {
                let path = path.clone();
                std::thread::spawn(move || {
                    let db = Db::open(&path).unwrap();
                    (0..25)
                        .map(|_| next(db.conn(), DEFAULT_TEMPLATE, "2026-03-01").unwrap())
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let mut all: Vec<String> = workers
            .into_iter()
            .flat_map(|w| w.join().unwrap())
            .collect();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 100);
        assert_eq!(all.last().unwrap(), "INV-2026-100");
    }
}
