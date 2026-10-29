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
use tauri::{App, WebviewWindow, WebviewWindowBuilder};

use crate::AppState;

const NOW: i64 = 1_700_000_000_000;

struct TestApp {
    _app: App<MockRuntime>,
    webview: WebviewWindow<MockRuntime>,
}

impl TestApp {
    fn new() -> Self {
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
            ))
            .build(mock_context(noop_assets()))
            .unwrap();
        let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        Self { _app: app, webview }
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
        json!({ "contacts": 0, "projects": 0, "tasks": 0, "contactMethods": 0, "customFields": 0, "notes": 1 })
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
