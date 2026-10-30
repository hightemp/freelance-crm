//! Security checks of the app's configuration (NFR-3): a strict content
//! security policy, no file system or shell access for the webview, and a
//! file protocol that serves attachments by id only.

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crm_core::domain::{FakeClock, Id, Timestamp};
    use serde_json::Value;

    use crate::reminders::tests::{Recording, app};

    fn config() -> Value {
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap()
    }

    /// The directives of a policy, by name.
    fn directives(csp: &str) -> std::collections::HashMap<&str, Vec<&str>> {
        csp.split(';')
            .filter_map(|d| {
                let mut parts = d.split_whitespace();
                Some((parts.next()?, parts.collect()))
            })
            .collect()
    }

    #[test]
    fn the_content_security_policy_is_strict() {
        let security = &config()["app"]["security"];
        for key in ["csp", "devCsp"] {
            let csp = security[key]
                .as_str()
                .unwrap_or_else(|| panic!("{key} is set"));
            assert!(!csp.contains("unsafe-eval"), "{key}: {csp}");
            let d = directives(csp);
            assert_eq!(d["default-src"], ["'self'"], "{key}");
            assert_eq!(d["object-src"], ["'none'"], "{key}");
            assert_eq!(d["frame-ancestors"], ["'none'"], "{key}");
            assert!(d["connect-src"].contains(&"ipc:"), "{key}");
        }
        // Scripts only from the app itself in builds (Tauri adds the hashes
        // of its inline scripts); no remote hosts anywhere.
        let csp = security["csp"].as_str().unwrap();
        let d = directives(csp);
        assert_eq!(d["script-src"], ["'self'"]);
        assert!(
            !csp.contains("https:") && !csp.contains("http://localhost:"),
            "{csp}"
        );
    }

    #[test]
    fn the_webview_has_no_file_system_or_shell_access() {
        let capability: Value =
            serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();
        for permission in capability["permissions"].as_array().unwrap() {
            let p = permission.as_str().unwrap_or_default();
            assert!(
                !["fs:", "shell:", "http:", "process:"]
                    .iter()
                    .any(|prefix| p.starts_with(prefix)),
                "{p}"
            );
        }
        let manifest = include_str!("../Cargo.toml");
        for plugin in ["tauri-plugin-fs", "tauri-plugin-shell", "tauri-plugin-http"] {
            assert!(!manifest.contains(plugin), "{plugin}");
        }
    }

    #[test]
    fn the_file_protocol_serves_attachments_by_id_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("outside.txt"), "private").unwrap();
        let state = app(
            dir.path(),
            Arc::new(FakeClock::at(Timestamp::from_millis(0))),
            Recording::default(),
        );
        for path in [
            "/../outside.txt",
            "/full/../../outside.txt",
            "/full/..%2F..%2Foutside.txt",
            "/full/%2E%2E%2Foutside.txt",
            &format!("/full/{}", dir.path().join("outside.txt").display()),
            &format!("/full/{}", Id::new()),
            "/crm.db",
        ] {
            let response = crate::protocol::respond(&state, path);
            assert_eq!(response.status(), 404, "{path}");
            assert!(response.body().is_empty(), "{path}");
        }
    }
}
