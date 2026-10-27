//! Guards the Tauri security configuration: minimal capabilities, only the
//! dialog and opener plugins (called from Rust, no webview permission), a
//! strict production CSP and a known set of IPC commands.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::Value;

fn read_json(relative: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

#[test]
fn only_the_default_capability_with_core_default_exists() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities");
    let files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(files, ["default.json"], "unexpected capability files");

    let capability = read_json("capabilities/default.json");
    assert_eq!(capability["windows"], serde_json::json!(["main"]));
    assert_eq!(
        capability["permissions"],
        serde_json::json!(["core:default"])
    );
    assert!(
        capability.get("remote").is_none(),
        "remote URLs must not get IPC access"
    );
}

#[test]
fn only_the_dialog_and_opener_plugins_without_webview_permissions() {
    let conf = read_json("tauri.conf.json");
    let plugins = conf.get("plugins").and_then(Value::as_object);
    assert!(plugins.is_none_or(|p| p.is_empty()), "plugins: {plugins:?}");

    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    let mut deps: Vec<&str> = manifest
        .lines()
        .filter_map(|l| l.trim().split_once(" = ").map(|(k, _)| k))
        .filter(|k| k.starts_with("tauri-plugin-"))
        .collect();
    deps.sort();
    assert_eq!(deps, ["tauri-plugin-dialog", "tauri-plugin-opener"]);

    // No capability grants any plugin permission (only core:default exists).
    let capability = read_json("capabilities/default.json");
    let permissions = capability["permissions"].as_array().unwrap();
    assert!(
        permissions
            .iter()
            .all(|p| p.as_str().is_some_and(|p| p.starts_with("core:"))),
        "{permissions:?}"
    );

    // The opener's link-click script is disabled: links go through open_url.
    let lib = include_str!("../src/lib.rs");
    assert!(lib.contains(".open_js_links_on_click(false)"));
}

#[test]
fn production_csp_is_strict() {
    let conf = read_json("tauri.conf.json");
    let security = &conf["app"]["security"];
    assert_eq!(security["freezePrototype"], Value::Bool(true));

    let csp = security["csp"]
        .as_object()
        .expect("CSP must be set as an object");
    let directive = |name: &str| csp.get(name).and_then(Value::as_str).unwrap_or_default();
    assert_eq!(directive("default-src"), "'self'");
    assert_eq!(directive("script-src"), "'self'");
    assert_eq!(directive("object-src"), "'none'");
    assert_eq!(directive("base-uri"), "'none'");
    assert_eq!(directive("frame-ancestors"), "'none'");
    for (name, value) in csp {
        let value = value.as_str().unwrap();
        assert!(!value.contains("unsafe-eval"), "{name} allows unsafe-eval");
        assert!(!value.contains('*'), "{name} uses a wildcard");
        for source in value.split_whitespace() {
            let remote = source.starts_with("http:") || source.starts_with("https:");
            assert!(
                !remote || source == "http://ipc.localhost",
                "{name} allows remote source {source}"
            );
        }
    }
    assert!(!directive("script-src").contains("unsafe-inline"));
}

#[test]
fn ipc_commands_are_the_expected_ones() {
    // Every command the webview can call; a new one must be added here on purpose.
    assert_eq!(
        keyrust_app_lib::COMMANDS,
        [
            "app_info",
            "get_settings",
            "update_settings",
            "exit_app",
            "recent_files",
            "choose_database_file",
            "choose_new_database_path",
            "unlock",
            "create",
            "lock_vault",
            "database_info",
            "tree",
            "list_entries",
            "get_entry",
            "get_entry_for_edit",
            "reveal_field",
            "copy_field",
            "save_entry",
            "delete_entry",
            "restore_entry",
            "create_group",
            "rename_group",
            "delete_group",
            "update_database",
            "change_master_password",
            "save",
            "save_overwrite",
            "reload",
            "check_file",
            "generate_password",
            "estimate_strength",
            "benchmark_kdf",
            "open_url",
        ]
    );
    let source = include_str!("../src/lib.rs");
    let handler = source
        .split("generate_handler![")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .expect("generate_handler! invocation");
    let registered: Vec<_> = handler
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(registered, keyrust_app_lib::COMMANDS);
}
