//! Harness for the structured fuzz target (`fuzz/fuzz_targets/vault_roundtrip.rs`),
//! behind the `fuzzing` feature. It builds a database from arbitrary content,
//! saves it with keepass-rs and reopens it through `Vault`: random mutations of
//! an encrypted file stop at the HMAC check, this reaches the XML writer and
//! parser and the decompression with extreme content.
//!
//! The oracle compares everything the harness generates: the group tree,
//! every field with its protection flag, attachments and history, both through
//! `Vault`'s views and on the reopened keepass-rs model.

use std::collections::BTreeMap;

use arbitrary::Arbitrary;
use keepass::config::{DatabaseConfig, KdfConfig};
use keepass::db::{GroupRef, History, Value, fields};
use keepass::{Database, DatabaseKey};
use secrecy::{ExposeSecret, SecretString};

use crate::kdbx::limits::Limits;
use crate::model::Field;
use crate::vault::Vault;

pub const PASSWORD: &str = "fuzz";

const MAX_GROUPS: usize = 8;
const MAX_ENTRIES: usize = 16;
const MAX_CUSTOM: usize = 4;
const MAX_HISTORY: usize = 3;
const MAX_TEXT: usize = 512;
const MAX_ATTACHMENT: usize = 1024;

#[derive(Debug, Arbitrary)]
pub struct Model {
    groups: Vec<GroupSpec>,
    entries: Vec<EntrySpec>,
}

#[derive(Debug, Arbitrary)]
struct GroupSpec {
    name: String,
    parent: u8,
}

#[derive(Debug, Arbitrary)]
struct EntrySpec {
    group: u8,
    title: String,
    username: String,
    password: String,
    url: String,
    notes: String,
    custom: Vec<(String, String, bool)>,
    history: Vec<String>,
    attachment: Option<(String, Vec<u8>)>,
}

/// XML 1.0 cannot carry most control characters, and parsers turn `\r` into
/// `\n`: content is reduced to what any KDBX writer can round-trip.
///
/// Known keepass-rs 0.17.0 behaviour (found by this harness, to report
/// upstream): a value made only of whitespace reads back as empty. Such values
/// are generated as empty so the fuzzer keeps looking for other differences.
fn xml_safe(s: &str) -> String {
    let safe: String = s
        .chars()
        .take(MAX_TEXT)
        .map(|c| if c == '\r' { '\n' } else { c })
        .filter(|&c| c == '\t' || c == '\n' || (c >= ' ' && c != '\u{FFFE}' && c != '\u{FFFF}'))
        .collect();
    if safe.trim().is_empty() {
        String::new()
    } else {
        safe
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Expected {
    title: String,
    username: String,
    password: String,
    url: String,
    notes: String,
}

/// Field name → (value, protected). Empty values are left out: KDBX writers
/// do not distinguish an empty field from an absent one.
type Fields = BTreeMap<String, (String, bool)>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct EntrySnapshot {
    /// Names of the groups from the root (excluded) down to the entry.
    path: Vec<String>,
    fields: Fields,
    attachments: BTreeMap<String, Vec<u8>>,
    history: Vec<Fields>,
}

/// The parts of a database the harness generates, in a comparable form:
/// every group path, and every entry with its place in the tree.
#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    groups: Vec<Vec<String>>,
    entries: Vec<EntrySnapshot>,
}

fn fields_of(fields: &std::collections::HashMap<String, Value<String>>) -> Fields {
    fields
        .iter()
        .filter(|(_, v)| !v.get().is_empty())
        .map(|(k, v)| (k.clone(), (v.get().clone(), v.is_protected())))
        .collect()
}

fn snapshot(db: &Database) -> Snapshot {
    fn walk(group: GroupRef<'_>, path: &[String], out: &mut Snapshot) {
        for entry in group.entries() {
            out.entries.push(EntrySnapshot {
                path: path.to_vec(),
                fields: fields_of(&entry.fields),
                attachments: entry
                    .attachments_named()
                    .map(|(name, a)| (name.to_owned(), a.data.get().clone()))
                    .collect(),
                history: entry
                    .history
                    .as_ref()
                    .map(|h| {
                        h.get_entries()
                            .iter()
                            .map(|e| fields_of(&e.fields))
                            .collect()
                    })
                    .unwrap_or_default(),
            });
        }
        for child in group.groups() {
            let mut child_path = path.to_vec();
            child_path.push(child.name.clone());
            out.groups.push(child_path.clone());
            walk(child, &child_path, out);
        }
    }
    let mut out = Snapshot {
        groups: Vec::new(),
        entries: Vec::new(),
    };
    walk(db.root(), &[], &mut out);
    out.groups.sort();
    out.entries.sort();
    out
}

/// Saves `model` and reopens it; `Err` describes a difference or failure.
pub fn roundtrip(model: &Model) -> Result<(), String> {
    let mut config = DatabaseConfig::default();
    config.kdf_config = KdfConfig::Aes { rounds: 1 };
    let mut db = Database::with_config(config);

    let mut groups = vec![db.root().id()];
    for spec in model.groups.iter().take(MAX_GROUPS) {
        let parent = groups[usize::from(spec.parent) % groups.len()];
        let mut parent = db.group_mut(parent).ok_or("parent group")?;
        let mut group = parent.add_group();
        group.name = xml_safe(&spec.name);
        groups.push(group.id());
    }

    let mut expected = Vec::new();
    for spec in model.entries.iter().take(MAX_ENTRIES) {
        let group = groups[usize::from(spec.group) % groups.len()];
        let mut parent = db.group_mut(group).ok_or("group")?;
        let mut entry = parent.add_entry();
        let e = Expected {
            title: xml_safe(&spec.title),
            username: xml_safe(&spec.username),
            password: xml_safe(&spec.password),
            url: xml_safe(&spec.url),
            notes: xml_safe(&spec.notes),
        };
        entry.set_unprotected(fields::TITLE, e.title.clone());
        entry.set_unprotected(fields::USERNAME, e.username.clone());
        entry.set_protected(fields::PASSWORD, e.password.clone());
        entry.set_unprotected(fields::URL, e.url.clone());
        entry.set_unprotected(fields::NOTES, e.notes.clone());
        for (key, value, protected) in spec.custom.iter().take(MAX_CUSTOM) {
            let key = xml_safe(key);
            let standard = [
                fields::TITLE,
                fields::USERNAME,
                fields::PASSWORD,
                fields::URL,
                fields::NOTES,
            ];
            if key.trim().is_empty()
                || standard.contains(&key.as_str())
                || entry.get(&key).is_some()
            {
                continue;
            }
            let value = xml_safe(value);
            if *protected {
                entry.set_protected(key, value);
            } else {
                entry.set_unprotected(key, value);
            }
        }
        if let Some((name, data)) = &spec.attachment {
            let name = xml_safe(name);
            if !name.trim().is_empty() {
                let data = data
                    .iter()
                    .copied()
                    .take(MAX_ATTACHMENT)
                    .collect::<Vec<u8>>();
                entry.add_attachment(name, Value::unprotected(data));
            }
        }
        if !spec.history.is_empty() {
            let mut history = History::default();
            for old in spec.history.iter().take(MAX_HISTORY) {
                let mut version = (*entry).clone();
                version.history = None;
                version.set_protected(fields::PASSWORD, xml_safe(old));
                history.add_entry(version);
            }
            entry.history = Some(history);
        }
        expected.push(e);
    }

    let mut bytes = Vec::new();
    db.save(&mut bytes, DatabaseKey::new().with_password(PASSWORD))
        .map_err(|e| format!("save: {e}"))?;
    let vault = Vault::open_bytes_with_limits(&bytes, SecretString::from(PASSWORD), &Limits::FUZZ)
        .map_err(|e| format!("reopen: {e}"))?;

    let (written, read) = (snapshot(&db), snapshot(vault.database()));
    if written != read {
        return Err(format!(
            "round trip changed the database:\n{written:#?}\n{read:#?}"
        ));
    }

    let mut actual = Vec::new();
    for summary in vault.list_entries() {
        let reveal = |field| {
            vault
                .reveal_field(summary.id, field)
                .map(|s| s.expose_secret().to_owned())
                .map_err(|e| format!("reveal: {e}"))
        };
        actual.push(Expected {
            title: summary.title.clone(),
            username: summary.username.clone(),
            password: reveal(Field::Password)?,
            url: summary.url.clone(),
            notes: reveal(Field::Notes)?,
        });
    }
    expected.sort();
    actual.sort();
    if expected != actual {
        return Err(format!(
            "round trip changed the entries:\n{expected:#?}\n{actual:#?}"
        ));
    }
    Ok(())
}
