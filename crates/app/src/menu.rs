//! Native application menu, built from the command registry shared with the
//! frontend (`ui/src/lib/commands/commands.json`) and labelled with its English
//! messages (`ui/messages/en.json`). A click emits the `menu` event with the
//! command id; the webview runs the same command as the keyboard shortcut.

use std::collections::HashMap;
use std::error::Error;

use serde::Deserialize;
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Runtime};

const REGISTRY_JSON: &str = include_str!("../ui/src/lib/commands/commands.json");
const MESSAGES_JSON: &str = include_str!("../ui/messages/en.json");

/// Event carrying the id of a clicked menu command to the webview.
pub const MENU_EVENT: &str = "menu";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    pub menus: Vec<MenuSpec>,
    pub commands: Vec<CommandSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuSpec {
    pub id: String,
    pub label_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandSpec {
    pub id: String,
    pub menu: Option<String>,
    /// On macOS, `"app"` moves the item to the application menu.
    #[serde(default)]
    pub mac_menu: Option<String>,
    pub label_key: String,
    pub shortcut: Option<String>,
    #[serde(default)]
    pub native_accelerator: bool,
    #[serde(default)]
    pub separator_before: bool,
}

pub fn registry() -> Result<Registry, serde_json::Error> {
    serde_json::from_str(REGISTRY_JSON)
}

/// English labels by message key (the `$schema` entry is skipped).
pub fn labels() -> Result<HashMap<String, String>, serde_json::Error> {
    let raw: HashMap<String, serde_json::Value> = serde_json::from_str(MESSAGES_JSON)?;
    Ok(raw
        .into_iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_owned())))
        .collect())
}

fn label<'a>(labels: &'a HashMap<String, String>, key: &str) -> Result<&'a str, Box<dyn Error>> {
    labels
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing message {key}").into())
}

/// Whether `command` sits in `menu` on this platform.
fn placed_in(command: &CommandSpec, menu: &str) -> bool {
    if cfg!(target_os = "macos") && command.mac_menu.is_some() {
        return command.mac_menu.as_deref() == Some(menu);
    }
    command.menu.as_deref() == Some(menu)
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> Result<Menu<R>, Box<dyn Error>> {
    let registry = registry()?;
    let labels = labels()?;

    let item = |c: &CommandSpec| -> Result<_, Box<dyn Error>> {
        let mut builder = MenuItemBuilder::with_id(c.id.as_str(), label(&labels, &c.label_key)?);
        if c.native_accelerator
            && let Some(shortcut) = &c.shortcut
        {
            builder = builder.accelerator(shortcut);
        }
        Ok(builder.build(app)?)
    };

    let mut menu = MenuBuilder::new(app);

    if cfg!(target_os = "macos") {
        let mut sub = SubmenuBuilder::new(app, label(&labels, "app_name")?);
        for c in registry.commands.iter().filter(|c| placed_in(c, "app")) {
            if c.id == "quit" {
                sub = sub
                    .separator()
                    .services()
                    .separator()
                    .hide()
                    .hide_others()
                    .show_all()
                    .separator();
            } else if c.separator_before {
                sub = sub.separator();
            }
            sub = sub.item(&item(c)?);
        }
        menu = menu.item(&sub.build()?);
    }

    for spec in &registry.menus {
        let mut sub = SubmenuBuilder::new(app, label(&labels, &spec.label_key)?);
        if spec.id == "edit" {
            // Predefined items keep copy and paste working in text fields
            // (on macOS they only work through the menu).
            sub = sub
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all();
        }
        let mut first = spec.id != "edit";
        for c in registry.commands.iter().filter(|c| placed_in(c, &spec.id)) {
            if c.separator_before && !first {
                sub = sub.separator();
            }
            first = false;
            sub = sub.item(&item(c)?);
        }
        menu = menu.item(&sub.build()?);
    }

    Ok(menu.build()?)
}

/// Forwards clicks on registry commands to the webview.
pub fn forward<R: Runtime>(app: &AppHandle<R>, id: &str, known: &[String]) {
    if known.iter().any(|k| k == id)
        && let Err(e) = app.emit(MENU_EVENT, id)
    {
        tracing::warn!(error = %e, "cannot forward menu command");
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const MODIFIERS: &[&str] = &["CmdOrCtrl", "Shift", "Alt"];
    const NAMED_KEYS: &[&str] = &["Slash", "Comma", "Delete", "Enter", "Escape"];

    fn valid_key(key: &str) -> bool {
        (key.len() == 1
            && key
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()))
            || NAMED_KEYS.contains(&key)
            || key
                .strip_prefix('F')
                .and_then(|n| n.parse::<u8>().ok())
                .is_some_and(|n| (1..=24).contains(&n))
    }

    #[test]
    fn registry_and_labels_parse() {
        let registry = registry().expect("commands.json");
        assert!(registry.commands.len() > 10);
        let labels = labels().expect("en.json");
        assert_eq!(labels.get("app_name").map(String::as_str), Some("Keyrust"));
    }

    #[test]
    fn ids_are_unique_and_menus_exist() {
        let registry = registry().expect("commands.json");
        let mut ids = HashSet::new();
        for c in &registry.commands {
            assert!(ids.insert(&c.id), "duplicate command id {}", c.id);
        }
        let menus: HashSet<_> = registry.menus.iter().map(|m| m.id.as_str()).collect();
        for c in &registry.commands {
            if let Some(menu) = &c.menu {
                assert!(
                    menus.contains(menu.as_str()),
                    "{} refers to menu {menu}",
                    c.id
                );
            }
            if let Some(menu) = &c.mac_menu {
                assert_eq!(menu, "app", "{}", c.id);
            }
        }
    }

    #[test]
    fn every_label_key_has_an_english_message() {
        let registry = registry().expect("commands.json");
        let labels = labels().expect("en.json");
        for key in registry
            .menus
            .iter()
            .map(|m| &m.label_key)
            .chain(registry.commands.iter().map(|c| &c.label_key))
        {
            assert!(labels.contains_key(key), "missing message {key}");
        }
    }

    #[test]
    fn accelerators_are_well_formed() {
        let registry = registry().expect("commands.json");
        for c in &registry.commands {
            let Some(shortcut) = &c.shortcut else {
                assert!(!c.native_accelerator, "{} has no shortcut", c.id);
                continue;
            };
            let mut parts: Vec<&str> = shortcut.split('+').collect();
            let key = parts.pop().unwrap_or_default();
            assert!(valid_key(key), "{}: key {key}", c.id);
            for modifier in parts {
                assert!(
                    MODIFIERS.contains(&modifier),
                    "{}: modifier {modifier}",
                    c.id
                );
            }
        }
    }

    #[test]
    fn native_accelerators_leave_text_editing_alone() {
        let registry = registry().expect("commands.json");
        let editing = [
            "CmdOrCtrl+C",
            "CmdOrCtrl+V",
            "CmdOrCtrl+X",
            "CmdOrCtrl+A",
            "CmdOrCtrl+Z",
            "Delete",
            "Enter",
        ];
        for c in registry.commands.iter().filter(|c| c.native_accelerator) {
            let shortcut = c.shortcut.as_deref().unwrap_or_default();
            assert!(!editing.contains(&shortcut), "{} grabs {shortcut}", c.id);
        }
    }
}
