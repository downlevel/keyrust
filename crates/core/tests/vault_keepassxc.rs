//! Vault ↔ KeePassXC compatibility (Milestone 3a, V1–V5).
//!
//! Databases are created or changed through `keyrust_core::vault::Vault` and
//! then read back by KeePassXC (`keepassxc-cli`), the independent oracle.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::{Path, PathBuf};

use common::*;
use keyrust_core::error::VaultError;
use keyrust_core::model::{Argon2idParams, Cipher, CreateOptions, EntryDraft, EntryId, GroupId};
use keyrust_core::vault::Vault;
use secrecy::SecretString;

const PASSWORD: &str = "vault-test-password";

fn secret(s: &str) -> SecretString {
    SecretString::from(s.to_owned())
}

fn small_kdf() -> Argon2idParams {
    Argon2idParams {
        memory_mib: 16,
        iterations: 2,
        parallelism: 2,
    }
}

fn create(dir: &Path) -> (PathBuf, Vault) {
    let path = dir.join("created.kdbx");
    let vault = Vault::create(
        &path,
        CreateOptions {
            name: "Created by Keyrust".into(),
            password: secret(PASSWORD),
            cipher: Cipher::ChaCha20,
            kdf: small_kdf(),
        },
    )
    .expect("create");
    (path, vault)
}

fn draft(group: GroupId, title: &str, password: &str) -> EntryDraft {
    EntryDraft {
        id: None,
        group_id: group,
        title: title.into(),
        username: "alice".into(),
        password: secret(password),
        url: "https://bank.example".into(),
        notes: "PIN hint: ✓ ünïcode".into(),
    }
}

/// Export node (entry or group) whose `UUID` is `uuid`.
fn by_uuid<'a, 'i>(
    doc: &'a roxmltree::Document<'i>,
    tag: &str,
    uuid: uuid::Uuid,
) -> Option<roxmltree::Node<'a, 'i>> {
    doc.descendants().find(|n| {
        n.has_tag_name(tag)
            && !n.parent().is_some_and(|p| p.has_tag_name("History"))
            && text(*n, "UUID").is_some_and(|u| base64_decode(u) == uuid.as_bytes())
    })
}

fn history_passwords(entry: roxmltree::Node<'_, '_>) -> Vec<String> {
    child(entry, "History")
        .map(|h| {
            children(h, "Entry")
                .map(|v| {
                    field(v, "Password")
                        .map(|(p, _)| p.to_owned())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// V1 — a database built entirely with the Vault API reads back in KeePassXC.
#[test]
fn v1_keepassxc_reads_a_vault_built_database() {
    let Some(cli) = require_cli("v1_keepassxc_reads_a_vault_built_database") else {
        return;
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let (path, mut vault) = create(dir.path());
    let root = vault.tree().id;
    let work = vault.create_group(root, "Work").expect("group");
    let dev = vault.create_group(work, "Dev").expect("group");
    let bank = vault
        .save_entry(&draft(dev, "Bank", "first"))
        .expect("entry");
    for pw in ["second", "third"] {
        let mut d = vault.entry_for_edit(bank).expect("draft");
        d.password = secret(pw);
        vault.save_entry(&d).expect("update");
    }
    let old = vault.save_entry(&draft(root, "Old", "old")).expect("entry");
    vault.delete_entry(old).expect("to bin");
    vault.save().expect("save");

    let xml = cli.export_xml(&path, PASSWORD);
    let doc = parse(&xml);
    assert_eq!(text(meta(&doc), "DatabaseName"), Some("Created by Keyrust"));
    let work_node = by_uuid(&doc, "Group", work.0).expect("Work");
    assert_eq!(text(work_node, "Name"), Some("Work"));
    let dev_node = by_uuid(&doc, "Group", dev.0).expect("Dev");
    assert_eq!(dev_node.parent(), Some(work_node));

    let bank_node = by_uuid(&doc, "Entry", bank.0).expect("Bank");
    assert_eq!(bank_node.parent(), Some(dev_node));
    assert_eq!(field(bank_node, "Password"), Some(("third", true)));
    assert_eq!(field(bank_node, "UserName"), Some(("alice", false)));
    assert_eq!(
        field(bank_node, "Notes"),
        Some(("PIN hint: ✓ ünïcode", false))
    );
    assert_eq!(history_passwords(bank_node), ["first", "second"]);

    let bin_uuid = text(meta(&doc), "RecycleBinUUID").expect("RecycleBinUUID");
    let old_node = by_uuid(&doc, "Entry", old.0).expect("Old");
    let bin = old_node.parent().expect("parent");
    assert_eq!(text(bin, "UUID"), Some(bin_uuid));
    assert_eq!(text(meta(&doc), "RecycleBinEnabled"), Some("True"));
}

/// UUID line prefix (`…/Entry[n]`) of the entry with `uuid` in canonical lines.
fn entry_prefix(lines: &[String], uuid: uuid::Uuid) -> String {
    lines
        .iter()
        .find_map(|l| {
            let (path, value) = l.split_once(" = ")?;
            let base = path.strip_suffix("/UUID[0]")?;
            (base.ends_with(']')
                && !base.contains("/History[")
                && base64_decode(value.trim_matches('"')) == uuid.as_bytes())
            .then(|| base.to_owned())
        })
        .expect("entry in canonical lines")
}

/// V2 — editing one entry of the KeePassXC fixture changes nothing else.
#[test]
fn v2_editing_one_entry_keeps_everything_else() {
    let Some(cli) = require_cli("v2_editing_one_entry_keeps_everything_else") else {
        return;
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("rich.kdbx");
    std::fs::copy(fixture("keepassxc-rich.kdbx"), &path).expect("copy");

    let before_db = keepass::Database::open(
        &mut std::fs::File::open(&path).expect("open"),
        keepass::DatabaseKey::new().with_password(FIXTURE_PASSWORD),
    )
    .expect("keepass-rs");
    let before = cli.export_xml(&path, FIXTURE_PASSWORD);

    let mut vault = Vault::open(&path, secret(FIXTURE_PASSWORD)).expect("open");
    let github: EntryId = vault.search("github")[0];
    let mut d = vault.entry_for_edit(github).expect("draft");
    d.username = "someone-else".into();
    vault.save_entry(&d).expect("update");
    vault.save().expect("save");

    let after_db = keepass::Database::open(
        &mut std::fs::File::open(&path).expect("open"),
        keepass::DatabaseKey::new().with_password(FIXTURE_PASSWORD),
    )
    .expect("keepass-rs");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let after = cli.export_xml(&path, FIXTURE_PASSWORD);

    let before_lines = canonical_lines(
        &before,
        &resolve_attachments(
            &cli,
            &fixture("keepassxc-rich.kdbx"),
            &before_db,
            &before,
            FIXTURE_PASSWORD,
        ),
    );
    let after_lines = canonical_lines(
        &after,
        &resolve_attachments(&cli, &path, &after_db, &after, FIXTURE_PASSWORD),
    );
    let prefix = entry_prefix(&before_lines, github.0);
    assert_eq!(prefix, entry_prefix(&after_lines, github.0));
    let (lost, added) = diff(&before_lines, &after_lines);

    // Outside the edited entry nothing changed; inside it, the two existing
    // history versions are untouched and only the username, times and the
    // new third history version differ.
    let allowed = |line: &String| {
        line.starts_with(&format!("{prefix}/"))
            && !line.starts_with(&format!("{prefix}/History[0]/Entry[0]/"))
            && !line.starts_with(&format!("{prefix}/History[0]/Entry[1]/"))
    };
    let unexpected: Vec<_> = lost.iter().chain(&added).filter(|l| !allowed(l)).collect();
    assert!(
        unexpected.is_empty(),
        "unexpected changes:\n{unexpected:#?}"
    );
    assert!(added.iter().any(|l| l.contains("someone-else")));
    assert!(
        added.iter().all(|l| {
            l.starts_with(&format!("{prefix}/History[0]/Entry[2]/"))
                || l.starts_with(&format!("{prefix}/String["))
                || l.starts_with(&format!("{prefix}/Times[0]/"))
        }),
        "{added:#?}"
    );

    let doc = parse(&after);
    let node = by_uuid(&doc, "Entry", github.0).expect("GitHub");
    assert_eq!(history_passwords(node).len(), 3);
    assert_eq!(&history_passwords(node)[..2], ["gh-old-1", "gh-old-2"]);
}

/// V3 — recycle bin, restore and permanent deletion with tombstones.
#[test]
fn v3_recycle_bin_restore_and_tombstones() {
    let Some(cli) = require_cli("v3_recycle_bin_restore_and_tombstones") else {
        return;
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("rich.kdbx");
    std::fs::copy(fixture("keepassxc-rich.kdbx"), &path).expect("copy");
    let mut vault = Vault::open(&path, secret(FIXTURE_PASSWORD)).expect("open");

    let root = vault.tree().id;
    let parent = vault.create_group(root, "Doomed").expect("group");
    let inner = vault.create_group(parent, "Inner").expect("group");
    let deep = vault
        .save_entry(&draft(inner, "Deep", "pw"))
        .expect("entry");
    let gmail = vault.search("gmail")[0];
    let email_group = vault.get_entry(gmail).expect("gmail").summary.group_id;

    vault.delete_entry(gmail).expect("to bin");
    vault.restore_entry(gmail).expect("restore");
    let gone = vault.save_entry(&draft(root, "Gone", "pw")).expect("entry");
    vault.delete_entry(gone).expect("to bin");
    vault.delete_entry(gone).expect("permanently");
    vault.delete_group(parent).expect("to bin");
    vault.delete_group(parent).expect("permanently");
    vault.save().expect("save");

    let xml = cli.export_xml(&path, FIXTURE_PASSWORD);
    let doc = parse(&xml);
    let gmail_node = by_uuid(&doc, "Entry", gmail.0).expect("gmail restored");
    assert_eq!(
        gmail_node
            .parent()
            .and_then(|g| text(g, "UUID"))
            .map(base64_decode),
        Some(email_group.0.as_bytes().to_vec())
    );
    let deleted: Vec<Vec<u8>> = child(doc.root_element(), "Root")
        .and_then(|r| child(r, "DeletedObjects"))
        .map(|d| {
            children(d, "DeletedObject")
                .filter_map(|o| text(o, "UUID").map(base64_decode))
                .collect()
        })
        .unwrap_or_default();
    for uuid in [gone.0, parent.0, inner.0, deep.0] {
        assert!(
            deleted.contains(&uuid.as_bytes().to_vec()),
            "no tombstone for {uuid}"
        );
        assert!(by_uuid(&doc, "Entry", uuid).is_none() && by_uuid(&doc, "Group", uuid).is_none());
    }
    // The fixture's own tombstone is still there.
    assert!(deleted.contains(&base64_decode("mZmZmZmZmZmZmZmZmZmZmQ==")));
}

/// V4 — password, KDF and cipher changes; an AES-KDF file stays AES-KDF.
#[test]
fn v4_security_settings_changes() {
    let Some(cli) = require_cli("v4_security_settings_changes") else {
        return;
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let (path, mut vault) = create(dir.path());
    vault
        .change_master_password(secret("brand-new-password"))
        .expect("change");
    vault
        .set_kdf(Argon2idParams {
            memory_mib: 32,
            iterations: 3,
            parallelism: 1,
        })
        .expect("kdf");
    vault.set_cipher(Cipher::Aes256).expect("cipher");
    vault.save().expect("save");

    let info = cli.db_info(&path, "brand-new-password");
    assert!(info.contains("AES 256-bit"), "{info}");
    assert!(info.contains("Argon2id (3 rounds, 32768 KB)"), "{info}");
    let old = cli.run(&["db-info", "-q", path.to_str().expect("utf-8")], PASSWORD);
    assert!(!old.success, "the old password still opens the database");

    let rich = dir.path().join("rich.kdbx");
    std::fs::copy(fixture("keepassxc-rich.kdbx"), &rich).expect("copy");
    let before = cli.db_info(&rich, FIXTURE_PASSWORD);
    let mut vault = Vault::open(&rich, secret(FIXTURE_PASSWORD)).expect("open");
    vault.rename_database("Renamed").expect("rename");
    vault.save().expect("save");
    let after = cli.db_info(&rich, FIXTURE_PASSWORD);
    let kdf = |s: &str| s.lines().find(|l| l.contains("KDF")).map(str::to_owned);
    assert_eq!(kdf(&after), kdf(&before));
    assert!(kdf(&after).is_some_and(|l| l.contains("AES")), "{after}");
}

/// V5 — a change by KeePassXC blocks saving until Reload or Keep mine.
#[test]
fn v5_external_change_by_keepassxc() {
    let Some(cli) = require_cli("v5_external_change_by_keepassxc") else {
        return;
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let (path, mut vault) = create(dir.path());
    let root = vault.tree().id;
    vault.save_entry(&draft(root, "Mine", "pw")).expect("entry");

    let added = cli.run(
        &[
            "add",
            "-q",
            "-u",
            "bob",
            path.to_str().expect("utf-8"),
            "/From KeePassXC",
        ],
        PASSWORD,
    );
    assert!(added.success, "{}", added.stderr);

    assert!(matches!(vault.save(), Err(VaultError::ExternalChange)));
    vault.reload().expect("reload");
    assert_eq!(vault.search("keepassxc").len(), 1);
    assert!(vault.search("mine").is_empty());
    vault
        .save_entry(&draft(root, "Mine again", "pw"))
        .expect("entry");
    vault
        .save()
        .expect("save after reload (KeePassXC wrote KDBX 4.0)");

    let xml = cli.export_xml(&path, PASSWORD);
    assert!(xml.contains("From KeePassXC") && xml.contains("Mine again"));
}
