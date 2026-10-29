//! Versioned schema migrations. Append new steps to the end; never edit
//! a step that has already shipped.

use rusqlite_migration::{M, Migrations};

const STEPS: &[&str] = &[
    // 1: key-value application settings, values are JSON-encoded
    "CREATE TABLE settings (
        key        TEXT PRIMARY KEY NOT NULL,
        value      TEXT NOT NULL,
        updated_at INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000)
    ) STRICT;",
    // 2: clients (persons and companies)
    "CREATE TABLE clients (
        id                 TEXT PRIMARY KEY NOT NULL,
        kind               TEXT NOT NULL CHECK (kind IN ('person', 'company')),
        last_name          TEXT,
        first_name         TEXT,
        middle_name        TEXT,
        company_name       TEXT,
        legal_name         TEXT,
        display_name       TEXT NOT NULL,
        sort_key           TEXT NOT NULL,
        status             TEXT NOT NULL CHECK (status IN ('lead', 'active', 'paused', 'former')),
        default_currency   TEXT,
        default_rate_minor INTEGER CHECK (default_rate_minor >= 0),
        created_at         INTEGER NOT NULL,
        updated_at         INTEGER NOT NULL,
        archived_at        INTEGER
    ) STRICT;
    CREATE INDEX clients_by_name ON clients (archived_at IS NOT NULL, sort_key);",
];

pub const LATEST_VERSION: u32 = STEPS.len() as u32;

pub fn migrations() -> Migrations<'static> {
    Migrations::new(STEPS.iter().map(|sql| M::up(sql)).collect())
}
