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
        json!({ "contacts": 0, "projects": 0, "tasks": 0, "secrets": 0, "contactMethods": 0, "customFields": 0, "notes": 1, "events": 0 })
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
    app.ok("delete_event", json!({ "id": event["id"] }));
    assert_eq!(
        app.invoke("get_event", json!({ "id": event["id"] })),
        Err(not_found("event", &event["id"]))
    );
}
