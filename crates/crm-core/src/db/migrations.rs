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
    // 3: contact methods of clients and contact persons (polymorphic owner,
    // so cascades are done by triggers)
    "CREATE TABLE contact_methods (
        id         TEXT PRIMARY KEY NOT NULL,
        owner_type TEXT NOT NULL CHECK (owner_type IN ('client', 'contact')),
        owner_id   TEXT NOT NULL,
        kind       TEXT NOT NULL
                   CHECK (kind IN ('email', 'phone', 'telegram', 'whatsapp', 'website', 'other')),
        value      TEXT NOT NULL,
        label      TEXT,
        position   INTEGER NOT NULL,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX contact_methods_by_owner ON contact_methods (owner_type, owner_id, position);
    CREATE TRIGGER clients_delete_contact_methods AFTER DELETE ON clients BEGIN
        DELETE FROM contact_methods WHERE owner_type = 'client' AND owner_id = old.id;
    END;",
    // 4: global tags; name_key is the case-folded name (unique)
    "CREATE TABLE tags (
        id         TEXT PRIMARY KEY NOT NULL,
        name       TEXT NOT NULL,
        name_key   TEXT NOT NULL UNIQUE,
        color      TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    ) STRICT;
    CREATE TABLE taggings (
        tag_id     TEXT NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
        owner_type TEXT NOT NULL CHECK (owner_type IN ('client', 'project', 'task', 'secret')),
        owner_id   TEXT NOT NULL,
        PRIMARY KEY (tag_id, owner_type, owner_id)
    ) STRICT, WITHOUT ROWID;
    CREATE INDEX taggings_by_owner ON taggings (owner_type, owner_id);
    CREATE TRIGGER clients_delete_taggings AFTER DELETE ON clients BEGIN
        DELETE FROM taggings WHERE owner_type = 'client' AND owner_id = old.id;
    END;",
];

pub const LATEST_VERSION: u32 = STEPS.len() as u32;

pub fn migrations() -> Migrations<'static> {
    Migrations::new(STEPS.iter().map(|sql| M::up(sql)).collect())
}
