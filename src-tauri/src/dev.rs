//! Development-only environment tweaks (compiled in debug builds only).

const LOCAL_HOSTS: [&str; 2] = ["127.0.0.1", "localhost"];

/// Adds the dev server hosts to a `no_proxy` list. With `http_proxy` set and
/// no `no_proxy`, WebKitGTK sends even `127.0.0.1:1420` through the proxy and
/// the window shows "Message was restarted too many times".
pub fn no_proxy_with_localhost(existing: Option<&str>) -> String {
    let existing = existing.unwrap_or("").trim();
    let present: Vec<&str> = existing.split(',').map(str::trim).collect();
    let missing: Vec<&str> = LOCAL_HOSTS
        .into_iter()
        .filter(|host| !present.contains(host))
        .collect();
    match (existing.is_empty(), missing.is_empty()) {
        (true, _) => LOCAL_HOSTS.join(","),
        (false, true) => existing.to_owned(),
        (false, false) => format!("{existing},{}", missing.join(",")),
    }
}

/// Must run before any thread is spawned (env mutation is not thread-safe).
pub fn bypass_proxy_for_dev_server() {
    for key in ["no_proxy", "NO_PROXY"] {
        let value = no_proxy_with_localhost(std::env::var(key).ok().as_deref());
        // SAFETY: called at the very start of `run()`, before Tauri or any
        // other code starts threads.
        unsafe { std::env::set_var(key, value) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_list_when_missing() {
        assert_eq!(no_proxy_with_localhost(None), "127.0.0.1,localhost");
        assert_eq!(no_proxy_with_localhost(Some("")), "127.0.0.1,localhost");
    }

    #[test]
    fn appends_to_existing_list_without_duplicates() {
        assert_eq!(
            no_proxy_with_localhost(Some("example.com,localhost")),
            "example.com,localhost,127.0.0.1"
        );
        assert_eq!(
            no_proxy_with_localhost(Some("127.0.0.1, localhost")),
            "127.0.0.1, localhost"
        );
    }
}
