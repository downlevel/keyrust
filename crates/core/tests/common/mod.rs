//! Helpers for the KeePassXC compatibility spike.
//!
//! KeePassXC (through `keepassxc-cli`) is the independent oracle: what a file
//! contains is established from KeePassXC's own export, never from keepass-rs.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use chrono::{NaiveDate, NaiveDateTime, TimeDelta};
use keepass::Database;

/// Test-only password of the committed fixtures. Not a secret.
pub const FIXTURE_PASSWORD: &str = "keyrust-test-password";

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

// ---------------------------------------------------------------------------
// keepassxc-cli
// ---------------------------------------------------------------------------

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        [name.to_string(), format!("{name}.exe")]
            .into_iter()
            .map(|candidate| dir.join(candidate))
            .find(|p| p.is_file())
    })
}

/// Where the official KeePassXC installers put the CLI, which they do not add
/// to `PATH` on Windows and macOS.
const STANDARD_LOCATIONS: &[&str] = &[
    r"C:\Program Files\KeePassXC\keepassxc-cli.exe",
    "/Applications/KeePassXC.app/Contents/MacOS/keepassxc-cli",
];

/// Locates `keepassxc-cli`: `KEEPASSXC_CLI` if set, otherwise `PATH`, otherwise
/// the standard install locations.
///
/// A missing CLI **fails** the test: the harness hides the output of passing
/// tests, so a silent skip would look like a verification. Skipping is an
/// explicit opt-out (`KEYRUST_SKIP_KEEPASSXC=1`) that the person running the
/// tests chooses and therefore knows about.
pub fn require_cli(test: &str) -> Option<Cli> {
    if std::env::var("KEYRUST_SKIP_KEEPASSXC").as_deref() == Ok("1") {
        eprintln!("WARNING: {test} SKIPPED (not verified): KEYRUST_SKIP_KEEPASSXC=1");
        return None;
    }
    let found = match std::env::var_os("KEEPASSXC_CLI") {
        // An explicit path must be right: no silent fallback to another binary.
        Some(p) => {
            let p = PathBuf::from(p);
            assert!(
                p.is_file(),
                "{test}: KEEPASSXC_CLI={} is not a file",
                p.display()
            );
            Some(p)
        }
        None => find_in_path("keepassxc-cli").or_else(|| {
            STANDARD_LOCATIONS
                .iter()
                .map(PathBuf::from)
                .find(|p| p.is_file())
        }),
    };
    match found {
        Some(path) => Some(Cli { path }),
        None => panic!(
            "{test}: keepassxc-cli not found in PATH nor in {STANDARD_LOCATIONS:?}. \
             Install KeePassXC (not KeePass 2) or set KEEPASSXC_CLI to its path, \
             or set KEYRUST_SKIP_KEEPASSXC=1 to skip the KeePassXC checks knowingly."
        ),
    }
}

pub struct Cli {
    path: PathBuf,
}

pub struct CliOutput {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Cli {
    /// Runs a subcommand, passing the password on stdin (never as an argument).
    pub fn run(&self, args: &[&str], password: &str) -> CliOutput {
        let mut child = Command::new(&self.path)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn keepassxc-cli");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(format!("{password}\n").as_bytes())
            .expect("write password");
        let out = child.wait_with_output().expect("wait keepassxc-cli");
        CliOutput {
            success: out.status.success(),
            stdout: out.stdout,
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    }

    fn run_ok(&self, args: &[&str], password: &str) -> Vec<u8> {
        let out = self.run(args, password);
        assert!(out.success, "keepassxc-cli {args:?} failed: {}", out.stderr);
        out.stdout
    }

    pub fn export_xml(&self, db: &Path, password: &str) -> String {
        let db = db.to_str().expect("utf-8 path");
        String::from_utf8(self.run_ok(&["export", "-q", "-f", "xml", db], password))
            .expect("export is utf-8")
    }

    pub fn db_info(&self, db: &Path, password: &str) -> String {
        let db = db.to_str().expect("utf-8 path");
        String::from_utf8_lossy(&self.run_ok(&["db-info", "-q", db], password)).into_owned()
    }

    /// Exports an attachment to a file (`--stdout` mangles non-UTF-8 bytes).
    pub fn attachment(&self, db: &Path, entry: &str, name: &str, password: &str) -> Vec<u8> {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("attachment");
        let args = [
            "attachment-export",
            "-q",
            db.to_str().expect("utf-8 path"),
            entry,
            name,
            out.to_str().expect("utf-8 path"),
        ];
        self.run_ok(&args, password);
        std::fs::read(out).expect("read exported attachment")
    }
}

// ---------------------------------------------------------------------------
// Export XML access
// ---------------------------------------------------------------------------

pub fn parse(xml: &str) -> roxmltree::Document<'_> {
    roxmltree::Document::parse(xml).expect("export is well-formed XML")
}

pub fn child<'a, 'i>(node: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    node.children().find(|n| n.has_tag_name(name))
}

pub fn children<'a, 'i>(
    node: roxmltree::Node<'a, 'i>,
    name: &'a str,
) -> impl Iterator<Item = roxmltree::Node<'a, 'i>> + 'a {
    node.children().filter(move |n| n.has_tag_name(name))
}

pub fn text<'a>(node: roxmltree::Node<'a, '_>, name: &str) -> Option<&'a str> {
    child(node, name).map(|n| n.text().unwrap_or(""))
}

pub fn meta<'a, 'i>(doc: &'a roxmltree::Document<'i>) -> roxmltree::Node<'a, 'i> {
    child(doc.root_element(), "Meta").expect("Meta")
}

pub fn root_group<'a, 'i>(doc: &'a roxmltree::Document<'i>) -> roxmltree::Node<'a, 'i> {
    child(child(doc.root_element(), "Root").expect("Root"), "Group").expect("root group")
}

/// Finds the (current, not historical) entry with the given base64 UUID.
pub fn entry<'a, 'i>(doc: &'a roxmltree::Document<'i>, uuid: &str) -> roxmltree::Node<'a, 'i> {
    doc.descendants()
        .find(|n| {
            n.has_tag_name("Entry")
                && !n.parent().is_some_and(|p| p.has_tag_name("History"))
                && text(*n, "UUID") == Some(uuid)
        })
        .unwrap_or_else(|| panic!("entry {uuid} not found"))
}

pub fn group<'a, 'i>(doc: &'a roxmltree::Document<'i>, uuid: &str) -> roxmltree::Node<'a, 'i> {
    doc.descendants()
        .find(|n| n.has_tag_name("Group") && text(*n, "UUID") == Some(uuid))
        .unwrap_or_else(|| panic!("group {uuid} not found"))
}

/// `(value, protected)` of an entry string field.
pub fn field<'a>(entry: roxmltree::Node<'a, '_>, key: &str) -> Option<(&'a str, bool)> {
    children(entry, "String")
        .find(|s| text(*s, "Key") == Some(key))
        .map(|s| {
            let value = child(s, "Value").expect("Value");
            (
                value.text().unwrap_or(""),
                value.attribute("ProtectInMemory") == Some("True"),
            )
        })
}

pub fn custom_data<'a>(node: roxmltree::Node<'a, '_>) -> BTreeMap<&'a str, &'a str> {
    child(node, "CustomData")
        .map(|cd| {
            children(cd, "Item")
                .map(|i| (text(i, "Key").unwrap_or(""), text(i, "Value").unwrap_or("")))
                .collect()
        })
        .unwrap_or_default()
}

pub fn tags(entry: roxmltree::Node<'_, '_>) -> Vec<String> {
    let mut tags: Vec<String> = text(entry, "Tags")
        .unwrap_or("")
        .split([',', ';'])
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(String::from)
        .collect();
    tags.sort();
    tags
}

/// KDBX4 stores times as base64 little-endian seconds since 0001-01-01.
pub fn kdbx_time(b64: &str) -> NaiveDateTime {
    let bytes = base64_decode(b64);
    let secs = i64::from_le_bytes(bytes.try_into().expect("8-byte time"));
    NaiveDate::from_ymd_opt(1, 1, 1)
        .expect("epoch")
        .and_hms_opt(0, 0, 0)
        .expect("midnight")
        + TimeDelta::seconds(secs)
}

pub fn base64_decode(s: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes().filter(|c| *c != b'=' && !c.is_ascii_whitespace()) {
        let v = ALPHABET.iter().position(|a| *a == c).expect("base64 char") as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Canonical form for lossless comparison
// ---------------------------------------------------------------------------

/// Elements whose value a save legitimately changes. Every entry here is a
/// documented normalisation (see docs/spikes/0001-keepass-rs-kdbx4.md); any
/// other difference between two exports is a failure.
///
/// Empty on purpose: with keepass-rs 0.17.0 not even `Meta/Generator` changes.
pub const NORMALISED: &[&str] = &[];

/// CustomData item that `keepassxc-cli export` stamps with the time of the
/// export itself (not of the file), so two exports of one file differ.
pub const EXPORT_TIME_CUSTOM_DATA_KEY: &str = "_LAST_MODIFIED";

/// Flattens an export into sorted `path = value` lines.
///
/// Attachment references (`<Value Ref="n"/>`) are replaced by the attachment's
/// content, looked up by [`attachment_key`] in `contents` (built by
/// [`resolve_attachments`]). The `Ref` number itself is not compared: writers
/// may renumber the pool, and recent KeePassXC exports write `Ref="0"` for
/// every attachment. A missing attachment fails the test rather than being skipped.
pub fn canonical_lines(xml: &str, contents: &BTreeMap<AttachmentKey, Vec<u8>>) -> Vec<String> {
    let doc = parse(xml);
    let mut lines = Vec::new();
    walk(
        doc.root_element(),
        "/KeePassFile",
        "/KeePassFile",
        contents,
        &mut lines,
    );
    lines.sort();
    lines
}

/// `path` carries sibling positions (`/Group[0]/Entry[1]`) so that the order of
/// groups, entries and history versions matters; `kind` is the same path
/// without positions, used to match [`NORMALISED`].
fn walk(
    node: roxmltree::Node<'_, '_>,
    path: &str,
    kind: &str,
    contents: &BTreeMap<AttachmentKey, Vec<u8>>,
    lines: &mut Vec<String>,
) {
    if NORMALISED.contains(&kind) {
        return;
    }
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    let mut elements = Vec::new();
    let mut element_children: Vec<_> = node.children().filter(|c| c.is_element()).collect();
    if node.has_tag_name("CustomData") {
        // A key/value map: KeePassXC exports its items in unstable order.
        element_children.retain(|i| text(*i, "Key") != Some(EXPORT_TIME_CUSTOM_DATA_KEY));
        element_children.sort_by_key(|i| text(*i, "Key").unwrap_or("").to_string());
    }
    for c in element_children {
        let name = c.tag_name().name();
        let idx = seen.entry(name).or_default();
        elements.push((c, format!("{path}/{name}[{idx}]"), format!("{kind}/{name}")));
        *idx += 1;
    }
    for attr in node.attributes() {
        let value = if attr.name() == "Ref" {
            let binary = node.parent().expect("Binary owning the Ref");
            let key = attachment_key(binary);
            let content = contents
                .get(&key)
                .unwrap_or_else(|| panic!("unresolved attachment {key:?} at {path}"));
            format!("content:{}", hex(content))
        } else {
            attr.value().to_string()
        };
        lines.push(format!("{path}@{} = {value}", attr.name()));
    }
    if elements.is_empty() {
        lines.push(format!("{path} = {:?}", node.text().unwrap_or("")));
    }
    for (c, child_path, child_kind) in elements {
        walk(c, &child_path, &child_kind, contents, lines);
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Lines only in `a` and only in `b` (multiset difference), for readable diffs.
pub fn diff(a: &[String], b: &[String]) -> (Vec<String>, Vec<String>) {
    let mut only_b: Vec<String> = b.to_vec();
    let mut only_a = Vec::new();
    for line in a {
        if let Some(pos) = only_b.iter().position(|l| l == line) {
            only_b.remove(pos);
        } else {
            only_a.push(line.clone());
        }
    }
    (only_a, only_b)
}

/// Identifies an attachment without the export's `Ref` numbering: owning
/// entry UUID (base64, as exported), history version index (`None` for the
/// current entry) and attachment name.
pub type AttachmentKey = (String, Option<usize>, String);

/// [`AttachmentKey`] of a `<Binary>` element of an export.
pub fn attachment_key(binary: roxmltree::Node<'_, '_>) -> AttachmentKey {
    let entry = binary.parent().expect("Entry owning the Binary");
    let uuid = text(entry, "UUID").expect("Entry/UUID").to_string();
    let name = text(binary, "Key").expect("Binary/Key").to_string();
    let history = entry
        .parent()
        .filter(|p| p.has_tag_name("History"))
        .map(|h| {
            children(h, "Entry")
                .position(|e| e == entry)
                .expect("history version")
        });
    (uuid, history, name)
}

/// Content of every attachment of an export, keyed by [`AttachmentKey`].
///
/// Current entries are read through KeePassXC itself (`attachment-export`).
/// `keepassxc-cli` cannot export attachments of history versions, so those are
/// read with keepass-rs from `parsed` (the same file, already opened), and
/// T1 pins keepass-rs's reading of them on the KeePassXC fixture.
///
/// When the export's `Ref`s are real pool indices (KeePassXC 2.7.x), they are
/// still used as an independent check: every attachment sharing a `Ref` must
/// have the same content, history versions included. Recent KeePassXC exports
/// write `Ref="0"` for every attachment, which this check detects and skips.
pub fn resolve_attachments(
    cli: &Cli,
    db: &Path,
    parsed: &Database,
    xml: &str,
    password: &str,
) -> BTreeMap<AttachmentKey, Vec<u8>> {
    let doc = parse(xml);
    let mut contents = BTreeMap::new();
    let mut by_ref: BTreeMap<String, Vec<Vec<u8>>> = BTreeMap::new();
    for binary in doc.descendants().filter(|n| n.has_tag_name("Binary")) {
        let key = attachment_key(binary);
        let (uuid, history, name) = &key;
        let entry = binary.parent().expect("Entry");
        let content = match history {
            None => cli.attachment(db, &entry_path(entry), name, password),
            Some(index) => keepass_rs_history_attachment(parsed, uuid, *index, name),
        };
        let reference = child(binary, "Value")
            .and_then(|v| v.attribute("Ref"))
            .expect("Ref");
        by_ref
            .entry(reference.to_string())
            .or_default()
            .push(content.clone());
        assert!(
            contents.insert(key.clone(), content).is_none(),
            "duplicate attachment {key:?}"
        );
    }

    let current: BTreeMap<String, Vec<u8>> = doc
        .descendants()
        .filter(|n| n.has_tag_name("Binary"))
        .filter(|b| attachment_key(*b).1.is_none())
        .map(|b| {
            let reference = child(b, "Value").and_then(|v| v.attribute("Ref"));
            (
                reference.expect("Ref").to_string(),
                contents[&attachment_key(b)].clone(),
            )
        })
        .collect();
    let refs_are_pool_indices = by_ref
        .iter()
        .filter(|(r, _)| current.contains_key(*r))
        .all(|(r, all)| all.iter().all(|c| *c == current[r]));
    if refs_are_pool_indices {
        for (reference, all) in &by_ref {
            let expected = current
                .get(reference)
                .unwrap_or_else(|| panic!("Ref {reference} used only by history versions"));
            assert!(
                all.iter().all(|c| c == expected),
                "Ref {reference} resolves to different contents"
            );
        }
    }
    contents
}

fn keepass_rs_history_attachment(db: &Database, uuid: &str, index: usize, name: &str) -> Vec<u8> {
    let uuid = base64_decode(uuid);
    let entry = db
        .iter_all_entries()
        .find(|e| e.id().uuid().as_bytes().as_slice() == uuid.as_slice())
        .expect("entry of the history version");
    let version = entry.historical(index).expect("history version");
    version
        .attachments_named()
        .find(|(n, _)| *n == name)
        .map(|(_, a)| a.data.get().clone())
        .expect("history attachment")
}

/// `/Group/Sub/Title` path of an entry, as `keepassxc-cli` expects it (the root
/// group's name is not part of the path).
pub fn entry_path(entry: roxmltree::Node<'_, '_>) -> String {
    let mut names: Vec<&str> = entry
        .ancestors()
        .filter(|n| n.has_tag_name("Group"))
        .map(|g| text(g, "Name").unwrap_or(""))
        .collect();
    names.pop(); // root group
    names.reverse();
    let title = field(entry, "Title").map(|(t, _)| t).unwrap_or("");
    format!(
        "/{}",
        names
            .into_iter()
            .chain([title])
            .collect::<Vec<_>>()
            .join("/")
    )
    .replace("//", "/")
}
