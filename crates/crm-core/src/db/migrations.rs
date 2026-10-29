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
    // 5: user-defined fields of clients, contacts and projects
    "CREATE TABLE custom_fields (
        id         TEXT PRIMARY KEY NOT NULL,
        owner_type TEXT NOT NULL CHECK (owner_type IN ('client', 'contact', 'project')),
        owner_id   TEXT NOT NULL,
        key        TEXT NOT NULL,
        value      TEXT NOT NULL,
        field_type TEXT NOT NULL
                   CHECK (field_type IN ('text', 'url', 'email', 'phone', 'date', 'number')),
        position   INTEGER NOT NULL,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX custom_fields_by_owner ON custom_fields (owner_type, owner_id, position);
    CREATE TRIGGER clients_delete_custom_fields AFTER DELETE ON clients BEGIN
        DELETE FROM custom_fields WHERE owner_type = 'client' AND owner_id = old.id;
    END;",
    // 6: Markdown notes of any main entity
    "CREATE TABLE notes (
        id         TEXT PRIMARY KEY NOT NULL,
        owner_type TEXT NOT NULL CHECK (owner_type IN
                       ('client', 'contact', 'project', 'task', 'event', 'invoice', 'secret')),
        owner_id   TEXT NOT NULL,
        body_md    TEXT NOT NULL,
        pinned     INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX notes_by_owner ON notes (owner_type, owner_id, pinned DESC, updated_at DESC);
    CREATE TRIGGER clients_delete_notes AFTER DELETE ON clients BEGIN
        DELETE FROM notes WHERE owner_type = 'client' AND owner_id = old.id;
    END;",
    // 7: contact persons of company clients; at most one primary per client
    "CREATE TABLE contacts (
        id           TEXT PRIMARY KEY NOT NULL,
        client_id    TEXT NOT NULL REFERENCES clients (id) ON DELETE CASCADE,
        last_name    TEXT,
        first_name   TEXT,
        middle_name  TEXT,
        position     TEXT,
        note         TEXT,
        display_name TEXT NOT NULL,
        sort_key     TEXT NOT NULL,
        is_primary   INTEGER NOT NULL DEFAULT 0 CHECK (is_primary IN (0, 1)),
        created_at   INTEGER NOT NULL,
        updated_at   INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX contacts_by_client ON contacts (client_id, sort_key);
    CREATE UNIQUE INDEX contacts_one_primary ON contacts (client_id) WHERE is_primary = 1;
    CREATE TRIGGER contacts_delete_owned AFTER DELETE ON contacts BEGIN
        DELETE FROM contact_methods WHERE owner_type = 'contact' AND owner_id = old.id;
        DELETE FROM custom_fields WHERE owner_type = 'contact' AND owner_id = old.id;
        DELETE FROM notes WHERE owner_type = 'contact' AND owner_id = old.id;
    END;",
    // 8: requisites of the profile and of clients; `fields` is a JSON object
    "CREATE TABLE requisites (
        owner_type TEXT NOT NULL CHECK (owner_type IN ('profile', 'client')),
        owner_id   TEXT NOT NULL,
        country    TEXT NOT NULL,
        legal_form TEXT NOT NULL,
        name       TEXT,
        fields     TEXT NOT NULL,
        updated_at INTEGER NOT NULL,
        PRIMARY KEY (owner_type, owner_id)
    ) STRICT, WITHOUT ROWID;
    CREATE TRIGGER clients_delete_requisites AFTER DELETE ON clients BEGIN
        DELETE FROM requisites WHERE owner_type = 'client' AND owner_id = old.id;
    END;",
    // 9: projects of clients; dates are ISO `YYYY-MM-DD` text
    "CREATE TABLE projects (
        id             TEXT PRIMARY KEY NOT NULL,
        client_id      TEXT NOT NULL REFERENCES clients (id) ON DELETE CASCADE,
        name           TEXT NOT NULL,
        sort_key       TEXT NOT NULL,
        description_md TEXT NOT NULL DEFAULT '',
        status         TEXT NOT NULL
                       CHECK (status IN ('lead', 'active', 'paused', 'completed', 'cancelled')),
        billing_type   TEXT NOT NULL CHECK (billing_type IN ('fixed', 'hourly')),
        currency       TEXT NOT NULL,
        rate_minor     INTEGER CHECK (rate_minor >= 0),
        budget_minor   INTEGER CHECK (budget_minor >= 0),
        start_date     TEXT,
        deadline       TEXT CHECK (deadline IS NULL OR start_date IS NULL OR deadline >= start_date),
        created_at     INTEGER NOT NULL,
        updated_at     INTEGER NOT NULL,
        archived_at    INTEGER
    ) STRICT;
    CREATE INDEX projects_by_client ON projects (client_id, sort_key);
    CREATE INDEX projects_by_name ON projects (archived_at IS NOT NULL, sort_key);
    CREATE TRIGGER projects_delete_owned AFTER DELETE ON projects BEGIN
        DELETE FROM custom_fields WHERE owner_type = 'project' AND owner_id = old.id;
        DELETE FROM notes WHERE owner_type = 'project' AND owner_id = old.id;
        DELETE FROM taggings WHERE owner_type = 'project' AND owner_id = old.id;
    END;",
    // 10: kanban columns with four localized defaults, and tasks. Tasks of a
    // project always carry its client, so client cascades cover them too.
    "CREATE TABLE task_statuses (
        id        TEXT PRIMARY KEY NOT NULL,
        name      TEXT,
        name_key  TEXT,
        category  TEXT NOT NULL CHECK (category IN ('todo', 'in_progress', 'done')),
        color     TEXT NOT NULL,
        position  INTEGER NOT NULL,
        wip_limit INTEGER CHECK (wip_limit > 0),
        CHECK (name IS NOT NULL OR name_key IS NOT NULL)
    ) STRICT;
    INSERT INTO task_statuses (id, name_key, category, color, position) VALUES
        ('00000000-0000-7000-8000-000000000001', 'todo', 'todo', '#64748b', 0),
        ('00000000-0000-7000-8000-000000000002', 'in_progress', 'in_progress', '#3b82f6', 1),
        ('00000000-0000-7000-8000-000000000003', 'review', 'in_progress', '#f59e0b', 2),
        ('00000000-0000-7000-8000-000000000004', 'done', 'done', '#22c55e', 3);
    CREATE TABLE tasks (
        id               TEXT PRIMARY KEY NOT NULL,
        title            TEXT NOT NULL,
        description_md   TEXT NOT NULL DEFAULT '',
        status_id        TEXT NOT NULL REFERENCES task_statuses (id),
        priority         TEXT NOT NULL CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
        project_id       TEXT REFERENCES projects (id) ON DELETE CASCADE,
        client_id        TEXT REFERENCES clients (id) ON DELETE CASCADE,
        due_date         TEXT,
        due_time         TEXT CHECK (due_time IS NULL OR due_date IS NOT NULL),
        estimate_minutes INTEGER CHECK (estimate_minutes >= 0),
        board_position   TEXT NOT NULL,
        completed_at     INTEGER,
        created_at       INTEGER NOT NULL,
        updated_at       INTEGER NOT NULL,
        CHECK (project_id IS NULL OR client_id IS NOT NULL)
    ) STRICT;
    CREATE INDEX tasks_by_status ON tasks (status_id, board_position);
    CREATE INDEX tasks_by_project ON tasks (project_id);
    CREATE INDEX tasks_by_client ON tasks (client_id);
    CREATE INDEX tasks_by_due ON tasks (due_date);
    CREATE TRIGGER tasks_delete_owned AFTER DELETE ON tasks BEGIN
        DELETE FROM notes WHERE owner_type = 'task' AND owner_id = old.id;
        DELETE FROM taggings WHERE owner_type = 'task' AND owner_id = old.id;
    END;",
];

pub const LATEST_VERSION: u32 = STEPS.len() as u32;

pub fn migrations() -> Migrations<'static> {
    Migrations::new(STEPS.iter().map(|sql| M::up(sql)).collect())
}
