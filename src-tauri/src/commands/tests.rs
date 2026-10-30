//! Thin command tests: invoke through the real IPC handler of a mock app
//! and check that DTOs and errors cross the boundary as the frontend
//! expects. Business rules are tested in `crm_core`.

use std::sync::Arc;

use crm_core::vault::crypto::KdfParams;

use crm_core::db::Db;
use crm_core::domain::{FakeClock, Timestamp};
use serde_json::{Value, json};
use tauri::test::{
    INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets,
};
use tauri::webview::InvokeRequest;
use tauri::{App, Manager, WebviewWindow, WebviewWindowBuilder};

use crate::AppState;
use crate::clipboard::Clipboard;
use crate::desktop::Desktop;

const NOW: i64 = 1_700_000_000_000;

/// Records clipboard writes instead of touching the system clipboard.
#[derive(Clone, Default)]
struct RecordingClipboard(Arc<std::sync::Mutex<Vec<String>>>);

impl Clipboard for RecordingClipboard {
    fn write_text(&self, text: &str) -> Result<(), String> {
        self.0.lock().unwrap().push(text.to_owned());
        Ok(())
    }

    /// A 2×1 red image when the last written text is "image".
    fn read_image(&self) -> Result<Option<crate::clipboard::RgbaImage>, String> {
        let has_image = self.0.lock().unwrap().last().is_some_and(|t| t == "image");
        Ok(has_image.then(|| crate::clipboard::RgbaImage {
            width: 2,
            height: 1,
            rgba: [255, 0, 0, 255].repeat(2),
        }))
    }
}

impl RecordingClipboard {
    fn writes(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

/// Records what would be opened or revealed in the system.
#[derive(Clone, Default)]
struct RecordingDesktop(Arc<std::sync::Mutex<Vec<(String, std::path::PathBuf)>>>);

impl Desktop for RecordingDesktop {
    fn open_path(&self, path: &std::path::Path) -> Result<(), String> {
        self.0
            .lock()
            .unwrap()
            .push(("open".into(), path.to_owned()));
        Ok(())
    }

    fn reveal(&self, path: &std::path::Path) -> Result<(), String> {
        self.0
            .lock()
            .unwrap()
            .push(("reveal".into(), path.to_owned()));
        Ok(())
    }
}

struct SilentNotifier;

impl crate::notifier::Notifier for SilentNotifier {
    fn notify(&self, _: &str, _: &str) -> Result<(), String> {
        Ok(())
    }
}

struct TestApp {
    app: App<MockRuntime>,
    webview: WebviewWindow<MockRuntime>,
    clipboard: RecordingClipboard,
    desktop: RecordingDesktop,
    dir: tempfile::TempDir,
}

impl TestApp {
    fn new() -> Self {
        let clipboard = RecordingClipboard::default();
        let desktop = RecordingDesktop::default();
        let dir = tempfile::tempdir().unwrap();
        let app = mock_builder()
            .invoke_handler(crate::bindings::builder().invoke_handler())
            .manage(AppState::new(
                Db::open_in_memory().unwrap(),
                Arc::new(FakeClock::at(Timestamp::from_millis(NOW))),
                // Cheap key derivation keeps the tests fast.
                KdfParams {
                    m_kib: 64,
                    t: 1,
                    p: 1,
                },
                Box::new(clipboard.clone()),
                dir.path().to_owned(),
                Box::new(desktop.clone()),
                Box::new(SilentNotifier),
            ))
            .build(mock_context(noop_assets()))
            .unwrap();
        let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        Self {
            app,
            webview,
            clipboard,
            desktop,
            dir,
        }
    }

    fn unlock_vault(&self) {
        let state = self.app.state::<AppState>();
        let db = state.db.lock().unwrap();
        match state.vault.status(db.conn()).unwrap() {
            crm_core::vault::VaultStatus::Uninitialized => {
                state.vault.init(db.conn(), "master password").unwrap();
            }
            _ => state.vault.unlock(db.conn(), "master password").unwrap(),
        }
    }

    fn lock_vault(&self) {
        self.app.state::<AppState>().vault.lock();
    }

    fn invoke(&self, cmd: &str, args: Value) -> Result<Value, Value> {
        let request = InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: if cfg!(windows) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .unwrap(),
            body: args.into(),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.into(),
        };
        get_ipc_response(&self.webview, request).map(|body| body.deserialize().unwrap())
    }

    fn ok(&self, cmd: &str, args: Value) -> Value {
        self.invoke(cmd, args)
            .unwrap_or_else(|e| panic!("{cmd} failed: {e}"))
    }

    fn company(&self, name: &str) -> Value {
        self.ok(
            "create_client",
            json!({ "input": {
                "kind": "company", "lastName": null, "firstName": null, "middleName": null,
                "companyName": name, "legalName": null, "status": "active",
                "defaultCurrency": "RUB", "defaultRateMinor": 150000
            }}),
        )
    }
}

fn not_found(entity: &str, id: &Value) -> Value {
    json!({ "code": format!("{entity}.not_found"), "params": { "id": id } })
}

#[test]
fn clients_round_trip() {
    let app = TestApp::new();
    let client = app.company("Ромашка");
    assert_eq!(client["displayName"], "Ромашка");
    assert_eq!(client["defaultCurrency"], "RUB");
    assert_eq!(client["defaultRateMinor"], 150000);
    assert_eq!(client["createdAt"], NOW);
    let id = &client["id"];

    let list = app.ok(
        "list_clients",
        json!({ "filter": { "query": "ром", "status": null, "archived": false, "tagIds": [] } }),
    );
    assert_eq!(list, json!([client]));
    assert_eq!(app.ok("get_client", json!({ "id": id })), client);

    let archived = app.ok("archive_client", json!({ "id": id }));
    assert_eq!(archived["archivedAt"], NOW);
    assert!(app.ok("restore_client", json!({ "id": id }))["archivedAt"].is_null());
    assert!(app.ok("delete_client", json!({ "id": id })).is_null());
    assert_eq!(
        app.invoke("get_client", json!({ "id": id })),
        Err(not_found("client", id))
    );
}

#[test]
fn client_delete_impact() {
    let app = TestApp::new();
    let id = app.company("Ромашка")["id"].clone();
    app.ok(
        "create_note",
        json!({ "owner": { "type": "client", "id": id }, "input": { "bodyMd": "x", "pinned": false } }),
    );
    assert_eq!(
        app.ok("client_delete_impact", json!({ "id": id })),
        json!({ "contacts": 0, "projects": 0, "tasks": 0, "secrets": 0, "contactMethods": 0, "customFields": 0, "notes": 1, "events": 0, "timeEntries": 0, "interactions": 0, "invoices": 0 })
    );
}

#[test]
fn validation_errors_are_codes_with_params() {
    let app = TestApp::new();
    let err = app
        .invoke(
            "create_client",
            json!({ "input": {
                "kind": "company", "lastName": null, "firstName": null, "middleName": null,
                "companyName": "  ", "legalName": null, "status": "lead",
                "defaultCurrency": null, "defaultRateMinor": null
            }}),
        )
        .unwrap_err();
    assert_eq!(
        err,
        json!({ "code": "validation.required", "params": { "field": "companyName" } })
    );
}

#[test]
fn update_client() {
    let app = TestApp::new();
    let id = app.company("Ромашка")["id"].clone();
    let updated = app.ok(
        "update_client",
        json!({ "id": id, "input": {
            "kind": "person", "lastName": "Иванов", "firstName": "Иван", "middleName": null,
            "companyName": null, "legalName": null, "status": "paused",
            "defaultCurrency": null, "defaultRateMinor": null
        }}),
    );
    assert_eq!(updated["displayName"], "Иванов Иван");
}

#[test]
fn contacts_and_their_methods() {
    let app = TestApp::new();
    let client_id = app.company("Ромашка")["id"].clone();
    let contact = app.ok(
        "create_contact",
        json!({ "clientId": client_id, "input": {
            "lastName": "Сидоров", "firstName": "Пётр", "middleName": null,
            "position": "CTO", "note": null, "isPrimary": true
        }}),
    );
    assert_eq!(contact["displayName"], "Сидоров Пётр");
    let id = &contact["id"];
    let updated = app.ok(
        "update_contact",
        json!({ "id": id, "input": {
            "lastName": "Сидоров", "firstName": "Пётр", "middleName": null,
            "position": "CEO", "note": null, "isPrimary": false
        }}),
    );
    assert_eq!(updated["position"], "CEO");
    assert_eq!(
        app.ok("set_primary_contact", json!({ "id": id }))["isPrimary"],
        true
    );
    assert_eq!(
        app.ok("list_contacts", json!({ "clientId": client_id }))
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let owner = json!({ "type": "contact", "id": id });
    let methods = app.ok(
        "replace_contact_methods",
        json!({ "owner": owner, "methods": [
            { "kind": "email", "value": "petr@example.com", "label": "work" }
        ]}),
    );
    assert_eq!(methods[0]["value"], "petr@example.com");
    assert_eq!(
        app.ok("list_contact_methods", json!({ "owner": owner })),
        methods
    );
    let err = app
        .invoke(
            "replace_contact_methods",
            json!({ "owner": owner, "methods": [{ "kind": "email", "value": "x", "label": null }]}),
        )
        .unwrap_err();
    assert_eq!(
        err,
        json!({ "code": "validation.email", "params": { "field": "value", "index": 0 } })
    );

    assert!(app.ok("delete_contact", json!({ "id": id })).is_null());
    assert_eq!(
        app.invoke("delete_contact", json!({ "id": id })),
        Err(not_found("contact", id))
    );
}

#[test]
fn tags() {
    let app = TestApp::new();
    let client_id = app.company("Ромашка")["id"].clone();
    let tag = app.ok(
        "create_tag",
        json!({ "input": { "name": "VIP", "color": "#FF0000" } }),
    );
    assert_eq!(tag["color"], "#ff0000");
    let id = &tag["id"];
    let renamed = app.ok(
        "update_tag",
        json!({ "id": id, "input": { "name": "Важный", "color": "#00ff00" } }),
    );
    assert_eq!(app.ok("list_tags", json!({})), json!([renamed]));

    let target = json!({ "type": "client", "id": client_id });
    assert_eq!(
        app.ok("set_tags", json!({ "target": target, "tagIds": [id] })),
        json!([renamed])
    );
    assert_eq!(
        app.ok("tags_of", json!({ "target": target })),
        json!([renamed])
    );
    assert_eq!(
        app.invoke(
            "create_tag",
            json!({ "input": { "name": "важный", "color": "#000000" } })
        ),
        Err(json!({ "code": "tag.duplicate", "params": { "name": "важный" } }))
    );
    assert!(app.ok("delete_tag", json!({ "id": id })).is_null());
    assert_eq!(app.ok("tags_of", json!({ "target": target })), json!([]));
}

#[test]
fn custom_fields() {
    let app = TestApp::new();
    let owner = json!({ "type": "client", "id": app.company("Ромашка")["id"] });
    let field = |key: &str| {
        app.ok(
            "create_custom_field",
            json!({ "owner": owner, "input": { "key": key, "value": "", "fieldType": "text" } }),
        )
    };
    let (a, b) = (field("A"), field("B"));
    let updated = app.ok(
        "update_custom_field",
        json!({ "id": a["id"], "input": { "key": "A", "value": "3,5", "fieldType": "number" } }),
    );
    assert_eq!(updated["value"], "3.5");
    let reordered = app.ok(
        "reorder_custom_fields",
        json!({ "owner": owner, "ids": [b["id"], a["id"]] }),
    );
    assert_eq!(reordered[0]["key"], "B");
    assert_eq!(
        app.ok("list_custom_fields", json!({ "owner": owner })),
        reordered
    );
    assert!(
        app.ok("delete_custom_field", json!({ "id": b["id"] }))
            .is_null()
    );
    assert_eq!(
        app.invoke(
            "update_custom_field",
            json!({ "id": a["id"], "input": { "key": "A", "value": "x", "fieldType": "date" } })
        ),
        Err(json!({ "code": "validation.date", "params": { "field": "value" } }))
    );
}

#[test]
fn notes() {
    let app = TestApp::new();
    let owner = json!({ "type": "client", "id": app.company("Ромашка")["id"] });
    let note = app.ok(
        "create_note",
        json!({ "owner": owner, "input": { "bodyMd": "# Привет", "pinned": false } }),
    );
    let id = &note["id"];
    let updated = app.ok(
        "update_note",
        json!({ "id": id, "input": { "bodyMd": "# Пока", "pinned": false } }),
    );
    assert_eq!(updated["bodyMd"], "# Пока");
    let pinned = app.ok("set_note_pinned", json!({ "id": id, "pinned": true }));
    assert_eq!(pinned["pinned"], true);
    assert_eq!(
        app.ok("list_notes", json!({ "owner": owner })),
        json!([pinned])
    );
    assert!(app.ok("delete_note", json!({ "id": id })).is_null());
    assert_eq!(
        app.invoke("delete_note", json!({ "id": id })),
        Err(not_found("note", id))
    );
}

#[test]
fn malformed_arguments_are_rejected() {
    let app = TestApp::new();
    assert!(
        app.invoke("get_client", json!({ "id": "not-a-uuid" }))
            .is_err()
    );
}

#[test]
fn requisites() {
    let app = TestApp::new();
    let schemas = app.ok("list_country_schemas", json!({}));
    let codes: Vec<_> = schemas
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["code"].clone())
        .collect();
    assert_eq!(
        codes,
        [json!("RU"), json!("US"), json!("GB"), json!("INTL")]
    );
    assert_eq!(schemas[0]["legalForms"][2]["key"], "ooo");

    let owner = json!({ "type": "profile" });
    assert!(
        app.ok("get_requisites", json!({ "owner": owner }))
            .is_null()
    );
    let input = |inn: &str| {
        json!({ "owner": owner, "input": {
            "country": "RU", "legalForm": "self_employed", "name": "Иванов Иван",
            "values": { "inn": inn }, "confirmCountryChange": false
        }})
    };
    assert_eq!(
        app.ok("save_requisites", input("123")),
        json!({ "status": "invalid", "errors": [{ "field": "inn", "rule": "ru_inn_12" }] })
    );
    let saved = app.ok("save_requisites", input("5001 0073 2259"));
    assert_eq!(saved["status"], "saved");
    assert_eq!(
        saved["requisites"]["values"],
        json!({ "inn": "500100732259" })
    );
    assert_eq!(
        app.ok("get_requisites", json!({ "owner": owner })),
        saved["requisites"]
    );

    let client = app.company("Ромашка")["id"].clone();
    assert!(
        app.ok(
            "get_requisites",
            json!({ "owner": { "type": "client", "id": client } })
        )
        .is_null()
    );
}

#[test]
fn projects() {
    let app = TestApp::new();
    let client = app.company("Ромашка")["id"].clone();
    let input = |name: &str| {
        json!({
            "clientId": client, "name": name, "descriptionMd": "", "status": "active",
            "billingType": "hourly", "currency": null, "rateMinor": null,
            "budgetMinor": null, "startDate": "2024-03-01", "deadline": null
        })
    };
    // The client's RUB currency and 1500.00 rate are inherited.
    let p = app.ok("create_project", json!({ "input": input("Сайт") }));
    assert_eq!(p["currency"], "RUB");
    assert_eq!(p["rateMinor"], 150000);
    assert_eq!(p["clientName"], "Ромашка");
    let id = &p["id"];

    let filter = json!({ "query": null, "status": null, "clientId": client, "archived": false });
    assert_eq!(
        app.ok("list_projects", json!({ "filter": filter })),
        json!([p])
    );
    assert_eq!(app.ok("get_project", json!({ "id": id })), p);
    let updated = app.ok(
        "update_project",
        json!({ "id": id, "input": input("Сайт 2") }),
    );
    assert_eq!(updated["name"], "Сайт 2");
    assert_eq!(
        app.ok("archive_project", json!({ "id": id }))["archivedAt"],
        NOW
    );
    assert!(app.ok("restore_project", json!({ "id": id }))["archivedAt"].is_null());

    let mut bad = input("Сайт");
    bad["deadline"] = json!("2024-01-01");
    assert_eq!(
        app.invoke("create_project", json!({ "input": bad })),
        Err(json!({ "code": "validation.before_start", "params": { "field": "deadline" } }))
    );
    assert!(app.ok("delete_project", json!({ "id": id })).is_null());
    assert_eq!(
        app.invoke("get_project", json!({ "id": id })),
        Err(not_found("project", id))
    );
}

#[test]
fn board_columns() {
    let app = TestApp::new();
    let columns = app.ok("list_board_columns", json!({}));
    assert_eq!(columns.as_array().unwrap().len(), 4);
    assert_eq!(columns[0]["nameKey"], "todo");
    assert_eq!(columns[0]["category"], "todo");
    assert_eq!(columns[0]["taskCount"], 0);
    assert_eq!(columns[0]["overLimit"], false);

    let created = app.ok(
        "create_status",
        json!({ "input": { "name": "QA", "category": "in_progress", "color": "#123456", "wipLimit": 2 } }),
    );
    let id = &created["id"];
    let renamed = app.ok(
        "update_status",
        json!({ "id": id, "input": { "name": "Тесты", "category": "in_progress", "color": "#123456", "wipLimit": null } }),
    );
    assert_eq!(renamed["name"], "Тесты");
    let ids: Vec<Value> = app
        .ok("list_board_columns", json!({}))
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].clone())
        .rev()
        .collect();
    let reordered = app.ok("reorder_statuses", json!({ "ids": ids }));
    assert_eq!(reordered[0]["id"], *id);
    assert!(
        app.ok("delete_status", json!({ "id": id, "moveTo": null }))
            .is_null()
    );
    assert_eq!(
        app.invoke(
            "delete_status",
            json!({ "id": columns[0]["id"], "moveTo": null })
        ),
        Err(json!({ "code": "status.last_of_category", "params": { "category": "todo" } }))
    );
}

#[test]
fn tasks_and_checklists() {
    let app = TestApp::new();
    let columns = app.ok("list_board_columns", json!({}));
    let (todo, doing, done) = (&columns[0]["id"], &columns[1]["id"], &columns[3]["id"]);
    let input = |title: &str| {
        json!({
            "title": title, "descriptionMd": "", "statusId": null, "priority": "high",
            "projectId": null, "clientId": null, "dueDate": "2023-01-02", "dueTime": null,
            "estimateMinutes": 30
        })
    };
    let a = app.ok("create_task", json!({ "input": input("A") }));
    let b = app.ok("create_task", json!({ "input": input("B") }));
    assert_eq!(a["statusId"], *todo);
    assert_eq!(a["checklistTotal"], 0);

    let filter = json!({
        "query": null, "statusIds": [], "priority": "high", "scope": { "type": "inbox" },
        "tagIds": [], "due": "overdue", "hideDone": false, "sort": "due"
    });
    let listed = app.ok("list_tasks", json!({ "filter": filter }));
    assert_eq!(
        listed.as_array().unwrap().len(),
        2,
        "both are overdue: NOW is in November 2023"
    );

    let moved = app.ok(
        "move_task",
        json!({ "id": b["id"], "statusId": doing, "after": null, "before": null }),
    );
    assert_eq!(moved["statusId"], *doing);
    let finished = app.ok(
        "set_task_status",
        json!({ "id": a["id"], "statusId": done }),
    );
    assert_eq!(finished["completedAt"], NOW);
    let renamed = app.ok(
        "update_task",
        json!({ "id": a["id"], "input": input("A2") }),
    );
    assert_eq!(renamed["title"], "A2");
    assert_eq!(app.ok("get_task", json!({ "id": a["id"] }))["title"], "A2");

    let item = app.ok(
        "add_checklist_item",
        json!({ "taskId": a["id"], "text": "Макет" }),
    );
    let checked = app.ok(
        "update_checklist_item",
        json!({ "id": item["id"], "text": "Макет", "done": true }),
    );
    assert_eq!(checked["done"], true);
    let second = app.ok(
        "add_checklist_item",
        json!({ "taskId": a["id"], "text": "Код" }),
    );
    let order = app.ok(
        "reorder_checklist",
        json!({ "taskId": a["id"], "ids": [second["id"], item["id"]] }),
    );
    assert_eq!(order[0]["text"], "Код");
    assert_eq!(
        app.ok("list_checklist", json!({ "taskId": a["id"] })),
        order
    );
    assert!(
        app.ok("delete_checklist_item", json!({ "id": second["id"] }))
            .is_null()
    );
    assert_eq!(
        app.ok("get_task", json!({ "id": a["id"] }))["checklistDone"],
        1
    );

    assert!(app.ok("delete_task", json!({ "id": a["id"] })).is_null());
    assert_eq!(
        app.invoke(
            "move_task",
            json!({ "id": b["id"], "statusId": todo, "after": a["id"], "before": null })
        ),
        Err(json!({ "code": "task.stale_position", "params": {} }))
    );
}

#[test]
fn vault_touch_and_autolock_setting() {
    let app = TestApp::new();
    assert!(app.ok("vault_touch", json!({})).is_null());
    let mut settings = app.ok("get_settings", json!({}));
    assert_eq!(settings["vaultAutolockMinutes"], 15);
    settings["vaultAutolockMinutes"] = json!(0);
    let saved = app.ok("save_settings", json!({ "settings": settings }));
    assert_eq!(saved["vaultAutolockMinutes"], 0);
}

#[test]
fn secrets() {
    let app = TestApp::new();
    let templates = app.ok("list_secret_templates", json!({}));
    assert_eq!(templates[0]["kind"], "login");
    assert_eq!(
        templates[0]["fields"][2],
        json!({ "key": "password", "required": true, "secret": true, "multiline": false })
    );

    let input = json!({
        "title": "Хостинг", "kind": "login", "clientId": null, "projectId": null,
        "expiresOn": null,
        "payload": { "fields": { "username": "admin", "password": "hunter2" }, "custom": [] }
    });
    // Values need an unlocked vault.
    assert_eq!(
        app.invoke("create_secret", json!({ "input": input })),
        Err(json!({ "code": "vault.locked", "params": {} }))
    );
    app.unlock_vault();
    let secret = app.ok("create_secret", json!({ "input": input }));
    let id = &secret["id"];
    assert_eq!(secret["title"], "Хостинг");
    assert!(secret.get("payload").is_none());

    let filter = json!({ "query": null, "kind": null, "clientId": null, "projectId": null });
    let listed = app.ok("list_secrets", json!({ "filter": filter }));
    assert_eq!(listed, json!([secret]));
    assert!(!listed.to_string().contains("hunter2"));
    assert_eq!(app.ok("get_secret", json!({ "id": id })), secret);

    let password = json!({ "type": "field", "key": "password" });
    assert_eq!(
        app.ok(
            "reveal_secret_field",
            json!({ "id": id, "field": password })
        ),
        "hunter2"
    );
    assert_eq!(
        app.ok("secret_payload", json!({ "id": id }))["fields"]["username"],
        "admin"
    );
    let outline = app.ok("secret_outline", json!({ "id": id }));
    assert_eq!(
        outline,
        json!({ "fields": ["username", "password"], "custom": [] })
    );

    // Copying goes to the clipboard on the backend; nothing is returned.
    assert!(
        app.ok("copy_secret_field", json!({ "id": id, "field": password }))
            .is_null()
    );
    assert_eq!(app.clipboard.writes(), ["hunter2"]);

    let mut changed = input.clone();
    changed["title"] = json!("Хостинг 2");
    assert_eq!(
        app.ok("update_secret", json!({ "id": id, "input": changed }))["title"],
        "Хостинг 2"
    );

    app.lock_vault();
    assert_eq!(
        app.invoke(
            "reveal_secret_field",
            json!({ "id": id, "field": password })
        ),
        Err(json!({ "code": "vault.locked", "params": {} }))
    );
    assert_eq!(app.clipboard.writes().len(), 1);
    assert!(app.ok("delete_secret", json!({ "id": id })).is_null());
}

#[test]
fn generators() {
    let app = TestApp::new();
    let password = app.ok(
        "generate_password",
        json!({ "options": { "length": 24, "lowercase": true, "uppercase": false,
            "digits": true, "symbols": false, "excludeSimilar": true } }),
    );
    let value = password["value"].as_str().unwrap();
    assert_eq!(value.len(), 24);
    assert!(
        value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    );
    assert!(password["entropyBits"].as_f64().unwrap() > 100.0);

    let phrase = app.ok(
        "generate_passphrase",
        json!({ "options": { "words": 4, "separator": ".", "capitalize": false } }),
    );
    assert_eq!(phrase["value"].as_str().unwrap().split('.').count(), 4);
    assert_eq!(
        app.invoke(
            "generate_password",
            json!({ "options": { "length": 1, "lowercase": true,
            "uppercase": true, "digits": true, "symbols": true, "excludeSimilar": false } })
        ),
        Err(json!({ "code": "validation.range", "params": { "field": "length" } }))
    );
}

#[test]
fn ssh_keys() {
    let app = TestApp::new();
    let pair = app.ok(
        "generate_ssh_key",
        json!({ "keyType": "ed25519", "comment": "deploy", "passphrase": null }),
    );
    assert_eq!(pair["algorithm"], "ssh-ed25519");
    let private_key = pair["privateKey"].as_str().unwrap().to_owned();
    let imported = app.ok(
        "import_ssh_key",
        json!({ "privateKey": private_key, "passphrase": null }),
    );
    assert_eq!(imported["publicKey"], pair["publicKey"]);
    assert_eq!(
        app.invoke(
            "import_ssh_key",
            json!({ "privateKey": "junk", "passphrase": null })
        ),
        Err(json!({ "code": "ssh.invalid_key", "params": {} }))
    );

    app.unlock_vault();
    let secret = app.ok(
        "create_secret",
        json!({ "input": {
            "title": "Deploy key", "kind": "ssh_key", "clientId": null, "projectId": null,
            "expiresOn": null,
            "payload": { "fields": { "private_key": private_key, "public_key": pair["publicKey"] }, "custom": [] }
        }}),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("id_deploy");
    assert!(
        app.ok(
            "export_ssh_key",
            json!({ "id": secret["id"], "path": path })
        )
        .is_null()
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), private_key);
}

#[test]
fn vault_lifecycle() {
    let app = TestApp::new();
    assert_eq!(app.ok("vault_status", json!({})), "uninitialized");
    assert_eq!(
        app.invoke("vault_init", json!({ "password": "short" })),
        Err(
            json!({ "code": "validation.min_length", "params": { "field": "password", "min": 8 } })
        )
    );
    let recovery_key = app.ok("vault_init", json!({ "password": "master password" }));
    assert_eq!(recovery_key.as_str().unwrap().split('-').count(), 14);
    assert_eq!(app.ok("vault_status", json!({})), "unlocked");
    assert!(app.ok("vault_lock", json!({})).is_null());
    assert_eq!(app.ok("vault_status", json!({})), "locked");
    assert_eq!(
        app.invoke("vault_unlock", json!({ "password": "wrong password" })),
        Err(json!({ "code": "vault.wrong_password", "params": {} }))
    );
    assert!(
        app.ok("vault_unlock", json!({ "password": "master password" }))
            .is_null()
    );
    assert!(
        app.ok(
            "vault_change_password",
            json!({ "oldPassword": "master password", "newPassword": "another password" })
        )
        .is_null()
    );
    app.ok("vault_lock", json!({}));
    assert!(
        app.ok("vault_unlock", json!({ "password": "another password" }))
            .is_null()
    );
}

#[test]
fn recovery_key() {
    let app = TestApp::new();
    let first = app.ok("vault_init", json!({ "password": "master password" }));
    let second = app.ok("vault_rotate_recovery_key", json!({}));
    assert_ne!(first, second);
    app.ok("vault_lock", json!({}));
    assert_eq!(
        app.invoke(
            "vault_recover",
            json!({ "recoveryKey": first, "newPassword": "new password 1" })
        ),
        Err(json!({ "code": "recovery.invalid", "params": {} }))
    );
    assert!(
        app.ok(
            "vault_recover",
            json!({ "recoveryKey": second, "newPassword": "new password 1" })
        )
        .is_null()
    );
    assert_eq!(app.ok("vault_status", json!({})), "unlocked");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recovery.txt");
    assert!(
        app.ok(
            "save_recovery_key_file",
            json!({ "path": path, "recoveryKey": second })
        )
        .is_null()
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap().trim(),
        second.as_str().unwrap()
    );
    // Anything that is not a recovery key is refused.
    assert_eq!(
        app.invoke(
            "save_recovery_key_file",
            json!({ "path": dir.path().join("x"), "recoveryKey": "#!/bin/sh" })
        ),
        Err(json!({ "code": "recovery.invalid_format", "params": {} }))
    );
}

#[test]
fn attachments_open_reveal_save_and_delete() {
    let app = TestApp::new();
    let client = app.company("Ромашка");
    let owner = json!({ "type": "client", "id": client["id"] });
    let source = app.dir.path().join("Договор.pdf");
    std::fs::write(&source, b"%PDF").unwrap();

    let attached = app.ok(
        "attach_file",
        json!({ "owner": owner, "path": source, "encrypt": false }),
    );
    assert_eq!(attached["fileName"], "Договор.pdf");
    assert_eq!(attached["mime"], "application/pdf");
    assert_eq!(attached["size"], 4);
    assert_eq!(attached["hasThumbnail"], false);
    let id = &attached["id"];
    assert_eq!(
        app.ok("list_attachments", json!({ "owner": owner })),
        json!([attached])
    );

    app.ok("open_attachment", json!({ "id": id }));
    app.ok("reveal_attachment", json!({ "id": id }));
    let calls = app.desktop.0.lock().unwrap().clone();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "open");
    assert_eq!(calls[0].1.file_name().unwrap(), "Договор.pdf");
    assert_eq!(std::fs::read(&calls[0].1).unwrap(), b"%PDF");
    assert_eq!(calls[1].0, "reveal");

    let dest = app.dir.path().join("copy.pdf");
    app.ok("save_attachment_as", json!({ "id": id, "path": dest }));
    assert_eq!(std::fs::read(&dest).unwrap(), b"%PDF");

    app.ok("delete_attachment", json!({ "id": id }));
    assert_eq!(
        app.ok("list_attachments", json!({ "owner": owner })),
        json!([])
    );
    assert_eq!(
        app.invoke("open_attachment", json!({ "id": id })),
        Err(not_found("attachment", id))
    );
}

#[test]
fn encrypted_attachments_are_wiped_on_lock_and_not_revealed() {
    let app = TestApp::new();
    app.unlock_vault();
    let client = app.company("Ромашка");
    let owner = json!({ "type": "client", "id": client["id"] });
    let source = app.dir.path().join("passport.jpg");
    std::fs::write(&source, b"JPEG").unwrap();
    let attached = app.ok(
        "attach_file",
        json!({ "owner": owner, "path": source, "encrypt": true }),
    );
    assert_eq!(attached["encrypted"], true);
    let id = &attached["id"];

    app.ok("open_attachment", json!({ "id": id }));
    let opened = app.desktop.0.lock().unwrap()[0].1.clone();
    assert!(opened.exists());
    // A decrypted copy must not be left in a folder the user browses to.
    assert_eq!(
        app.invoke("reveal_attachment", json!({ "id": id })),
        Err(json!({ "code": "attachment.encrypted", "params": {} }))
    );
    app.ok("vault_lock", json!({}));
    assert!(!opened.exists());
    assert_eq!(
        app.invoke("open_attachment", json!({ "id": id })),
        Err(json!({ "code": "vault.locked", "params": {} }))
    );
}

#[test]
fn start_up_removes_leftovers_and_unreferenced_files() {
    let app = TestApp::new();
    let leftover = app
        .dir
        .path()
        .join("open")
        .join("secure")
        .join("x")
        .join("a.txt");
    std::fs::create_dir_all(leftover.parent().unwrap()).unwrap();
    std::fs::write(&leftover, b"decrypted").unwrap();
    let client = app.company("Ромашка");
    let owner = json!({ "type": "client", "id": client["id"] });
    let source = app.dir.path().join("a.txt");
    std::fs::write(&source, b"data").unwrap();
    app.ok(
        "attach_file",
        json!({ "owner": owner, "path": source, "encrypt": false }),
    );
    app.ok("delete_client", json!({ "id": client["id"] }));

    app.app.state::<AppState>().start_up().unwrap();
    assert!(!app.dir.path().join("open").exists());
    // SHA-256 of "data".
    let hash = "3a6eb0790f39ac87c94f3856b2dd2c5d110e6811602261a9a923d3bb23adc8b7";
    let blob = app.dir.path().join("blobs/3a/6e").join(hash);
    assert!(!blob.exists(), "the deleted client's file is gone");
}

#[test]
fn pasting_an_image_from_the_clipboard() {
    let app = TestApp::new();
    let client = app.company("Ромашка");
    let owner = json!({ "type": "client", "id": client["id"] });
    let args = json!({ "owner": owner, "fileName": "Screenshot.png", "encrypt": false });
    assert_eq!(
        app.invoke("attach_clipboard_image", args.clone()),
        Err(json!({ "code": "clipboard.no_image", "params": {} }))
    );
    app.clipboard.0.lock().unwrap().push("image".into());
    let attached = app.ok("attach_clipboard_image", args);
    assert_eq!(attached["fileName"], "Screenshot.png");
    assert_eq!(attached["mime"], "image/png");
    assert_eq!(attached["hasThumbnail"], true);
}

#[test]
fn image_protocol_serves_thumbnails_and_images() {
    let app = TestApp::new();
    app.unlock_vault();
    let client = app.company("Ромашка");
    let owner = json!({ "type": "client", "id": client["id"] });
    app.clipboard.0.lock().unwrap().push("image".into());
    let plain = app.ok(
        "attach_clipboard_image",
        json!({ "owner": owner, "fileName": "a.png", "encrypt": false }),
    );
    let secret = app.ok(
        "attach_clipboard_image",
        json!({ "owner": owner, "fileName": "b.png", "encrypt": true }),
    );
    let state = app.app.state::<AppState>();
    let get = |path: String| crate::protocol::respond(&state, &path);

    let thumb = get(format!("/thumb/{}", plain["id"].as_str().unwrap()));
    assert_eq!(thumb.status(), 200);
    assert_eq!(thumb.headers()["content-type"], "image/png");
    assert!(thumb.body().starts_with(b"\x89PNG"));
    // convertFileSrc() encodes the slash.
    let full = get(format!("/full%2F{}", secret["id"].as_str().unwrap()));
    assert_eq!(full.status(), 200);
    assert_eq!(full.headers()["cache-control"], "no-store");

    app.lock_vault();
    assert_eq!(
        get(format!("/full/{}", secret["id"].as_str().unwrap())).status(),
        403
    );
    assert_eq!(get(format!("/thumb/{}", uuid_like())).status(), 404);
    assert_eq!(get("/nonsense".into()).status(), 404);
}

fn uuid_like() -> String {
    crm_core::domain::Id::new().to_string()
}

#[test]
fn events_round_trip() {
    let app = TestApp::new();
    let input = json!({
        "title": "Созвон", "descriptionMd": "", "location": null, "allDay": false,
        "start": "2026-03-10T10:00", "end": "2026-03-10T11:00", "tzid": "Europe/Moscow",
        "clientId": null, "projectId": null, "taskId": null, "contactId": null
    });
    let event = app.ok("create_event", json!({ "input": input }));
    assert_eq!(event["startAt"], 1_773_126_000_000_i64);
    assert_eq!(event["start"], "2026-03-10T10:00");
    let list = app.ok(
        "list_events",
        json!({ "from": 1_773_100_000_000_i64, "to": 1_773_200_000_000_i64 }),
    );
    assert_eq!(list, json!([event]));
    let mut bad = input.clone();
    bad["tzid"] = json!("Nowhere");
    assert_eq!(
        app.invoke("update_event", json!({ "id": event["id"], "input": bad })),
        Err(json!({ "code": "validation.timezone", "params": { "field": "tzid" } }))
    );
    let mut weekly = input.clone();
    weekly["rrule"] = json!("FREQ=WEEKLY;COUNT=2");
    let series = app.ok("create_event", json!({ "input": weekly }));
    let week = 7 * 24 * 3_600_000_i64;
    let occurrences = app.ok(
        "list_occurrences",
        json!({ "from": 1_773_100_000_000_i64, "to": 1_773_200_000_000_i64 + 2 * week }),
    );
    let mine: Vec<_> = occurrences
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| o["event"]["id"] == series["id"])
        .map(|o| o["recurrenceId"].clone())
        .collect();
    assert_eq!(mine, [json!("2026-03-10T10:00"), json!("2026-03-17T10:00")]);
    app.ok(
        "delete_occurrence",
        json!({ "id": series["id"], "recurrenceId": "2026-03-17T10:00", "scope": "this" }),
    );
    assert_eq!(
        app.ok("get_event", json!({ "id": series["id"] }))["exdates"],
        json!(["2026-03-17T10:00"])
    );
    let mut moved = input.clone();
    moved["start"] = json!("2026-03-10T12:00");
    moved["end"] = json!("2026-03-10T13:00");
    let over = app.ok(
        "update_occurrence",
        json!({ "id": series["id"], "recurrenceId": "2026-03-10T10:00", "scope": "this", "input": moved }),
    );
    assert_eq!(over["seriesId"], series["id"]);
    let items = app.ok(
        "calendar_items",
        json!({
            "from": 1_773_100_000_000_i64, "to": 1_773_200_000_000_i64,
            "layers": { "events": true, "tasks": true, "projects": true, "invoices": true,
                        "showCompleted": false }
        }),
    );
    let moved = items
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["occurrence"]["event"]["seriesId"] == series["id"])
        .unwrap();
    assert_eq!(moved["kind"], "event");
    assert_eq!(moved["start"], "2026-03-10T12:00");
    assert_eq!(moved["occurrence"]["recurrenceId"], "2026-03-10T10:00");
    let owner = json!({ "type": "event", "id": event["id"] });
    let set = app.ok(
        "set_reminders",
        json!({ "owner": owner, "minutesBefore": [60, 10] }),
    );
    assert_eq!(set[0]["minutesBefore"], 10);
    assert_eq!(app.ok("list_reminders", json!({ "owner": owner })), set);
    app.ok("delete_event", json!({ "id": event["id"] }));
    assert_eq!(
        app.invoke("get_event", json!({ "id": event["id"] })),
        Err(not_found("event", &event["id"]))
    );
}

#[test]
fn ics_preview_and_import() {
    let app = TestApp::new();
    let path = app.dir.path().join("calendar.ics");
    std::fs::write(
        &path,
        "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a@x\r\nDTSTART:20260310T100000Z\r\n\
         SUMMARY:Созвон\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:b@x\r\nSUMMARY:Без начала\r\n\
         END:VEVENT\r\nEND:VCALENDAR\r\n",
    )
    .unwrap();
    let preview = app.ok("preview_ics", json!({ "path": path }));
    assert_eq!(preview["events"][0]["title"], "Созвон");
    assert_eq!(preview["issues"][0]["error"]["code"], "ics.no_start");
    let link = json!({ "clientId": null, "projectId": null });
    let report = app.ok(
        "import_ics",
        json!({ "path": path, "uids": null, "link": link }),
    );
    assert_eq!(
        (report["created"].clone(), report["updated"].clone()),
        (json!(1), json!(0))
    );
    let again = app.ok(
        "import_ics",
        json!({ "path": path, "uids": ["a@x"], "link": link }),
    );
    assert_eq!(again["updated"], 1);
    let out = app.dir.path().join("out.ics");
    let filter = json!({ "clientId": null, "projectId": null, "from": null, "to": null });
    app.ok("export_ics", json!({ "path": out, "filter": filter }));
    let written = std::fs::read_to_string(&out).unwrap();
    assert!(written.contains("UID:a@x\r\n"));
    assert!(written.contains("SUMMARY:Созвон\r\n"));
    assert_eq!(
        app.invoke(
            "preview_ics",
            json!({ "path": app.dir.path().join("missing.ics") })
        ),
        Err(json!({ "code": "file.read_failed", "params": {} }))
    );
}

#[test]
fn missed_reminders_are_listed_until_seen() {
    let app = TestApp::new();
    assert_eq!(app.ok("missed_reminders", json!({})), json!([]));
    {
        let state = app.app.state::<AppState>();
        let db = state.db.lock().unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO events (id, uid, title, all_day, start_local, end_local, tzid,
                     start_at, end_at, created_at, updated_at)
                 VALUES ('01900000-0000-7000-8000-000000000001', 'e1@x', 'Созвон', 0, '2023-11-14T22:13', '2023-11-14T22:13', 'UTC',
                     1700000000000, 1700000000000, 1, 1);
                 INSERT INTO reminders (id, owner_type, owner_id, minutes_before, created_at)
                 VALUES ('01900000-0000-7000-8000-000000000002', 'event', '01900000-0000-7000-8000-000000000001', 0, 1);
                 INSERT INTO reminder_log (reminder_id, occurrence, fire_at, owner_type, owner_id,
                     title, start_at, all_day, fired_at, missed)
                 VALUES ('01900000-0000-7000-8000-000000000002', '', 1699990000000, 'event', '01900000-0000-7000-8000-000000000001', 'Созвон', 1700000000000, 0,
                     1700000000000, 1);",
            )
            .unwrap();
    }
    let missed = app.ok("missed_reminders", json!({}));
    assert_eq!(missed[0]["title"], "Созвон");
    assert_eq!(
        missed[0]["owner"],
        json!({ "type": "event", "id": "01900000-0000-7000-8000-000000000001" })
    );
    app.ok("mark_missed_reminders_seen", json!({}));
    assert_eq!(app.ok("missed_reminders", json!({})), json!([]));
}

#[test]
fn timer_and_time_entries() {
    let app = TestApp::new();
    assert_eq!(app.ok("timer_running", json!({})), json!(null));
    let target = json!({ "taskId": null, "projectId": null, "clientId": null });
    let running = app.ok(
        "timer_start",
        json!({ "target": target, "description": "Почта", "billable": false }),
    );
    assert_eq!(running["endedAt"], json!(null));
    assert_eq!(app.ok("timer_running", json!({}))["id"], running["id"]);
    let stopped = app.ok("timer_stop", json!({}));
    assert_eq!(stopped["endedAt"], NOW);
    assert_eq!(
        app.invoke("timer_stop", json!({})),
        Err(json!({ "code": "timer.not_running", "params": {} }))
    );
    let saved = app.ok(
        "create_time_entry",
        json!({ "input": {
            "target": target, "description": "", "billable": true,
            "startedAt": NOW - 3_600_000, "endedAt": NOW - 60_000
        }}),
    );
    assert_eq!(saved["overlaps"], json!([]));
    app.ok("delete_time_entry", json!({ "id": saved["entry"]["id"] }));
    // The default setting stops a running timer when another starts.
    app.ok(
        "timer_start",
        json!({ "target": target, "description": "a", "billable": true }),
    );
    app.ok(
        "timer_start",
        json!({ "target": target, "description": "b", "billable": true }),
    );
    assert_eq!(app.ok("timer_running", json!({}))["description"], "b");
    app.ok("timer_stop", json!({}));
    let filter =
        json!({ "from": null, "to": null, "clientId": null, "projectId": null, "billable": null });
    assert_eq!(
        app.ok("list_time_entries", json!({ "filter": filter }))
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let report = app.ok(
        "time_report",
        json!({ "filter": filter, "group": "project" }),
    );
    assert_eq!(report["rows"][0]["key"], "");
    let path = app.dir.path().join("time.csv");
    let headers: Vec<String> = (1..=11).map(|i| format!("h{i}")).collect();
    app.ok(
        "export_time_csv",
        json!({ "path": path, "filter": filter, "headers": headers }),
    );
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .starts_with("h1,h2,")
    );
}

#[test]
fn interaction_feed_pages() {
    let app = TestApp::new();
    let client = app.company("Ромашка")["id"].clone();
    for (i, kind) in ["call", "email", "call"].iter().enumerate() {
        app.ok(
            "create_interaction",
            json!({ "input": {
                "clientId": client, "contactId": null, "projectId": null, "kind": kind,
                "occurredAt": NOW - i as i64 * 60_000, "summaryMd": format!("#{i}")
            }}),
        );
    }
    let first = app.ok(
        "interaction_feed",
        json!({ "clientId": client, "kind": null, "after": null, "limit": 2 }),
    );
    assert_eq!(first["items"].as_array().unwrap().len(), 2);
    let rest = app.ok(
        "interaction_feed",
        json!({ "clientId": client, "kind": null, "after": first["next"], "limit": 2 }),
    );
    assert_eq!(rest["items"][0]["summaryMd"], "#2");
    assert_eq!(rest["next"], json!(null));
    let source = &first["items"][0]["id"];
    let task = app.ok(
        "follow_up_task",
        json!({ "id": source, "title": "Перезвонить", "dueDate": null }),
    );
    assert_eq!(task["kind"], "task");
    let page = app.ok(
        "interaction_feed",
        json!({ "clientId": client, "kind": null, "after": null, "limit": 1 }),
    );
    assert_eq!(page["items"][0]["followUps"][0]["title"], "Перезвонить");
}

#[test]
fn invoices_round_trip() {
    let app = TestApp::new();
    let client = app.company("Ромашка")["id"].clone();
    let input = json!({
        "number": "INV-1", "clientId": client, "projectId": null, "currency": "RUB",
        "issueDate": "2026-03-01", "dueDate": "2026-03-15", "status": "sent",
        "items": [{ "description": "Вёрстка", "quantityMilli": 1500, "unit": "ч",
                    "price": { "amountMinor": 300000, "currency": "RUB" } }],
        "discount": { "type": "percent", "value": 1000 }, "taxBp": 0, "notesMd": "",
        "language": "ru"
    });
    let invoice = app.ok("create_invoice", json!({ "input": input }));
    assert_eq!(
        invoice["totals"]["total"],
        json!({ "amountMinor": 405000, "currency": "RUB" })
    );
    assert_eq!(invoice["items"][0]["amount"]["amountMinor"], 450000);
    let listed = app.ok(
        "list_invoices",
        json!({ "clientId": client, "projectId": null }),
    );
    assert_eq!(listed[0]["number"], "INV-1");
    assert_eq!(
        app.invoke("delete_client", json!({ "id": client })),
        Err(json!({ "code": "client.has_invoices", "params": {} }))
    );
    app.ok("delete_invoice", json!({ "id": invoice["id"] }));
    assert_eq!(
        app.ok(
            "preview_invoice_number",
            json!({ "issueDate": "2026-03-01" })
        ),
        "INV-2026-001"
    );
    let filter = json!({ "from": null, "to": null, "clientId": client, "projectId": null, "billable": null });
    assert_eq!(
        app.ok("unbilled_time", json!({ "filter": filter })),
        json!([])
    );
    assert_eq!(
        app.invoke(
            "invoice_from_time",
            json!({ "input": {
                "clientId": client, "projectId": null, "from": 0, "to": NOW, "grouping": "by_task",
                "description": "", "number": "", "issueDate": "2026-03-01", "dueDate": null,
                "taxBp": 0, "language": "en"
            }})
        ),
        Err(json!({ "code": "invoice.no_time", "params": {} }))
    );
}

#[test]
fn payments_follow_the_invoice() {
    let app = TestApp::new();
    let client = app.company("Ромашка")["id"].clone();
    let invoice = app.ok(
        "create_invoice",
        json!({ "input": {
            "number": "", "clientId": client, "projectId": null, "currency": "RUB",
            "issueDate": "2026-03-01", "dueDate": null, "status": "sent",
            "items": [{ "description": "Работа", "quantityMilli": 1000, "unit": "",
                        "price": { "amountMinor": 100000, "currency": "RUB" } }],
            "discount": { "type": "none" }, "taxBp": 0, "notesMd": "", "language": "ru"
        }}),
    );
    let payment = |amount: i64| {
        json!({ "input": {
            "invoiceId": invoice["id"], "projectId": null,
            "amount": { "amountMinor": amount, "currency": "RUB" },
            "paidOn": "2026-03-05", "method": "", "note": ""
        }})
    };
    let saved = app.ok("create_payment", payment(40_000));
    assert_eq!(saved["overpaid"], json!(null));
    assert_eq!(
        app.ok("get_invoice", json!({ "id": invoice["id"] }))["status"],
        "partially_paid"
    );
    let over = app.ok("create_payment", payment(70_000));
    assert_eq!(over["overpaid"]["amountMinor"], 10_000);
    assert_eq!(
        app.ok("invoice_payments", json!({ "invoiceId": invoice["id"] }))
            .as_array()
            .unwrap()
            .len(),
        2
    );
    app.ok("delete_payment", json!({ "id": over["payment"]["id"] }));
    assert_eq!(
        app.ok("get_invoice", json!({ "id": invoice["id"] }))["status"],
        "partially_paid"
    );
}

#[test]
fn expenses_round_trip() {
    let app = TestApp::new();
    let input = json!({
        "projectId": null, "category": "Хостинг",
        "amount": { "amountMinor": 50000, "currency": "RUB" },
        "spentOn": "2026-03-02", "description": ""
    });
    let expense = app.ok("create_expense", json!({ "input": input }));
    assert_eq!(app.ok("expense_categories", json!({})), json!(["Хостинг"]));
    let filter = json!({ "projectId": null, "from": null, "to": null, "category": null });
    assert_eq!(
        app.ok("list_expenses", json!({ "filter": filter }))[0]["id"],
        expense["id"]
    );
    app.ok("delete_expense", json!({ "id": expense["id"] }));
}

#[test]
fn invoice_pdf_and_profile_images() {
    let app = TestApp::new();
    let client = app.company("Ромашка")["id"].clone();
    let invoice = app.ok(
        "create_invoice",
        json!({ "input": {
            "number": "", "clientId": client, "projectId": null, "currency": "RUB",
            "issueDate": "2026-03-01", "dueDate": null, "status": "sent",
            "items": [{ "description": "Разработка", "quantityMilli": 1000, "unit": "ч",
                        "price": { "amountMinor": 150000, "currency": "RUB" } }],
            "discount": { "type": "none" }, "taxBp": 0, "notesMd": "", "language": "ru"
        }}),
    );
    let path = app.dir.path().join("invoice.pdf");
    app.ok(
        "save_invoice_pdf",
        json!({ "id": invoice["id"], "path": path }),
    );
    let pdf = std::fs::read(&path).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    let text = pdf_extract::extract_text_from_mem(&pdf).unwrap();
    assert!(text.contains("INV-2026-001"), "{text}");
    assert!(text.contains("Ромашка"), "{text}");

    app.ok("open_invoice_pdf", json!({ "id": invoice["id"] }));
    let opened = app.desktop.0.lock().unwrap().last().unwrap().1.clone();
    assert_eq!(opened.file_name().unwrap(), "INV-2026-001.pdf");

    assert_eq!(
        app.ok("profile_image", json!({ "kind": "logo" })),
        json!(null)
    );
    let logo = app.dir.path().join("logo.png");
    // A 1×1 PNG.
    std::fs::write(
        &logo,
        [
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99,
            0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ],
    )
    .unwrap();
    app.ok("set_profile_image", json!({ "kind": "logo", "path": logo }));
    assert!(
        app.ok("profile_image", json!({ "kind": "logo" }))
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,")
    );
    let pdf_file = app.dir.path().join("x.pdf");
    std::fs::write(&pdf_file, b"%PDF").unwrap();
    assert_eq!(
        app.invoke(
            "set_profile_image",
            json!({ "kind": "stamp", "path": pdf_file })
        ),
        Err(json!({ "code": "image.unsupported", "params": {} }))
    );
    app.ok("delete_profile_image", json!({ "kind": "logo" }));
}

#[test]
fn taxes_round_trip() {
    let app = TestApp::new();
    // No profile country yet: only "custom".
    assert_eq!(app.ok("tax_presets", json!({})), json!([]));
    app.ok(
        "save_requisites",
        json!({ "owner": { "type": "profile" }, "input": {
            "country": "RU", "legalForm": "self_employed", "name": "Иванов",
            "values": { "inn": "500100732259" }, "confirmCountryChange": false
        }}),
    );
    let presets = app.ok("tax_presets", json!({}));
    assert_eq!(
        presets
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["key"].clone())
            .collect::<Vec<_>>(),
        [json!("npd"), json!("usn_income")]
    );
    let npd = app.ok(
        "tax_preset",
        json!({ "key": "npd", "validFrom": "2026-01-01" }),
    );
    assert_eq!(
        npd["rules"][0],
        json!({ "rateBp": 400, "clientKind": "person", "currency": null })
    );
    let saved = app.ok("save_tax_regime", json!({ "id": null, "input": npd }));
    assert_eq!(app.ok("list_tax_regimes", json!({}))[0]["id"], saved["id"]);
    // Off by default: no figures.
    assert_eq!(
        app.ok(
            "tax_estimate",
            json!({ "from": "2026-01-01", "to": "2026-12-31" })
        ),
        json!(null)
    );
    app.ok("delete_tax_regime", json!({ "id": saved["id"] }));
}

#[test]
fn finance_dashboard_for_a_period() {
    let app = TestApp::new();
    let d = app.ok(
        "finance_dashboard",
        json!({ "period": { "from": "2026-01-01", "to": "2026-02-28" } }),
    );
    assert_eq!(d["taxesEnabled"], false);
    assert_eq!(d["currencies"][0]["incomeMinor"], 0);
    assert_eq!(d["months"].as_array().unwrap().len(), 2);
    assert_eq!(
        app.invoke(
            "finance_dashboard",
            json!({ "period": { "from": "2026-02-01", "to": "2026-01-01" } })
        )
        .unwrap_err(),
        json!({ "code": "validation.before_start", "params": { "field": "to" } })
    );
}
