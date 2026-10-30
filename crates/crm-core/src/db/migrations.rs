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
    // 11: case-folded task titles for search
    "ALTER TABLE tasks ADD COLUMN title_key TEXT NOT NULL DEFAULT '';
    UPDATE tasks SET title_key = lower(title);",
    // 12: checklist items of tasks
    "CREATE TABLE checklist_items (
        id         TEXT PRIMARY KEY NOT NULL,
        task_id    TEXT NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
        text       TEXT NOT NULL,
        done       INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1)),
        position   INTEGER NOT NULL,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX checklist_items_by_task ON checklist_items (task_id, position);",
    // 13: vault metadata (singleton): KDF parameters, salt and the wrapped DEK
    "CREATE TABLE vault_meta (
        id           INTEGER PRIMARY KEY CHECK (id = 1),
        version      INTEGER NOT NULL,
        kdf_m_kib    INTEGER NOT NULL,
        kdf_t        INTEGER NOT NULL,
        kdf_p        INTEGER NOT NULL,
        salt         BLOB NOT NULL,
        wrap_nonce   BLOB NOT NULL,
        wrapped_dek  BLOB NOT NULL,
        created_at   INTEGER NOT NULL,
        updated_at   INTEGER NOT NULL
    ) STRICT;",
    // 14: secrets; only the payload (all field values) is encrypted. A
    // project secret carries the project's client, so client deletion
    // cascades to it.
    "CREATE TABLE secrets (
        id          TEXT PRIMARY KEY NOT NULL,
        title       TEXT NOT NULL,
        title_key   TEXT NOT NULL,
        kind        TEXT NOT NULL CHECK (kind IN ('login', 'ssh_key', 'api_token', 'database',
                        'server', 'ftp', 'domain', 'env', 'bank_card', 'custom')),
        client_id   TEXT REFERENCES clients (id) ON DELETE CASCADE,
        project_id  TEXT REFERENCES projects (id) ON DELETE CASCADE,
        expires_on  TEXT,
        nonce       BLOB NOT NULL,
        ciphertext  BLOB NOT NULL,
        key_version INTEGER NOT NULL,
        created_at  INTEGER NOT NULL,
        updated_at  INTEGER NOT NULL,
        CHECK (project_id IS NULL OR client_id IS NOT NULL)
    ) STRICT;
    CREATE INDEX secrets_by_client ON secrets (client_id);
    CREATE INDEX secrets_by_project ON secrets (project_id);
    CREATE TRIGGER secrets_delete_owned AFTER DELETE ON secrets BEGIN
        DELETE FROM notes WHERE owner_type = 'secret' AND owner_id = old.id;
        DELETE FROM taggings WHERE owner_type = 'secret' AND owner_id = old.id;
    END;",
    // 15: the DEK wrapped a second time by the recovery key (never stored)
    "ALTER TABLE vault_meta ADD COLUMN recovery_nonce BLOB;
    ALTER TABLE vault_meta ADD COLUMN recovery_wrapped BLOB;",
    // 16: stored files; plain blobs are keyed by their SHA-256
    "CREATE TABLE blobs (
        id         TEXT PRIMARY KEY NOT NULL,
        sha256     TEXT NOT NULL,
        size       INTEGER NOT NULL CHECK (size >= 0),
        mime       TEXT NOT NULL,
        encrypted  INTEGER NOT NULL DEFAULT 0 CHECK (encrypted IN (0, 1)),
        ref_count  INTEGER NOT NULL CHECK (ref_count >= 0),
        created_at INTEGER NOT NULL
    ) STRICT;",
    // 17: files attached to records; each attachment holds one blob reference
    "CREATE TABLE attachments (
        id         TEXT PRIMARY KEY NOT NULL,
        owner_type TEXT NOT NULL CHECK (owner_type IN ('client', 'contact', 'project', 'task',
                       'event', 'interaction', 'invoice', 'expense', 'secret')),
        owner_id   TEXT NOT NULL,
        blob_id    TEXT NOT NULL REFERENCES blobs (id),
        file_name  TEXT NOT NULL,
        mime       TEXT NOT NULL,
        created_at INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX attachments_by_owner ON attachments (owner_type, owner_id, created_at DESC);
    CREATE INDEX attachments_by_blob ON attachments (blob_id);
    CREATE TRIGGER attachments_release_blob AFTER DELETE ON attachments BEGIN
        UPDATE blobs SET ref_count = ref_count - 1 WHERE id = old.blob_id;
    END;
    CREATE TRIGGER clients_delete_attachments AFTER DELETE ON clients BEGIN
        DELETE FROM attachments WHERE owner_type = 'client' AND owner_id = old.id;
    END;
    CREATE TRIGGER contacts_delete_attachments AFTER DELETE ON contacts BEGIN
        DELETE FROM attachments WHERE owner_type = 'contact' AND owner_id = old.id;
    END;
    CREATE TRIGGER projects_delete_attachments AFTER DELETE ON projects BEGIN
        DELETE FROM attachments WHERE owner_type = 'project' AND owner_id = old.id;
    END;
    CREATE TRIGGER tasks_delete_attachments AFTER DELETE ON tasks BEGIN
        DELETE FROM attachments WHERE owner_type = 'task' AND owner_id = old.id;
    END;
    CREATE TRIGGER secrets_delete_attachments AFTER DELETE ON secrets BEGIN
        DELETE FROM attachments WHERE owner_type = 'secret' AND owner_id = old.id;
    END;",
    // 18: image thumbnails, one blob reference each
    "ALTER TABLE attachments ADD COLUMN thumb_blob_id TEXT REFERENCES blobs (id);
    CREATE TRIGGER attachments_release_thumb AFTER DELETE ON attachments
    WHEN old.thumb_blob_id IS NOT NULL BEGIN
        UPDATE blobs SET ref_count = ref_count - 1 WHERE id = old.thumb_blob_id;
    END;",
    // 19: calendar events; local wall-clock values plus UTC instants
    "CREATE TABLE events (
        id             TEXT PRIMARY KEY NOT NULL,
        uid            TEXT NOT NULL UNIQUE,
        title          TEXT NOT NULL,
        description_md TEXT NOT NULL DEFAULT '',
        location       TEXT,
        all_day        INTEGER NOT NULL DEFAULT 0 CHECK (all_day IN (0, 1)),
        start_local    TEXT NOT NULL,
        end_local      TEXT NOT NULL,
        tzid           TEXT NOT NULL,
        start_at       INTEGER NOT NULL,
        end_at         INTEGER NOT NULL CHECK (end_at >= start_at),
        client_id      TEXT REFERENCES clients (id) ON DELETE CASCADE,
        project_id     TEXT REFERENCES projects (id) ON DELETE CASCADE,
        task_id        TEXT REFERENCES tasks (id) ON DELETE SET NULL,
        contact_id     TEXT REFERENCES contacts (id) ON DELETE SET NULL,
        created_at     INTEGER NOT NULL,
        updated_at     INTEGER NOT NULL,
        CHECK (project_id IS NULL OR client_id IS NOT NULL)
    ) STRICT;
    CREATE INDEX events_by_time ON events (start_at, end_at);
    CREATE INDEX events_by_client ON events (client_id);
    CREATE INDEX events_by_project ON events (project_id);
    CREATE TRIGGER events_delete_owned AFTER DELETE ON events BEGIN
        DELETE FROM notes WHERE owner_type = 'event' AND owner_id = old.id;
        DELETE FROM attachments WHERE owner_type = 'event' AND owner_id = old.id;
    END;",
    // 20: recurrence; `series_end_at` is NULL for endless series
    "ALTER TABLE events ADD COLUMN rrule TEXT;
    ALTER TABLE events ADD COLUMN exdates TEXT NOT NULL DEFAULT '[]';
    ALTER TABLE events ADD COLUMN series_end_at INTEGER;",
    // 21: edited occurrences of a series (iCalendar RECURRENCE-ID)
    "ALTER TABLE events ADD COLUMN series_id TEXT REFERENCES events (id) ON DELETE CASCADE;
    ALTER TABLE events ADD COLUMN recurrence_id TEXT;
    CREATE UNIQUE INDEX events_overrides ON events (series_id, recurrence_id)
        WHERE series_id IS NOT NULL;",
    // 22: reminders, N minutes before an event occurrence or a deadline
    "CREATE TABLE reminders (
        id             TEXT PRIMARY KEY NOT NULL,
        owner_type     TEXT NOT NULL CHECK (owner_type IN ('event', 'task')),
        owner_id       TEXT NOT NULL,
        minutes_before INTEGER NOT NULL CHECK (minutes_before >= 0),
        created_at     INTEGER NOT NULL,
        UNIQUE (owner_type, owner_id, minutes_before)
    ) STRICT;
    CREATE TRIGGER events_delete_reminders AFTER DELETE ON events BEGIN
        DELETE FROM reminders WHERE owner_type = 'event' AND owner_id = old.id;
    END;
    CREATE TRIGGER tasks_delete_reminders AFTER DELETE ON tasks BEGIN
        DELETE FROM reminders WHERE owner_type = 'task' AND owner_id = old.id;
    END;",
    // 23: handled reminders (no repeats across restarts) and the last check
    "CREATE TABLE reminder_log (
        reminder_id TEXT NOT NULL REFERENCES reminders (id) ON DELETE CASCADE,
        occurrence  TEXT NOT NULL,
        fire_at     INTEGER NOT NULL,
        owner_type  TEXT NOT NULL,
        owner_id    TEXT NOT NULL,
        title       TEXT NOT NULL,
        start_at    INTEGER NOT NULL,
        all_day     INTEGER NOT NULL,
        fired_at    INTEGER NOT NULL,
        missed      INTEGER NOT NULL CHECK (missed IN (0, 1)),
        seen        INTEGER NOT NULL DEFAULT 0 CHECK (seen IN (0, 1)),
        PRIMARY KEY (reminder_id, occurrence, fire_at)
    ) STRICT;
    CREATE INDEX reminder_log_unseen ON reminder_log (missed, seen);
    CREATE TABLE reminder_state (
        id         INTEGER PRIMARY KEY CHECK (id = 1),
        last_check INTEGER NOT NULL
    ) STRICT;",
    // 24: the calendar event of a secret's renewal date
    "ALTER TABLE secrets ADD COLUMN renewal_event_id TEXT
        REFERENCES events (id) ON DELETE SET NULL;
    CREATE TRIGGER secrets_delete_renewal AFTER DELETE ON secrets
    WHEN old.renewal_event_id IS NOT NULL BEGIN
        DELETE FROM events WHERE id = old.renewal_event_id;
    END;",
    // 25: time entries; the one without an end is the running timer
    "CREATE TABLE time_entries (
        id          TEXT PRIMARY KEY NOT NULL,
        task_id     TEXT REFERENCES tasks (id) ON DELETE SET NULL,
        project_id  TEXT REFERENCES projects (id) ON DELETE CASCADE,
        client_id   TEXT REFERENCES clients (id) ON DELETE CASCADE,
        description TEXT NOT NULL DEFAULT '',
        started_at  INTEGER NOT NULL,
        ended_at    INTEGER CHECK (ended_at IS NULL OR ended_at >= started_at),
        billable    INTEGER NOT NULL DEFAULT 1 CHECK (billable IN (0, 1)),
        invoice_id  TEXT,
        created_at  INTEGER NOT NULL,
        updated_at  INTEGER NOT NULL,
        CHECK (project_id IS NULL OR client_id IS NOT NULL)
    ) STRICT;
    CREATE UNIQUE INDEX time_entries_one_running ON time_entries (ended_at IS NULL)
        WHERE ended_at IS NULL;
    CREATE INDEX time_entries_by_start ON time_entries (started_at);
    CREATE INDEX time_entries_by_project ON time_entries (project_id);
    CREATE INDEX time_entries_by_client ON time_entries (client_id);",
    // 26: history of interactions with clients
    "CREATE TABLE interactions (
        id          TEXT PRIMARY KEY NOT NULL,
        client_id   TEXT NOT NULL REFERENCES clients (id) ON DELETE CASCADE,
        contact_id  TEXT REFERENCES contacts (id) ON DELETE SET NULL,
        project_id  TEXT REFERENCES projects (id) ON DELETE SET NULL,
        kind        TEXT NOT NULL
                    CHECK (kind IN ('call', 'meeting', 'email', 'message', 'other')),
        occurred_at INTEGER NOT NULL,
        summary_md  TEXT NOT NULL,
        created_at  INTEGER NOT NULL,
        updated_at  INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX interactions_feed ON interactions (client_id, occurred_at DESC, id DESC);
    CREATE TRIGGER interactions_delete_attachments AFTER DELETE ON interactions BEGIN
        DELETE FROM attachments WHERE owner_type = 'interaction' AND owner_id = old.id;
    END;",
    // 27: tasks and events created from an interaction
    "CREATE TABLE interaction_follow_ups (
        interaction_id TEXT NOT NULL REFERENCES interactions (id) ON DELETE CASCADE,
        kind           TEXT NOT NULL CHECK (kind IN ('task', 'event')),
        owner_id       TEXT NOT NULL,
        created_at     INTEGER NOT NULL,
        PRIMARY KEY (kind, owner_id)
    ) STRICT;
    CREATE INDEX interaction_follow_ups_by_interaction
        ON interaction_follow_ups (interaction_id, created_at);
    CREATE TRIGGER tasks_delete_follow_up AFTER DELETE ON tasks BEGIN
        DELETE FROM interaction_follow_ups WHERE kind = 'task' AND owner_id = old.id;
    END;
    CREATE TRIGGER events_delete_follow_up AFTER DELETE ON events BEGIN
        DELETE FROM interaction_follow_ups WHERE kind = 'event' AND owner_id = old.id;
    END;",
    // 28: invoices, their items and payments (to an invoice or a project)
    "CREATE TABLE invoices (
        id             TEXT PRIMARY KEY NOT NULL,
        number         TEXT NOT NULL UNIQUE,
        client_id      TEXT NOT NULL REFERENCES clients (id) ON DELETE RESTRICT,
        project_id     TEXT REFERENCES projects (id) ON DELETE SET NULL,
        currency       TEXT NOT NULL,
        issue_date     TEXT NOT NULL,
        due_date       TEXT CHECK (due_date IS NULL OR due_date >= issue_date),
        status         TEXT NOT NULL CHECK (status IN ('draft', 'sent', 'cancelled')),
        discount_kind  TEXT NOT NULL CHECK (discount_kind IN ('none', 'percent', 'fixed')),
        discount_value INTEGER NOT NULL DEFAULT 0 CHECK (discount_value >= 0),
        tax_bp         INTEGER NOT NULL DEFAULT 0 CHECK (tax_bp BETWEEN 0 AND 10000),
        notes_md       TEXT NOT NULL DEFAULT '',
        language       TEXT NOT NULL CHECK (language IN ('en', 'ru')),
        created_at     INTEGER NOT NULL,
        updated_at     INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX invoices_by_client ON invoices (client_id, issue_date);
    CREATE TABLE invoice_items (
        id             TEXT PRIMARY KEY NOT NULL,
        invoice_id     TEXT NOT NULL REFERENCES invoices (id) ON DELETE CASCADE,
        position       INTEGER NOT NULL,
        description    TEXT NOT NULL,
        quantity_milli INTEGER NOT NULL CHECK (quantity_milli > 0),
        unit           TEXT NOT NULL DEFAULT '',
        price_minor    INTEGER NOT NULL CHECK (price_minor >= 0)
    ) STRICT;
    CREATE INDEX invoice_items_by_invoice ON invoice_items (invoice_id, position);
    CREATE TABLE payments (
        id           TEXT PRIMARY KEY NOT NULL,
        invoice_id   TEXT REFERENCES invoices (id) ON DELETE RESTRICT,
        project_id   TEXT REFERENCES projects (id) ON DELETE SET NULL,
        client_id    TEXT NOT NULL REFERENCES clients (id) ON DELETE RESTRICT,
        amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
        currency     TEXT NOT NULL,
        paid_on      TEXT NOT NULL,
        method       TEXT NOT NULL DEFAULT '',
        note         TEXT NOT NULL DEFAULT '',
        created_at   INTEGER NOT NULL,
        updated_at   INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX payments_by_invoice ON payments (invoice_id);
    CREATE INDEX payments_by_date ON payments (paid_on);",
    // 29: invoice number counters per template and period
    "CREATE TABLE invoice_counters (
        key  TEXT PRIMARY KEY NOT NULL,
        last INTEGER NOT NULL
    ) STRICT;",
    // 30: expenses; `category_key` groups categories regardless of case
    "CREATE TABLE expenses (
        id           TEXT PRIMARY KEY NOT NULL,
        project_id   TEXT REFERENCES projects (id) ON DELETE SET NULL,
        category     TEXT NOT NULL,
        category_key TEXT NOT NULL,
        amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
        currency     TEXT NOT NULL,
        spent_on     TEXT NOT NULL,
        description  TEXT NOT NULL DEFAULT '',
        created_at   INTEGER NOT NULL,
        updated_at   INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX expenses_by_date ON expenses (spent_on);
    CREATE INDEX expenses_by_project ON expenses (project_id);
    CREATE TRIGGER expenses_delete_attachments AFTER DELETE ON expenses BEGIN
        DELETE FROM attachments WHERE owner_type = 'expense' AND owner_id = old.id;
    END;",
    // 31: invoice documents: profile images, requisites snapshot, signature
    "CREATE TABLE profile_images (
        kind       TEXT PRIMARY KEY NOT NULL CHECK (kind IN ('logo', 'signature', 'stamp')),
        mime       TEXT NOT NULL,
        data       BLOB NOT NULL,
        updated_at INTEGER NOT NULL
    ) STRICT;
    ALTER TABLE invoices ADD COLUMN snapshot TEXT;
    ALTER TABLE invoices ADD COLUMN show_signature INTEGER
        CHECK (show_signature IS NULL OR show_signature IN (0, 1));",
    // 32: tax regimes, each in force from its date; rules as JSON
    "CREATE TABLE tax_regimes (
        id         TEXT PRIMARY KEY NOT NULL,
        name       TEXT NOT NULL,
        preset     TEXT NOT NULL,
        valid_from TEXT NOT NULL UNIQUE,
        rules      TEXT NOT NULL,
        created_at INTEGER NOT NULL
    ) STRICT;",
    // 33: reminders of invoice due dates. SQLite cannot change a CHECK, so
    // the table is rebuilt; the log that references it is set aside first
    // (dropping `reminders` would cascade into it), and the triggers that
    // name it are recreated.
    "CREATE TEMP TABLE reminder_log_copy AS SELECT * FROM reminder_log;
    DROP TABLE reminder_log;
    DROP TRIGGER events_delete_reminders;
    DROP TRIGGER tasks_delete_reminders;
    CREATE TABLE reminders_new (
        id             TEXT PRIMARY KEY NOT NULL,
        owner_type     TEXT NOT NULL CHECK (owner_type IN ('event', 'task', 'invoice')),
        owner_id       TEXT NOT NULL,
        minutes_before INTEGER NOT NULL CHECK (minutes_before >= 0),
        created_at     INTEGER NOT NULL,
        UNIQUE (owner_type, owner_id, minutes_before)
    ) STRICT;
    INSERT INTO reminders_new SELECT * FROM reminders;
    DROP TABLE reminders;
    ALTER TABLE reminders_new RENAME TO reminders;
    CREATE TRIGGER events_delete_reminders AFTER DELETE ON events BEGIN
        DELETE FROM reminders WHERE owner_type = 'event' AND owner_id = old.id;
    END;
    CREATE TRIGGER tasks_delete_reminders AFTER DELETE ON tasks BEGIN
        DELETE FROM reminders WHERE owner_type = 'task' AND owner_id = old.id;
    END;
    CREATE TRIGGER invoices_delete_reminders AFTER DELETE ON invoices BEGIN
        DELETE FROM reminders WHERE owner_type = 'invoice' AND owner_id = old.id;
    END;
    CREATE TABLE reminder_log (
        reminder_id TEXT NOT NULL REFERENCES reminders (id) ON DELETE CASCADE,
        occurrence  TEXT NOT NULL,
        fire_at     INTEGER NOT NULL,
        owner_type  TEXT NOT NULL,
        owner_id    TEXT NOT NULL,
        title       TEXT NOT NULL,
        start_at    INTEGER NOT NULL,
        all_day     INTEGER NOT NULL,
        fired_at    INTEGER NOT NULL,
        missed      INTEGER NOT NULL CHECK (missed IN (0, 1)),
        seen        INTEGER NOT NULL DEFAULT 0 CHECK (seen IN (0, 1)),
        PRIMARY KEY (reminder_id, occurrence, fire_at)
    ) STRICT;
    INSERT INTO reminder_log SELECT * FROM temp.reminder_log_copy;
    DROP TABLE temp.reminder_log_copy;
    CREATE INDEX reminder_log_unseen ON reminder_log (missed, seen);",
    // 34: global search (FR-SR-1, 2). Documents live in `search_docs`,
    // keyed by kind and id so that triggers find them without scanning;
    // `search_index` is FTS5 over their titles and text, with "ё" as "е"
    // (`remove_diacritics` only covers Latin letters). Secrets give only
    // their title, never their encrypted values.
    "CREATE TABLE search_docs (
        rowid      INTEGER PRIMARY KEY,
        kind       TEXT NOT NULL,
        id         TEXT NOT NULL,
        owner_type TEXT,
        owner_id   TEXT,
        title      TEXT NOT NULL,
        body       TEXT NOT NULL,
        UNIQUE (kind, id)
    ) STRICT;
    CREATE VIRTUAL TABLE search_index USING fts5(
        title, body,
        content = 'search_docs', content_rowid = 'rowid',
        tokenize = 'unicode61 remove_diacritics 2'
    );
    CREATE TRIGGER search_docs_insert AFTER INSERT ON search_docs BEGIN
        INSERT INTO search_index (rowid, title, body) VALUES (new.rowid,
            replace(replace(new.title, 'ё', 'е'), 'Ё', 'Е'),
            replace(replace(new.body, 'ё', 'е'), 'Ё', 'Е'));
    END;
    CREATE TRIGGER search_docs_delete AFTER DELETE ON search_docs BEGIN
        INSERT INTO search_index (search_index, rowid, title, body) VALUES ('delete', old.rowid,
            replace(replace(old.title, 'ё', 'е'), 'Ё', 'Е'),
            replace(replace(old.body, 'ё', 'е'), 'Ё', 'Е'));
    END;
    CREATE TRIGGER clients_search_insert AFTER INSERT ON clients BEGIN
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('client', new.id, NULL, NULL, new.display_name, coalesce(new.legal_name, ''));
    END;
    CREATE TRIGGER clients_search_update AFTER UPDATE ON clients BEGIN
        DELETE FROM search_docs WHERE kind = 'client' AND id = old.id;
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('client', new.id, NULL, NULL, new.display_name, coalesce(new.legal_name, ''));
    END;
    CREATE TRIGGER clients_search_delete AFTER DELETE ON clients BEGIN
        DELETE FROM search_docs WHERE kind = 'client' AND id = old.id;
    END;
    INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) SELECT 'client', r.id, NULL, NULL, r.display_name, coalesce(r.legal_name, '') FROM clients r;
    CREATE TRIGGER contacts_search_insert AFTER INSERT ON contacts BEGIN
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('contact', new.id, 'client', new.client_id, new.display_name, trim(coalesce(new.position, '') || ' ' || coalesce(new.note, '')));
    END;
    CREATE TRIGGER contacts_search_update AFTER UPDATE ON contacts BEGIN
        DELETE FROM search_docs WHERE kind = 'contact' AND id = old.id;
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('contact', new.id, 'client', new.client_id, new.display_name, trim(coalesce(new.position, '') || ' ' || coalesce(new.note, '')));
    END;
    CREATE TRIGGER contacts_search_delete AFTER DELETE ON contacts BEGIN
        DELETE FROM search_docs WHERE kind = 'contact' AND id = old.id;
    END;
    INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) SELECT 'contact', r.id, 'client', r.client_id, r.display_name, trim(coalesce(r.position, '') || ' ' || coalesce(r.note, '')) FROM contacts r;
    CREATE TRIGGER projects_search_insert AFTER INSERT ON projects BEGIN
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('project', new.id, NULL, NULL, new.name, new.description_md);
    END;
    CREATE TRIGGER projects_search_update AFTER UPDATE ON projects BEGIN
        DELETE FROM search_docs WHERE kind = 'project' AND id = old.id;
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('project', new.id, NULL, NULL, new.name, new.description_md);
    END;
    CREATE TRIGGER projects_search_delete AFTER DELETE ON projects BEGIN
        DELETE FROM search_docs WHERE kind = 'project' AND id = old.id;
    END;
    INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) SELECT 'project', r.id, NULL, NULL, r.name, r.description_md FROM projects r;
    CREATE TRIGGER tasks_search_insert AFTER INSERT ON tasks BEGIN
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('task', new.id, NULL, NULL, new.title, new.description_md);
    END;
    CREATE TRIGGER tasks_search_update AFTER UPDATE ON tasks BEGIN
        DELETE FROM search_docs WHERE kind = 'task' AND id = old.id;
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('task', new.id, NULL, NULL, new.title, new.description_md);
    END;
    CREATE TRIGGER tasks_search_delete AFTER DELETE ON tasks BEGIN
        DELETE FROM search_docs WHERE kind = 'task' AND id = old.id;
    END;
    INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) SELECT 'task', r.id, NULL, NULL, r.title, r.description_md FROM tasks r;
    CREATE TRIGGER events_search_insert AFTER INSERT ON events BEGIN
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('event', new.id, NULL, NULL, new.title, trim(new.description_md || ' ' || coalesce(new.location, '')));
    END;
    CREATE TRIGGER events_search_update AFTER UPDATE ON events BEGIN
        DELETE FROM search_docs WHERE kind = 'event' AND id = old.id;
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('event', new.id, NULL, NULL, new.title, trim(new.description_md || ' ' || coalesce(new.location, '')));
    END;
    CREATE TRIGGER events_search_delete AFTER DELETE ON events BEGIN
        DELETE FROM search_docs WHERE kind = 'event' AND id = old.id;
    END;
    INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) SELECT 'event', r.id, NULL, NULL, r.title, trim(r.description_md || ' ' || coalesce(r.location, '')) FROM events r;
    CREATE TRIGGER notes_search_insert AFTER INSERT ON notes BEGIN
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('note', new.id, new.owner_type, new.owner_id, trim(CASE WHEN instr(new.body_md, char(10)) > 0 THEN substr(new.body_md, 1, instr(new.body_md, char(10)) - 1) ELSE new.body_md END), new.body_md);
    END;
    CREATE TRIGGER notes_search_update AFTER UPDATE ON notes BEGIN
        DELETE FROM search_docs WHERE kind = 'note' AND id = old.id;
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('note', new.id, new.owner_type, new.owner_id, trim(CASE WHEN instr(new.body_md, char(10)) > 0 THEN substr(new.body_md, 1, instr(new.body_md, char(10)) - 1) ELSE new.body_md END), new.body_md);
    END;
    CREATE TRIGGER notes_search_delete AFTER DELETE ON notes BEGIN
        DELETE FROM search_docs WHERE kind = 'note' AND id = old.id;
    END;
    INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) SELECT 'note', r.id, r.owner_type, r.owner_id, trim(CASE WHEN instr(r.body_md, char(10)) > 0 THEN substr(r.body_md, 1, instr(r.body_md, char(10)) - 1) ELSE r.body_md END), r.body_md FROM notes r;
    CREATE TRIGGER interactions_search_insert AFTER INSERT ON interactions BEGIN
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('interaction', new.id, 'client', new.client_id, trim(CASE WHEN instr(new.summary_md, char(10)) > 0 THEN substr(new.summary_md, 1, instr(new.summary_md, char(10)) - 1) ELSE new.summary_md END), new.summary_md);
    END;
    CREATE TRIGGER interactions_search_update AFTER UPDATE ON interactions BEGIN
        DELETE FROM search_docs WHERE kind = 'interaction' AND id = old.id;
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('interaction', new.id, 'client', new.client_id, trim(CASE WHEN instr(new.summary_md, char(10)) > 0 THEN substr(new.summary_md, 1, instr(new.summary_md, char(10)) - 1) ELSE new.summary_md END), new.summary_md);
    END;
    CREATE TRIGGER interactions_search_delete AFTER DELETE ON interactions BEGIN
        DELETE FROM search_docs WHERE kind = 'interaction' AND id = old.id;
    END;
    INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) SELECT 'interaction', r.id, 'client', r.client_id, trim(CASE WHEN instr(r.summary_md, char(10)) > 0 THEN substr(r.summary_md, 1, instr(r.summary_md, char(10)) - 1) ELSE r.summary_md END), r.summary_md FROM interactions r;
    CREATE TRIGGER custom_fields_search_insert AFTER INSERT ON custom_fields BEGIN
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('field', new.id, new.owner_type, new.owner_id, new.key, new.value);
    END;
    CREATE TRIGGER custom_fields_search_update AFTER UPDATE ON custom_fields BEGIN
        DELETE FROM search_docs WHERE kind = 'field' AND id = old.id;
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('field', new.id, new.owner_type, new.owner_id, new.key, new.value);
    END;
    CREATE TRIGGER custom_fields_search_delete AFTER DELETE ON custom_fields BEGIN
        DELETE FROM search_docs WHERE kind = 'field' AND id = old.id;
    END;
    INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) SELECT 'field', r.id, r.owner_type, r.owner_id, r.key, r.value FROM custom_fields r;
    CREATE TRIGGER attachments_search_insert AFTER INSERT ON attachments BEGIN
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('file', new.id, new.owner_type, new.owner_id, new.file_name, '');
    END;
    CREATE TRIGGER attachments_search_update AFTER UPDATE ON attachments BEGIN
        DELETE FROM search_docs WHERE kind = 'file' AND id = old.id;
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('file', new.id, new.owner_type, new.owner_id, new.file_name, '');
    END;
    CREATE TRIGGER attachments_search_delete AFTER DELETE ON attachments BEGIN
        DELETE FROM search_docs WHERE kind = 'file' AND id = old.id;
    END;
    INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) SELECT 'file', r.id, r.owner_type, r.owner_id, r.file_name, '' FROM attachments r;
    CREATE TRIGGER secrets_search_insert AFTER INSERT ON secrets BEGIN
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('secret', new.id, NULL, NULL, new.title, '');
    END;
    CREATE TRIGGER secrets_search_update AFTER UPDATE ON secrets BEGIN
        DELETE FROM search_docs WHERE kind = 'secret' AND id = old.id;
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('secret', new.id, NULL, NULL, new.title, '');
    END;
    CREATE TRIGGER secrets_search_delete AFTER DELETE ON secrets BEGIN
        DELETE FROM search_docs WHERE kind = 'secret' AND id = old.id;
    END;
    INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) SELECT 'secret', r.id, NULL, NULL, r.title, '' FROM secrets r;
    CREATE TRIGGER invoices_search_insert AFTER INSERT ON invoices BEGIN
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('invoice', new.id, NULL, NULL, new.number, new.notes_md);
    END;
    CREATE TRIGGER invoices_search_update AFTER UPDATE ON invoices BEGIN
        DELETE FROM search_docs WHERE kind = 'invoice' AND id = old.id;
        INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) VALUES ('invoice', new.id, NULL, NULL, new.number, new.notes_md);
    END;
    CREATE TRIGGER invoices_search_delete AFTER DELETE ON invoices BEGIN
        DELETE FROM search_docs WHERE kind = 'invoice' AND id = old.id;
    END;
    INSERT INTO search_docs (kind, id, owner_type, owner_id, title, body) SELECT 'invoice', r.id, NULL, NULL, r.number, r.notes_md FROM invoices r;",
    // 35: automatic backups: the last success and attempt, the last error,
    // and the passphrase of vault-locked ones (as text for the scheduler,
    // and sealed with the DEK for their header)
    "CREATE TABLE backup_state (
        id               INTEGER PRIMARY KEY CHECK (id = 1),
        last_success_at  INTEGER,
        last_attempt_at  INTEGER,
        last_error       TEXT,
        vault_passphrase TEXT,
        vault_nonce      BLOB,
        vault_sealed     BLOB
    ) STRICT;
    INSERT INTO backup_state (id) VALUES (1);",
];

pub const LATEST_VERSION: u32 = STEPS.len() as u32;

pub fn migrations() -> Migrations<'static> {
    Migrations::new(STEPS.iter().map(|sql| M::up(sql)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn rebuilding_reminders_keeps_reminders_and_their_log() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations().to_version(&mut conn, 32).unwrap();
        conn.execute_batch(
            "INSERT INTO reminders VALUES ('r1', 'task', 't1', 15, 1);
             INSERT INTO reminder_log VALUES ('r1', '', 100, 'task', 't1', 'Отчёт', 200, 0,
                 150, 1, 0);",
        )
        .unwrap();
        migrations().to_latest(&mut conn).unwrap();
        let log: (String, String, bool) = conn
            .query_row(
                "SELECT l.reminder_id, l.title, l.missed FROM reminder_log l
                 JOIN reminders r ON r.id = l.reminder_id",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(log, ("r1".into(), "Отчёт".into(), true));
        // The log still follows its reminder.
        conn.execute("DELETE FROM reminders WHERE id = 'r1'", [])
            .unwrap();
        let left: i64 = conn
            .query_row("SELECT count(*) FROM reminder_log", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 0);
        // And invoices may have reminders now.
        conn.execute(
            "INSERT INTO reminders VALUES ('r2', 'invoice', 'i1', 0, 1)",
            [],
        )
        .unwrap();
    }
}
