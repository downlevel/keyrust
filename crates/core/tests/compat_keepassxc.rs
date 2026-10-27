//! Spike 0001: KDBX4 compatibility of keepass-rs with KeePassXC.
//!
//! See docs/spikes/0001-keepass-rs-kdbx4.md for the report. Expected values
//! below come from tests/fixtures/src/rich.xml (the fixture's source), never
//! from what keepass-rs reads back.

// Test code: a failed expectation is a test failure.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs::File;
use std::path::Path;

use common::*;
use keepass::config::{DatabaseConfig, InnerCipherConfig, KdfConfig, OuterCipherConfig};
use keepass::db::{CustomDataValue, Entry, History, Value, fields};
use keepass::{Database, DatabaseKey};

const RICH: &str = "keepassxc-rich.kdbx";

// UUIDs of tests/fixtures/src/rich.xml (16 repeated bytes, base64).
const ROOT: &str = "AQEBAQEBAQEBAQEBAQEBAQ==";
const WORK: &str = "AgICAgICAgICAgICAgICAg==";
const DEV: &str = "AwMDAwMDAwMDAwMDAwMDAw==";
const RECYCLE_BIN: &str = "BQUFBQUFBQUFBQUFBQUFBQ==";
const GITHUB: &str = "EREREREREREREREREREREQ==";
const GMAIL: &str = "EhISEhISEhISEhISEhISEg==";
const OLD_ACCOUNT: &str = "ExMTExMTExMTExMTExMTEw==";
const ICON: &str = "ISEhISEhISEhISEhISEhIQ==";
const TOMBSTONE: &str = "mZmZmZmZmZmZmZmZmZmZmQ==";

const NOTES: &str = "line one\nlìne twö 🚀\n第三行 & <tags>";
const README: &[u8] = b"hello keyrust\n";
const BLOB: &[u8] = &[0, 255, 16, 128, 254, 1, 2, 3];
const PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

fn key(password: &str) -> DatabaseKey {
    DatabaseKey::new().with_password(password)
}

fn open(path: &Path, password: &str) -> Database {
    Database::open(&mut File::open(path).expect("open file"), key(password))
        .expect("keepass-rs opens the database")
}

fn save(db: &Database, path: &Path, password: &str) {
    let mut file = File::create(path).expect("create file");
    db.save(&mut file, key(password))
        .expect("keepass-rs saves the database");
}

/// Asserts, from a KeePassXC export of a database derived from rich.xml, every
/// capability the spike covers. Used on the imported fixture (T0) and on the
/// keepass-rs re-saved copy (T2), so an empty-equals-empty comparison cannot pass.
fn assert_rich_capabilities(cli: &Cli, db: &Path, xml: &str) {
    let doc = parse(xml);
    let meta = meta(&doc);

    // Groups: nested tree with notes.
    let root = root_group(&doc);
    assert_eq!(text(root, "UUID"), Some(ROOT));
    assert_eq!(text(root, "Name"), Some("Keyrust Fixture"));
    let work = group(&doc, WORK);
    assert_eq!(text(work, "Name"), Some("Work"));
    assert_eq!(text(work, "Notes"), Some("Work accounts"));
    let dev = group(&doc, DEV);
    assert_eq!(text(dev, "Name"), Some("Dev"));
    assert_eq!(dev.parent().and_then(|p| text(p, "UUID")), Some(WORK));

    // Standard fields, Unicode and multi-line notes.
    let gh = entry(&doc, GITHUB);
    assert_eq!(gh.parent().and_then(|p| text(p, "UUID")), Some(DEV));
    assert_eq!(field(gh, "Title"), Some(("GitHub", false)));
    assert_eq!(field(gh, "UserName"), Some(("mauro", false)));
    assert_eq!(field(gh, "Password"), Some(("gh-current-ω-3", true)));
    assert_eq!(field(gh, "URL"), Some(("https://github.com", false)));
    assert_eq!(field(gh, "Notes"), Some((NOTES, false)));
    let gh_times = child(gh, "Times").expect("Times");
    assert_eq!(
        kdbx_time(text(gh_times, "CreationTime").expect("CreationTime")).to_string(),
        "2026-01-01 10:00:00"
    );
    assert_eq!(
        kdbx_time(text(gh_times, "LastModificationTime").expect("LastModificationTime"))
            .to_string(),
        "2026-01-04 10:00:00"
    );

    // Custom fields with their protection flag.
    assert_eq!(field(gh, "Recovery code"), Some(("RC-1234-5678", true)));
    assert_eq!(field(gh, "Environment"), Some(("production", false)));

    // Tags.
    assert_eq!(tags(gh), ["dev", "work"]);

    // Attachments, by content as KeePassXC exports them.
    let names: Vec<_> = children(gh, "Binary")
        .filter_map(|b| text(b, "Key"))
        .collect();
    assert_eq!(names, ["blob.bin", "readme.txt"]);
    assert_eq!(
        cli.attachment(db, "/Work/Dev/GitHub", "readme.txt", FIXTURE_PASSWORD),
        README
    );
    assert_eq!(
        cli.attachment(db, "/Work/Dev/GitHub", "blob.bin", FIXTURE_PASSWORD),
        BLOB
    );

    // History: two previous versions, oldest first, each with its own password
    // and the attachment it had.
    let history: Vec<_> = children(child(gh, "History").expect("History"), "Entry").collect();
    assert_eq!(history.len(), 2);
    assert_eq!(field(history[0], "Password"), Some(("gh-old-1", true)));
    assert_eq!(field(history[1], "Password"), Some(("gh-old-2", true)));
    for version in &history {
        assert_eq!(text(*version, "UUID"), Some(GITHUB));
        assert!(field(*version, "Recovery code").is_none());
        let refs: Vec<_> = children(*version, "Binary")
            .filter_map(|b| text(b, "Key"))
            .collect();
        assert_eq!(refs, ["readme.txt"]);
    }
    // The history attachment shares the pool slot of the current readme.txt.
    let readme_ref = |e: roxmltree::Node<'_, '_>| {
        children(e, "Binary")
            .find(|b| text(*b, "Key") == Some("readme.txt"))
            .and_then(|b| child(b, "Value")?.attribute("Ref"))
            .map(str::to_owned)
    };
    assert_eq!(readme_ref(history[0]), readme_ref(gh));

    // AutoType (preserved even though Keyrust will not implement auto-type).
    let autotype = child(gh, "AutoType").expect("AutoType");
    assert_eq!(text(autotype, "Enabled"), Some("True"));
    assert_eq!(
        text(autotype, "DefaultSequence"),
        Some("{USERNAME}{TAB}{PASSWORD}{ENTER}")
    );
    let association = child(autotype, "Association").expect("Association");
    assert_eq!(text(association, "Window"), Some("GitHub*"));
    assert_eq!(
        text(association, "KeystrokeSequence"),
        Some("{PASSWORD}{ENTER}")
    );

    // CustomData on the entry and on the database.
    assert_eq!(
        custom_data(gh).get("KEYRUST_ENTRY_KEY"),
        Some(&"entry-custom-value")
    );
    assert_eq!(
        custom_data(meta).get("KEYRUST_DB_KEY"),
        Some(&"db-custom-value")
    );

    // Custom icon, defined in Meta and used by the entry.
    assert_eq!(text(gh, "CustomIconUUID"), Some(ICON));
    let icon = children(child(meta, "CustomIcons").expect("CustomIcons"), "Icon")
        .find(|i| text(*i, "UUID") == Some(ICON))
        .expect("custom icon");
    assert_eq!(text(icon, "Data"), Some(PNG_B64));
    assert_eq!(text(icon, "Name"), Some("dot"));

    // Recycle bin, holding the deleted entry.
    assert_eq!(text(meta, "RecycleBinEnabled"), Some("True"));
    assert_eq!(text(meta, "RecycleBinUUID"), Some(RECYCLE_BIN));
    let old = entry(&doc, OLD_ACCOUNT);
    assert_eq!(
        old.parent().and_then(|p| text(p, "UUID")),
        Some(RECYCLE_BIN)
    );

    // Deletion tombstone.
    let deleted = child(
        child(doc.root_element(), "Root").expect("Root"),
        "DeletedObjects",
    )
    .expect("DeletedObjects");
    let tombstones: Vec<_> = children(deleted, "DeletedObject").collect();
    assert_eq!(tombstones.len(), 1);
    assert_eq!(text(tombstones[0], "UUID"), Some(TOMBSTONE));
    assert_eq!(
        kdbx_time(text(tombstones[0], "DeletionTime").expect("DeletionTime")).to_string(),
        "2026-03-01 12:00:00"
    );

    // The simple entry in another group.
    let gmail = entry(&doc, GMAIL);
    assert_eq!(field(gmail, "UserName"), Some(("mauro@example.com", false)));
    assert_eq!(field(gmail, "Password"), Some(("gm-p@ss", true)));
}

/// T0 — the KeePassXC-created fixture really contains every capability the
/// later tests claim to cover.
#[test]
fn t0_fixture_contains_every_capability() {
    let Some(cli) = require_cli("t0_fixture_contains_every_capability") else {
        return;
    };
    let path = fixture(RICH);
    let xml = cli.export_xml(&path, FIXTURE_PASSWORD);
    assert_rich_capabilities(&cli, &path, &xml);
}

/// T1 — keepass-rs reads the KeePassXC-created fixture correctly.
#[test]
fn t1_keepass_rs_reads_keepassxc_fixture() {
    let db = open(&fixture(RICH), FIXTURE_PASSWORD);
    let root = db.root();
    assert_eq!(root.name, "Keyrust Fixture");
    assert_eq!(db.meta.database_name.as_deref(), Some("Keyrust Fixture"));

    let work = root.group_by_name("Work").expect("Work group");
    assert_eq!(work.notes.as_deref(), Some("Work accounts"));
    let dev = root
        .group_by_path(&["Work", "Dev"])
        .expect("Work/Dev group");
    let gh = dev.entry_by_name("GitHub").expect("GitHub entry");

    assert_eq!(gh.get_username(), Some("mauro"));
    assert_eq!(gh.get_password(), Some("gh-current-ω-3"));
    assert_eq!(gh.get_url(), Some("https://github.com"));
    assert_eq!(gh.get(fields::NOTES), Some(NOTES));
    assert!(gh.fields[fields::PASSWORD].is_protected());
    assert_eq!(gh.get("Recovery code"), Some("RC-1234-5678"));
    assert!(gh.fields["Recovery code"].is_protected());
    assert_eq!(gh.get("Environment"), Some("production"));
    assert!(!gh.fields["Environment"].is_protected());
    assert_eq!(
        gh.times.creation.map(|t| t.to_string()).as_deref(),
        Some("2026-01-01 10:00:00")
    );

    let mut tags = gh.tags.clone();
    tags.sort();
    assert_eq!(tags, ["dev", "work"]);

    let mut attachments: Vec<(String, Vec<u8>)> = gh
        .attachments_named()
        .map(|(name, a)| (name.to_string(), a.data.get().clone()))
        .collect();
    attachments.sort();
    assert_eq!(
        attachments,
        [
            ("blob.bin".to_string(), BLOB.to_vec()),
            ("readme.txt".to_string(), README.to_vec())
        ]
    );

    // History: exactly two versions, in file order (oldest first), each with
    // its own fields, protection flags, timestamps and attachment.
    let history = gh.history.as_ref().expect("history").get_entries();
    assert_eq!(history.len(), 2);
    for (index, (password, modified)) in [
        ("gh-old-1", "2026-01-02 10:00:00"),
        ("gh-old-2", "2026-01-03 10:00:00"),
    ]
    .into_iter()
    .enumerate()
    {
        let version = gh.historical(index).expect("historical version");
        assert_eq!(version.get_title(), Some("GitHub"));
        assert_eq!(version.get_username(), Some("mauro"));
        assert_eq!(version.get_password(), Some(password));
        assert!(version.fields[fields::PASSWORD].is_protected());
        assert_eq!(version.get_url(), Some("https://github.com"));
        assert_eq!(version.get(fields::NOTES), Some(NOTES));
        assert!(version.get("Recovery code").is_none());
        assert!(version.get("Environment").is_none());
        assert_eq!(
            version
                .times
                .last_modification
                .map(|t| t.to_string())
                .as_deref(),
            Some(modified)
        );
        let attachments: Vec<(String, Vec<u8>)> = version
            .attachments_named()
            .map(|(name, a)| (name.to_string(), a.data.get().clone()))
            .collect();
        assert_eq!(attachments, [("readme.txt".to_string(), README.to_vec())]);
    }
    assert!(gh.historical(2).is_none());

    let autotype = gh.autotype.as_ref().expect("autotype");
    assert!(autotype.enabled);
    assert_eq!(
        autotype.default_sequence.as_deref(),
        Some("{USERNAME}{TAB}{PASSWORD}{ENTER}")
    );
    assert_eq!(autotype.associations.len(), 1);
    assert_eq!(autotype.associations[0].window, "GitHub*");
    assert_eq!(autotype.associations[0].sequence, "{PASSWORD}{ENTER}");

    let custom_value = |item: Option<&keepass::db::CustomDataItem>| match item?.value.as_ref()? {
        CustomDataValue::String(s) => Some(s.clone()),
        CustomDataValue::Binary(_) => None,
    };
    assert_eq!(
        custom_value(gh.custom_data.get("KEYRUST_ENTRY_KEY")).as_deref(),
        Some("entry-custom-value")
    );
    assert_eq!(
        custom_value(db.meta.custom_data.get("KEYRUST_DB_KEY")).as_deref(),
        Some("db-custom-value")
    );

    let icon = gh.custom_icon().expect("custom icon");
    assert_eq!(icon.name.as_deref(), Some("dot"));

    let bin = db.recycle_bin().expect("recycle bin");
    assert_eq!(bin.name, "Recycle Bin");
    assert!(bin.entry_by_name("Old account").is_some());
    assert_eq!(db.meta.recyclebin_enabled, Some(true));

    let tombstone = base64_decode(TOMBSTONE);
    let (uuid, time) = db.deleted_objects.iter().next().expect("tombstone");
    assert_eq!(db.deleted_objects.len(), 1);
    assert_eq!(uuid.as_bytes().as_slice(), tombstone.as_slice());
    assert_eq!(
        time.map(|t| t.to_string()).as_deref(),
        Some("2026-03-01 12:00:00")
    );
}

/// T2 — opening the fixture with keepass-rs and saving it again loses nothing
/// that KeePassXC can see.
#[test]
fn t2_round_trip_is_lossless() {
    let Some(cli) = require_cli("t2_round_trip_is_lossless") else {
        return;
    };
    let original = fixture(RICH);
    let dir = tempfile::tempdir().expect("tempdir");
    let resaved = dir.path().join("resaved.kdbx");
    save(
        &open(&original, FIXTURE_PASSWORD),
        &resaved,
        FIXTURE_PASSWORD,
    );

    let before = cli.export_xml(&original, FIXTURE_PASSWORD);
    // Exports stamp some values with the current time; make sure the two
    // exports never share a second, so such values cannot hide in the diff.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let after = cli.export_xml(&resaved, FIXTURE_PASSWORD);

    // Both copies must carry every capability, so equality is meaningful.
    assert_rich_capabilities(&cli, &original, &before);
    assert_rich_capabilities(&cli, &resaved, &after);

    let before_lines = canonical_lines(
        &before,
        &resolve_refs(&cli, &original, &before, FIXTURE_PASSWORD),
    );
    let after_lines = canonical_lines(
        &after,
        &resolve_refs(&cli, &resaved, &after, FIXTURE_PASSWORD),
    );
    let (lost, added) = diff(&before_lines, &after_lines);
    assert!(
        lost.is_empty() && added.is_empty(),
        "round trip changed the database\n--- only before ---\n{}\n--- only after ---\n{}",
        lost.join("\n"),
        added.join("\n")
    );
}

/// Builds a database from scratch with keepass-rs using the given KDBX4 settings.
fn create_database(outer: OuterCipherConfig) -> Database {
    let mut config = DatabaseConfig::default();
    config.outer_cipher_config = outer;
    config.inner_cipher_config = InnerCipherConfig::ChaCha20;
    // Deliberately small Argon2id parameters to keep tests fast.
    config.kdf_config = KdfConfig::Argon2id {
        iterations: 2,
        memory: 16 * 1024 * 1024,
        parallelism: 2,
        version: argon2::Version::Version13,
    };
    let mut db = Database::with_config(config);
    db.meta.database_name = Some("Created by keepass-rs".into());
    db.meta.recyclebin_enabled = Some(true);

    let mut root = db.root_mut();
    root.name = "Root".into();
    let mut group = root.add_group();
    group.name = "Personal".into();
    let mut sub = group.add_group();
    sub.name = "Banking".into();
    let mut entry = sub.add_entry();
    entry.set_unprotected(fields::TITLE, "Bank");
    entry.set_unprotected(fields::USERNAME, "alice");
    entry.set_protected(fields::PASSWORD, "first-password");
    entry.set_unprotected(fields::URL, "https://bank.example");
    entry.set_unprotected(fields::NOTES, "PIN hint: ✓ ünïcode");
    entry.set_protected("PIN", "4321");
    entry.add_attachment("statement.bin", Value::unprotected(BLOB.to_vec()));

    // Record the current version in the history, then change the password.
    let previous: Entry = (*entry).clone();
    let mut history = History::default();
    history.add_entry(previous);
    entry.history = Some(history);
    entry.set_protected(fields::PASSWORD, "second-password");
    db
}

fn assert_created_database(cli: &Cli, path: &Path, cipher_label: &str) {
    let xml = cli.export_xml(path, FIXTURE_PASSWORD);
    let doc = parse(&xml);
    let bank = doc
        .descendants()
        .find(|n| {
            n.has_tag_name("Entry")
                && !n.parent().is_some_and(|p| p.has_tag_name("History"))
                && field(*n, "Title") == Some(("Bank", false))
        })
        .expect("Bank entry");
    assert_eq!(entry_path(bank), "/Personal/Banking/Bank");
    assert_eq!(field(bank, "UserName"), Some(("alice", false)));
    assert_eq!(field(bank, "Password"), Some(("second-password", true)));
    assert_eq!(field(bank, "URL"), Some(("https://bank.example", false)));
    assert_eq!(field(bank, "Notes"), Some(("PIN hint: ✓ ünïcode", false)));
    assert_eq!(field(bank, "PIN"), Some(("4321", true)));
    assert_eq!(
        cli.attachment(
            path,
            "/Personal/Banking/Bank",
            "statement.bin",
            FIXTURE_PASSWORD
        ),
        BLOB
    );
    let history: Vec<_> = children(child(bank, "History").expect("History"), "Entry").collect();
    assert_eq!(history.len(), 1);
    assert_eq!(
        field(history[0], "Password"),
        Some(("first-password", true))
    );

    let info = cli.db_info(path, FIXTURE_PASSWORD);
    assert!(
        info.contains("Name: Created by keepass-rs"),
        "db-info: {info}"
    );
    assert!(
        info.contains(&format!("Cipher: {cipher_label}")),
        "db-info: {info}"
    );
    assert!(info.contains("KDF: Argon2id"), "db-info: {info}");
}

/// T3 — a database created by keepass-rs (Argon2id, AES-256) opens in KeePassXC.
#[test]
fn t3_keepassxc_opens_keepass_rs_database_aes() {
    let Some(cli) = require_cli("t3_keepassxc_opens_keepass_rs_database_aes") else {
        return;
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("created-aes.kdbx");
    save(
        &create_database(OuterCipherConfig::AES256),
        &path,
        FIXTURE_PASSWORD,
    );
    assert_created_database(&cli, &path, "AES 256-bit");
}

/// T3 — a database created by keepass-rs (Argon2id, ChaCha20) opens in KeePassXC.
#[test]
fn t3_keepassxc_opens_keepass_rs_database_chacha20() {
    let Some(cli) = require_cli("t3_keepassxc_opens_keepass_rs_database_chacha20") else {
        return;
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("created-chacha20.kdbx");
    save(
        &create_database(OuterCipherConfig::ChaCha20),
        &path,
        FIXTURE_PASSWORD,
    );
    assert_created_database(&cli, &path, "ChaCha20 256-bit");
}

/// T4 — keepass-rs rejects a wrong password with an error, not a panic.
#[test]
fn t4_keepass_rs_rejects_wrong_password() {
    let result = Database::open(
        &mut File::open(fixture(RICH)).expect("open fixture"),
        key("not-the-password"),
    );
    assert!(result.is_err(), "wrong password must be rejected");
}

/// T4 — KeePassXC rejects a keepass-rs database opened with a wrong password.
#[test]
fn t4_keepassxc_rejects_wrong_password() {
    let Some(cli) = require_cli("t4_keepassxc_rejects_wrong_password") else {
        return;
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("created.kdbx");
    save(
        &create_database(OuterCipherConfig::AES256),
        &path,
        FIXTURE_PASSWORD,
    );
    let out = cli.run(
        &["export", "-q", path.to_str().expect("utf-8 path")],
        "not-the-password",
    );
    assert!(!out.success, "KeePassXC must reject the wrong password");
    assert!(
        out.stdout.is_empty(),
        "no content may leak on a wrong password"
    );
}
