//! Guards against shipping test-only automation in release builds.

#[test]
fn e2e_webdriver_is_not_a_default_feature() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read Cargo.toml");
    let manifest: toml::Table = manifest.parse().expect("valid TOML");
    let defaults = manifest
        .get("features")
        .and_then(|f| f.get("default"))
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        !defaults.iter().any(|f| f.as_str() == Some("e2e")),
        "the e2e feature (embedded WebDriver server) must stay opt-in"
    );
    let webdriver = &manifest["dependencies"]["tauri-plugin-wdio-webdriver"];
    assert_eq!(
        webdriver.get("optional").and_then(|o| o.as_bool()),
        Some(true)
    );
}
