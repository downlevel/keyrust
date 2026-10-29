use std::path::PathBuf;
use std::time::Instant;

use secrecy::{ExposeSecret, SecretString};

use super::*;
use crate::file::{Phase, hook};
use crate::model::{Argon2idParams, Cipher, CreateOptions, EntryDraft, Field, KdfInfo};

const PASSWORD: &str = "unit-test-password";
const FIXTURE_PASSWORD: &str = "keyrust-test-password";

fn secret(s: &str) -> SecretString {
    SecretString::from(s.to_owned())
}

fn small_kdf() -> Argon2idParams {
    Argon2idParams {
        memory_mib: 8,
        iterations: 1,
        parallelism: 1,
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

fn new_vault() -> (Fixture, Vault) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("test.kdbx");
    let vault = Vault::create(
        &path,
        CreateOptions {
            name: "Test".into(),
            password: secret(PASSWORD),
            cipher: Cipher::ChaCha20,
            kdf: small_kdf(),
        },
    )
    .expect("create");
    (Fixture { _dir: dir, path }, vault)
}

fn fixture_copy(name: &str) -> (Fixture, Vault) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::copy(source, &path).expect("copy fixture");
    let vault = Vault::open(&path, secret(FIXTURE_PASSWORD)).expect("open fixture");
    (Fixture { _dir: dir, path }, vault)
}

fn draft(vault: &Vault, title: &str, password: &str) -> EntryDraft {
    EntryDraft {
        id: None,
        group_id: vault.tree().id,
        title: title.into(),
        username: format!("{title}-user"),
        password: secret(password),
        url: format!("https://{title}.example"),
        notes: format!("notes of {title}"),
    }
}

fn reopen(f: &Fixture) -> Vault {
    Vault::open(&f.path, secret(PASSWORD)).expect("reopen")
}

fn history_passwords(vault: &Vault, id: EntryId) -> Vec<String> {
    let e = vault.entry_ref(id).expect("entry");
    e.history
        .as_ref()
        .map(|h| {
            h.get_entries()
                .iter()
                .map(|v| v.get_password().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

// ------------------------------------------------------------------ open / create

#[test]
fn create_writes_a_kdbx4_argon2id_database_that_reopens() {
    let (f, vault) = new_vault();
    assert!(!vault.is_dirty());
    let info = reopen(&f).info();
    assert_eq!(info.name, "Test");
    assert_eq!(info.cipher, Cipher::ChaCha20);
    assert_eq!(info.version.0, 4);
    assert!(info.recycle_bin);
    assert_eq!(
        info.kdf,
        KdfInfo::Argon2 {
            variant: crate::model::Argon2Variant::Argon2id,
            memory_bytes: 8 * 1024 * 1024,
            iterations: 1,
            parallelism: 1,
        }
    );
}

#[test]
fn create_refuses_an_existing_file_and_empty_inputs() {
    let (f, _vault) = new_vault();
    let again = Vault::create(
        &f.path,
        CreateOptions {
            name: "Again".into(),
            password: secret(PASSWORD),
            cipher: Cipher::Aes256,
            kdf: small_kdf(),
        },
    );
    assert!(matches!(again, Err(VaultError::AlreadyExists)));
    let empty = Vault::create(
        &f.path.with_file_name("other.kdbx"),
        CreateOptions {
            name: "Other".into(),
            password: secret(""),
            cipher: Cipher::Aes256,
            kdf: small_kdf(),
        },
    );
    assert!(matches!(empty, Err(VaultError::Invalid(_))));
}

#[test]
fn typed_open_errors() {
    let (f, _vault) = new_vault();
    assert!(matches!(
        Vault::open(&f.path, secret("wrong")),
        Err(VaultError::WrongKey)
    ));
    assert!(matches!(
        Vault::open_bytes(b"definitely not a database", secret(PASSWORD)),
        Err(VaultError::InvalidFormat)
    ));
    let mut future = std::fs::read(&f.path).expect("read");
    future[8..12].copy_from_slice(&[0, 0, 5, 0]); // minor 0, major 5
    assert!(matches!(
        Vault::open_bytes(&future, secret(PASSWORD)),
        Err(VaultError::UnsupportedVersion { major: 5, minor: 0 })
    ));
    assert!(matches!(
        Vault::open(&f.path.with_file_name("missing.kdbx"), secret(PASSWORD)),
        Err(VaultError::Io(std::io::ErrorKind::NotFound))
    ));
}

#[test]
fn kdbx31_is_read_and_saved_as_kdbx4() {
    let (f, mut vault) = fixture_copy("keepassxc-kdbx31.kdbx");
    assert_eq!(vault.info().version, (3, 1));
    let legacy = vault.search("legacy");
    assert_eq!(legacy.len(), 1);
    vault.save().expect("save");
    let reopened = Vault::open(&f.path, secret(FIXTURE_PASSWORD)).expect("reopen");
    assert_eq!(reopened.info().version.0, 4);
    assert_eq!(
        reopened
            .reveal_field(legacy[0], Field::Password)
            .expect("reveal")
            .expose_secret(),
        "legacy-entry-password"
    );
}

/// The KDBX 3.1 fixture with one byte of its protected stream key (outer
/// header field 8) changed, as a sync provider could without the password.
fn kdbx31_with_changed_stream_key() -> Vec<u8> {
    let mut bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/keepassxc-kdbx31.kdbx"),
    )
    .expect("fixture");
    let mut pos = 12;
    loop {
        let id = bytes[pos];
        let len = usize::from(u16::from_le_bytes([bytes[pos + 1], bytes[pos + 2]]));
        if id == 8 {
            bytes[pos + 3] ^= 0x01;
            return bytes;
        }
        assert_ne!(id, 0, "no protected stream key field");
        pos += 3 + len;
    }
}

#[test]
fn a_kdbx31_header_that_does_not_match_its_header_hash_is_refused() {
    let tampered = kdbx31_with_changed_stream_key();
    // keepass-rs alone accepts it: protected values would decrypt to garbage.
    assert!(Database::parse(&tampered, key(&secret(FIXTURE_PASSWORD))).is_ok());
    assert!(matches!(
        Vault::open_bytes(&tampered, secret(FIXTURE_PASSWORD)),
        Err(VaultError::InvalidFormat)
    ));

    // Reload too: the file is replaced on disk by the tampered copy.
    let (f, mut vault) = fixture_copy("keepassxc-kdbx31.kdbx");
    std::fs::write(&f.path, &tampered).expect("tamper");
    assert!(matches!(vault.reload(), Err(VaultError::InvalidFormat)));
    assert_eq!(vault.info().version, (3, 1), "the vault is left as it was");
}

#[test]
fn the_reported_version_changes_only_when_the_file_is_replaced() {
    use crate::kdbx::limits::Limits;
    let (f, mut vault) = fixture_copy("keepassxc-kdbx31.kdbx");
    vault.rename_database("Renamed").expect("rename");

    vault.max_file_bytes = 100;
    let before = std::fs::read(&f.path).expect("read");
    assert!(matches!(vault.save(), Err(VaultError::TooExpensive)));
    assert_eq!(vault.info().version, (3, 1), "after a refused save");
    assert_eq!(std::fs::read(&f.path).expect("read"), before);
    vault.max_file_bytes = Limits::DEFAULT.max_file_bytes;

    let external = external_bytes();
    std::fs::write(&f.path, &external).expect("external write");
    assert!(matches!(vault.save(), Err(VaultError::ExternalChange)));
    assert_eq!(vault.info().version, (3, 1), "after an external change");
    assert_eq!(std::fs::read(&f.path).expect("read"), external);

    vault.save_overwrite().expect("keep mine");
    assert_eq!(vault.info().version, (4, 1));
    let reopened = Vault::open(&f.path, secret(FIXTURE_PASSWORD)).expect("reopen");
    assert_eq!(reopened.info().version, (4, 1));
}

#[test]
fn an_aes_kdf_database_keeps_its_kdf_unless_asked() {
    let (f, mut vault) = fixture_copy("keepassxc-rich.kdbx");
    let before = vault.info().kdf;
    assert!(matches!(before, KdfInfo::AesKdf { .. }));
    vault.rename_database("Renamed").expect("rename");
    vault.save().expect("save");
    let reopened = Vault::open(&f.path, secret(FIXTURE_PASSWORD)).expect("reopen");
    assert_eq!(reopened.info().kdf, before);
    assert_eq!(reopened.info().name, "Renamed");
}

// ------------------------------------------------------------------ secrets

#[test]
fn debug_output_reveals_nothing() {
    let (_f, mut vault) = new_vault();
    let d = draft(&vault, "Bank", "s3cret-p4ss");
    let id = vault.save_entry(&d).expect("save");
    let shown = format!(
        "{vault:?} {d:?} {:?}",
        vault.entry_for_edit(id).expect("draft")
    );
    for value in [
        PASSWORD,
        "s3cret-p4ss",
        "Bank-user",
        "Bank.example",
        "notes of Bank",
    ] {
        assert!(!shown.contains(value), "{value} leaked in {shown}");
    }
}

#[test]
fn passwords_are_stored_protected_and_only_revealed_on_request() {
    let (f, mut vault) = new_vault();
    let id = vault
        .save_entry(&draft(&vault, "Bank", "pw-1"))
        .expect("save");
    vault.save().expect("save file");
    let vault = reopen(&f);
    let e = vault.entry_ref(id).expect("entry");
    assert!(e.fields[fields::PASSWORD].is_protected());
    let detail = vault.get_entry(id).expect("detail");
    assert!(detail.has_password);
    assert_eq!(
        vault
            .reveal_field(id, Field::Password)
            .expect("reveal")
            .expose_secret(),
        "pw-1"
    );
}

// ------------------------------------------------------------------ entries

#[test]
fn history_is_oldest_first_and_unchanged_saves_add_nothing() {
    let (f, mut vault) = new_vault();
    let id = vault
        .save_entry(&draft(&vault, "Bank", "pw-1"))
        .expect("create");
    for pw in ["pw-2", "pw-3"] {
        let mut d = vault.entry_for_edit(id).expect("draft");
        d.password = secret(pw);
        vault.save_entry(&d).expect("update");
    }
    let unchanged = vault.entry_for_edit(id).expect("draft");
    vault.save_entry(&unchanged).expect("no-op");
    assert_eq!(history_passwords(&vault, id), ["pw-1", "pw-2"]);
    vault.save().expect("save");
    assert_eq!(history_passwords(&reopen(&f), id), ["pw-1", "pw-2"]);
}

#[test]
fn history_is_trimmed_to_the_default_limit() {
    let (_f, mut vault) = new_vault();
    let id = vault
        .save_entry(&draft(&vault, "Bank", "pw-0"))
        .expect("create");
    for n in 1..=12 {
        let mut d = vault.entry_for_edit(id).expect("draft");
        d.password = secret(&format!("pw-{n}"));
        vault.save_entry(&d).expect("update");
    }
    let history = history_passwords(&vault, id);
    assert_eq!(history.len(), DEFAULT_HISTORY_MAX_ITEMS);
    assert_eq!(history.first().map(String::as_str), Some("pw-2"));
    assert_eq!(history.last().map(String::as_str), Some("pw-11"));
}

#[test]
fn updating_a_fixture_entry_keeps_everything_else() {
    let (f, mut vault) = fixture_copy("keepassxc-rich.kdbx");
    let github = vault.search("github")[0];
    let names = |e: &EntryRef<'_>| {
        let mut n: Vec<String> = e.attachments_named().map(|(n, _)| n.to_owned()).collect();
        n.sort();
        n
    };
    let (before, before_attachments) = {
        let e = vault.entry_ref(github).expect("entry");
        ((*e).clone(), names(&e))
    };
    let mut d = vault.entry_for_edit(github).expect("draft");
    d.username = "someone-else".into();
    vault.save_entry(&d).expect("update");
    vault.save().expect("save");

    let vault = Vault::open(&f.path, secret(FIXTURE_PASSWORD)).expect("reopen");
    let after = vault.entry_ref(github).expect("entry");
    assert_eq!(after.get_username(), Some("someone-else"));
    assert_eq!(after.get("Recovery code"), before.get("Recovery code"));
    assert!(after.fields["Recovery code"].is_protected());
    assert_eq!(after.get("Environment"), before.get("Environment"));
    assert_eq!(after.tags, before.tags);
    assert_eq!(after.custom_data, before.custom_data);
    assert_eq!(after.autotype, before.autotype);
    assert_eq!(names(&after), before_attachments);
    assert_eq!(after.times.creation, before.times.creation);
    // KeePassXC's two versions stay first; the version just replaced is appended.
    let previous = before.get_password().unwrap_or_default().to_owned();
    assert_eq!(
        history_passwords(&vault, github),
        ["gh-old-1".to_owned(), "gh-old-2".to_owned(), previous]
    );
}

#[test]
fn delete_restore_and_permanent_delete_with_tombstones() {
    let (_f, mut vault) = new_vault();
    let root = vault.tree().id;
    let work = vault.create_group(root, "Work").expect("group");
    let mut d = draft(&vault, "Jira", "pw");
    d.group_id = work;
    let id = vault.save_entry(&d).expect("create");

    vault.delete_entry(id).expect("to bin");
    let bin = vault
        .tree()
        .children
        .iter()
        .find(|g| g.is_recycle_bin)
        .expect("bin")
        .id;
    assert_eq!(vault.get_entry(id).expect("entry").summary.group_id, bin);
    vault.restore_entry(id).expect("restore");
    assert_eq!(vault.get_entry(id).expect("entry").summary.group_id, work);

    vault.delete_entry(id).expect("to bin");
    vault.delete_entry(id).expect("permanently");
    assert!(matches!(vault.get_entry(id), Err(VaultError::NotFound)));
    assert!(vault.db.deleted_objects.contains_key(&id.0));
}

#[test]
fn deleting_a_group_from_the_bin_leaves_tombstones_for_everything_in_it() {
    let (_f, mut vault) = new_vault();
    let root = vault.tree().id;
    let parent = vault.create_group(root, "Parent").expect("group");
    let child = vault.create_group(parent, "Child").expect("group");
    let mut d = draft(&vault, "Deep", "pw");
    d.group_id = child;
    let entry = vault.save_entry(&d).expect("create");

    vault.delete_group(parent).expect("to bin");
    assert!(vault.db.deleted_objects.is_empty());
    vault.delete_group(parent).expect("permanently");
    for uuid in [parent.0, child.0, entry.0] {
        assert!(vault.db.deleted_objects.contains_key(&uuid), "{uuid}");
    }
}

#[test]
fn without_the_recycle_bin_deletion_is_permanent() {
    let (_f, mut vault) = new_vault();
    vault.set_recycle_bin_enabled(false);
    let id = vault
        .save_entry(&draft(&vault, "Gone", "pw"))
        .expect("create");
    vault.delete_entry(id).expect("delete");
    assert!(matches!(vault.get_entry(id), Err(VaultError::NotFound)));
    assert!(vault.db.deleted_objects.contains_key(&id.0));
}

#[test]
fn group_moves_reject_cycles_the_root_and_the_bin() {
    let (_f, mut vault) = new_vault();
    let root = vault.tree().id;
    let a = vault.create_group(root, "A").expect("group");
    let b = vault.create_group(a, "B").expect("group");
    assert!(vault.move_group(a, b).is_err());
    assert!(vault.move_group(root, a).is_err());
    let doomed = vault.create_group(root, "Doomed").expect("group");
    vault.delete_group(doomed).expect("to bin");
    let bin = vault
        .tree()
        .children
        .iter()
        .find(|g| g.is_recycle_bin)
        .expect("bin")
        .id;
    assert!(vault.move_group(a, bin).is_err());
    vault.move_group(b, root).expect("move");
    vault.rename_group(b, "Renamed").expect("rename");
    assert!(vault.tree().children.iter().any(|g| g.name == "Renamed"));
}

// ------------------------------------------------------------------ search

#[test]
fn search_matches_all_words_but_never_passwords_or_the_bin() {
    let (_f, mut vault) = new_vault();
    let a = vault
        .save_entry(&draft(&vault, "Gitea", "hunter2"))
        .expect("a");
    let b = vault.save_entry(&draft(&vault, "Mail", "pw")).expect("b");
    assert_eq!(vault.search("gitea notes"), [a]);
    assert_eq!(vault.search("MAIL.example"), [b]);
    assert!(vault.search("hunter2").is_empty());
    vault.delete_entry(a).expect("to bin");
    assert!(vault.search("gitea").is_empty());
}

#[test]
fn search_over_5000_entries_is_fast() {
    let (_f, mut vault) = new_vault();
    for n in 0..5000 {
        vault
            .save_entry(&draft(&vault, &format!("entry {n}"), "pw"))
            .expect("create");
    }
    vault.search("warm the index");
    let start = Instant::now();
    let hits = vault.search("entry 4999");
    let elapsed = start.elapsed();
    assert_eq!(hits.len(), 1);
    assert!(elapsed.as_millis() < 50, "{elapsed:?}");
}

// ------------------------------------------------------------------ external changes

fn external_bytes() -> Vec<u8> {
    let (f, mut other) = new_vault();
    other.rename_database("External").expect("rename");
    other.save().expect("save");
    std::fs::read(&f.path).expect("read")
}

#[test]
fn a_change_by_another_program_blocks_save_until_reload_or_overwrite() {
    let (f, mut vault) = new_vault();
    vault.rename_database("Mine").expect("rename");
    std::fs::write(&f.path, external_bytes()).expect("external write");
    assert!(!vault.file_unchanged().expect("check"));
    assert!(matches!(vault.save(), Err(VaultError::ExternalChange)));
    vault.save_overwrite().expect("keep mine");
    assert_eq!(reopen(&f).info().name, "Mine");

    std::fs::write(&f.path, external_bytes()).expect("external write");
    vault.reload().expect("reload");
    assert_eq!(vault.info().name, "External");
    assert!(!vault.is_dirty());
    vault.save().expect("save after reload");
}

#[test]
fn a_write_during_open_is_detected_at_the_next_save() {
    let (f, _vault) = new_vault();
    let external = external_bytes();
    let path = f.path.clone();
    hook::set(move |phase| {
        if phase == Phase::OpenAfterRead {
            std::fs::write(&path, &external).expect("external write");
        }
    });
    let mut vault = reopen(&f);
    hook::clear();
    // The model is the one that was read, and the next save notices.
    assert_eq!(vault.info().name, "Test");
    vault.rename_database("Mine").expect("rename");
    assert!(matches!(vault.save(), Err(VaultError::ExternalChange)));
}

#[test]
fn writes_during_save_are_never_overwritten() {
    for phase in [Phase::SaveAfterFirstCheck, Phase::SaveAfterFsync] {
        let (f, mut vault) = new_vault();
        vault.rename_database("Mine").expect("rename");
        let external = external_bytes();
        let path = f.path.clone();
        let written = external.clone();
        hook::set(move |p| {
            if p == phase {
                std::fs::write(&path, &written).expect("external write");
            }
        });
        let result = vault.save();
        hook::clear();
        assert!(
            matches!(result, Err(VaultError::ExternalChange)),
            "{phase:?}"
        );
        assert_eq!(std::fs::read(&f.path).expect("read"), external, "{phase:?}");
        let leftovers = std::fs::read_dir(f.path.parent().expect("dir"))
            .expect("dir")
            .filter(|e| {
                e.as_ref()
                    .is_ok_and(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            })
            .count();
        assert_eq!(leftovers, 0, "{phase:?}");
        assert!(vault.is_dirty());
    }
}

// ------------------------------------------------------------------ credentials

#[test]
fn reload_uses_the_committed_password_until_a_change_is_saved() {
    let (f, mut vault) = new_vault();
    vault
        .change_master_password(secret("new-password"))
        .expect("change");
    // Not saved yet: the file still opens with the old password.
    vault.reload().expect("reload with the committed password");
    vault
        .change_master_password(secret("new-password"))
        .expect("change");
    vault.save().expect("save");
    assert!(matches!(
        Vault::open(&f.path, secret(PASSWORD)),
        Err(VaultError::WrongKey)
    ));
    Vault::open(&f.path, secret("new-password")).expect("new password");
    vault
        .reload()
        .expect("reload with the new committed password");
}

#[test]
fn a_failed_reload_leaves_the_vault_untouched() {
    let (f, mut vault) = new_vault();
    vault.rename_database("Unsaved").expect("rename");
    std::fs::write(&f.path, b"garbage").expect("corrupt");
    assert!(vault.reload().is_err());
    assert_eq!(vault.info().name, "Unsaved");
    assert!(vault.is_dirty());
}

#[test]
fn kdf_and_cipher_changes_apply_on_save() {
    let (f, mut vault) = new_vault();
    vault
        .set_kdf(Argon2idParams {
            memory_mib: 16,
            iterations: 2,
            parallelism: 2,
        })
        .expect("kdf");
    vault.set_cipher(Cipher::Aes256).expect("cipher");
    vault.save().expect("save");
    let info = reopen(&f).info();
    assert_eq!(info.cipher, Cipher::Aes256);
    assert!(matches!(
        info.kdf,
        KdfInfo::Argon2 {
            memory_bytes: 16_777_216,
            iterations: 2,
            parallelism: 2,
            ..
        }
    ));
    assert!(
        vault
            .set_kdf(Argon2idParams {
                memory_mib: 1,
                iterations: 1,
                parallelism: 1
            })
            .is_err()
    );
}

// ------------------------------------------------------------------ inspection regressions

#[test]
fn moves_recycling_and_restoring_add_no_history() {
    let (_f, mut vault) = new_vault();
    let root = vault.tree().id;
    let work = vault.create_group(root, "Work").expect("group");
    let id = vault
        .save_entry(&draft(&vault, "Bank", "pw-1"))
        .expect("create");
    let mut d = vault.entry_for_edit(id).expect("draft");
    d.password = secret("pw-2");
    vault.save_entry(&d).expect("update");
    assert_eq!(history_passwords(&vault, id), ["pw-1"]);

    vault.move_entry(id, work).expect("move");
    vault.delete_entry(id).expect("to bin");
    vault.restore_entry(id).expect("restore");
    assert_eq!(history_passwords(&vault, id), ["pw-1"]);

    // Editing and moving in one save adds exactly one version: the old one.
    let mut d = vault.entry_for_edit(id).expect("draft");
    d.password = secret("pw-3");
    d.group_id = root;
    vault.save_entry(&d).expect("edit and move");
    assert_eq!(history_passwords(&vault, id), ["pw-1", "pw-2"]);
    assert_eq!(vault.get_entry(id).expect("entry").summary.group_id, root);
}

#[test]
fn an_unprotected_password_becomes_protected_on_save() {
    let (_f, mut vault) = new_vault();
    let id = vault
        .save_entry(&draft(&vault, "Imported", "plain"))
        .expect("create");
    let kid = vault.entry_ref(id).expect("entry").id();
    vault
        .db
        .entry_mut(kid)
        .expect("entry")
        .set_unprotected(fields::PASSWORD, "plain");
    let unchanged = vault.entry_for_edit(id).expect("draft");
    vault.save_entry(&unchanged).expect("save");
    let e = vault.entry_ref(id).expect("entry");
    assert!(e.fields[fields::PASSWORD].is_protected());
    assert_eq!(e.get_password(), Some("plain"));
    assert!(
        history_passwords(&vault, id).is_empty(),
        "no content change"
    );
    assert!(vault.is_dirty());
}

#[test]
fn history_limits_count_attachments_and_may_drop_everything() {
    let (_f, mut vault) = new_vault();
    let id = vault
        .save_entry(&draft(&vault, "Big", "pw-0"))
        .expect("create");
    let kid = vault.entry_ref(id).expect("entry").id();
    vault
        .db
        .entry_mut(kid)
        .expect("entry")
        .add_attachment("blob.bin", Value::unprotected(vec![7u8; 2 * 1024 * 1024]));

    vault.db.meta.history_max_size = Some(3 * 1024 * 1024);
    for n in 1..=3 {
        let mut d = vault.entry_for_edit(id).expect("draft");
        d.password = secret(&format!("pw-{n}"));
        vault.save_entry(&d).expect("update");
    }
    // Each version carries the 2 MiB attachment: only one fits in 3 MiB.
    assert_eq!(history_passwords(&vault, id), ["pw-2"]);

    vault.db.meta.history_max_size = Some(0);
    let mut d = vault.entry_for_edit(id).expect("draft");
    d.password = secret("pw-4");
    vault.save_entry(&d).expect("update");
    assert!(history_passwords(&vault, id).is_empty());

    vault.db.meta.history_max_size = None;
    vault.db.meta.history_max_items = Some(0);
    let mut d = vault.entry_for_edit(id).expect("draft");
    d.password = secret("pw-5");
    vault.save_entry(&d).expect("update");
    assert!(history_passwords(&vault, id).is_empty());
}

#[cfg(unix)]
#[test]
fn saving_keeps_restrictive_permissions_and_new_files_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let mode = |p: &Path| std::fs::metadata(p).expect("metadata").permissions().mode() & 0o777;

    let (f, mut vault) = new_vault();
    assert_eq!(mode(&f.path) & 0o077, 0, "a new vault is owner-only");
    std::fs::set_permissions(&f.path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    vault.rename_database("Again").expect("rename");
    vault.save().expect("save");
    assert_eq!(mode(&f.path), 0o600);

    std::fs::set_permissions(&f.path, std::fs::Permissions::from_mode(0o640)).expect("chmod");
    let mut vault = reopen(&f);
    vault
        .rename_database("Shared with the group")
        .expect("rename");
    vault.save().expect("save");
    assert_eq!(mode(&f.path), 0o640, "an existing mode is preserved");
}

#[test]
fn the_save_lock_lives_next_to_the_database() {
    let (f, _vault) = new_vault();
    let lock = f.path.with_file_name(".test.kdbx.keyrust-lock");
    assert!(lock.exists());
}

// ------------------------------------------------------------------ second inspection regressions

#[test]
fn views_debug_output_reveals_no_content() {
    let (_f, mut vault) = new_vault();
    let root = vault.tree().id;
    vault
        .create_group(root, "Secret Group 51f0")
        .expect("group");
    let id = vault
        .save_entry(&draft(&vault, "Bank", "s3cret-p4ss"))
        .expect("save");
    let shown = format!(
        "{:?} {:?} {:?} {:?}",
        vault.list_entries(),
        vault.get_entry(id).expect("detail"),
        vault.tree(),
        vault.info()
    );
    for value in [
        "Bank",
        "Bank-user",
        "Bank.example",
        "notes of Bank",
        "Secret Group",
        "Test",
    ] {
        assert!(!shown.contains(value), "{value} leaked in {shown}");
    }
}

#[test]
fn kdf_parameters_are_validated_as_a_whole() {
    let (_f, mut vault) = new_vault();
    let before = vault.info().kdf;
    for bad in [
        Argon2idParams {
            memory_mib: 8,
            iterations: 1,
            parallelism: 2048,
        },
        Argon2idParams {
            memory_mib: 4_194_312,
            iterations: 1,
            parallelism: 1,
        },
    ] {
        assert!(vault.set_kdf(bad).is_err(), "{bad:?}");
    }
    assert_eq!(vault.info().kdf, before);
    assert!(!vault.is_dirty());
}

#[cfg(unix)]
#[test]
fn saving_through_a_symbolic_link_updates_the_target() {
    let (f, _vault) = new_vault();
    let link = f.path.with_file_name("link.kdbx");
    std::os::unix::fs::symlink(&f.path, &link).expect("symlink");
    let mut vault = Vault::open(&link, secret(PASSWORD)).expect("open through the link");
    vault.rename_database("Through the link").expect("rename");
    vault.save().expect("save");
    assert!(
        std::fs::symlink_metadata(&link)
            .expect("meta")
            .file_type()
            .is_symlink()
    );
    assert_eq!(reopen(&f).info().name, "Through the link");
}

// ------------------------------------------------------------------ milestone 3c: resource limits

#[test]
fn files_beyond_the_limits_are_refused_before_the_kdf() {
    use crate::kdbx::limits::Limits;
    let (f, _vault) = new_vault();
    let bytes = std::fs::read(&f.path).expect("read");
    for strict in [
        Limits {
            max_argon2_memory_bytes: 1024 * 1024,
            ..Limits::DEFAULT
        },
        Limits {
            max_argon2_iterations: 0,
            ..Limits::DEFAULT
        },
        Limits {
            max_argon2_parallelism: 0,
            ..Limits::DEFAULT
        },
        Limits {
            max_file_bytes: 100,
            ..Limits::DEFAULT
        },
    ] {
        let start = Instant::now();
        let result = Vault::open_bytes_with_limits(&bytes, secret(PASSWORD), &strict);
        assert!(
            matches!(result, Err(VaultError::TooExpensive)),
            "{strict:?}"
        );
        assert!(start.elapsed().as_millis() < 50, "refused before the KDF");
    }
    Vault::open_bytes_with_limits(&bytes, secret(PASSWORD), &Limits::DEFAULT).expect("default");
}

#[test]
fn kdbx3_headers_keepass_rs_would_misread_are_refused() {
    use crate::kdbx::limits::Limits;
    let original = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/keepassxc-kdbx31.kdbx"),
    )
    .expect("fixture");
    // A field inserted right after the version, before KeePassXC's own ones.
    let with = |field: &[u8]| {
        let mut bytes = original[..12].to_vec();
        bytes.extend_from_slice(field);
        bytes.extend_from_slice(&original[12..]);
        bytes
    };
    // Empty or short fixed-width fields: keepass-rs would panic reading them.
    for field in [&[6u8, 0, 0][..], &[3, 2, 0, 1, 0], &[10, 0, 0]] {
        let result =
            Vault::open_bytes_with_limits(&with(field), secret(FIXTURE_PASSWORD), &Limits::DEFAULT);
        assert!(
            matches!(result, Err(VaultError::InvalidFormat)),
            "{field:?}"
        );
    }
    // A cheap first TransformRounds does not hide an expensive later one:
    // keepass-rs uses the last, and so does the check.
    let mut cheap = vec![6u8, 8, 0];
    cheap.extend_from_slice(&1u64.to_le_bytes());
    let strict = Limits {
        max_aes_rounds: 1,
        ..Limits::DEFAULT
    };
    let start = Instant::now();
    let result = Vault::open_bytes_with_limits(&with(&cheap), secret(FIXTURE_PASSWORD), &strict);
    assert!(matches!(result, Err(VaultError::TooExpensive)));
    assert!(start.elapsed().as_millis() < 50, "refused before the KDF");
}

#[test]
fn a_save_beyond_the_open_size_limit_is_refused_and_keeps_the_file() {
    use crate::kdbx::limits::Limits;
    let (f, mut vault) = new_vault();
    let before = std::fs::read(&f.path).expect("read");
    let big = EntryDraft {
        notes: "x".repeat(4096),
        ..draft(&vault, "Big", "pw")
    };
    vault.save_entry(&big).expect("entry");
    vault.max_file_bytes = before.len() as u64 + 100;
    assert!(matches!(vault.save(), Err(VaultError::TooExpensive)));
    assert!(vault.is_dirty(), "unsaved changes are kept");
    assert_eq!(
        std::fs::read(&f.path).expect("read"),
        before,
        "file untouched"
    );
    vault.max_file_bytes = Limits::DEFAULT.max_file_bytes;
    vault.save().expect("save within the limit");
}

#[test]
fn reading_is_bounded_even_if_the_file_grows() {
    let (f, _vault) = new_vault();
    let path = f.path.clone();
    let len = std::fs::metadata(&path).expect("meta").len();
    assert!(file::read_limited(&path, len).expect("read").is_some());
    assert!(file::read_limited(&path, len - 1).expect("read").is_none());

    let grown = path.clone();
    hook::set(move |phase| {
        if phase == Phase::OpenBeforeRead {
            let mut bigger = std::fs::read(&grown).expect("read");
            bigger.extend(std::iter::repeat_n(0u8, 4096));
            std::fs::write(&grown, bigger).expect("grow");
        }
    });
    let result = file::read_limited(&path, len + 100);
    hook::clear();
    assert!(
        result.expect("read").is_none(),
        "growth past the limit is refused"
    );
}

#[test]
fn change_checks_do_not_read_a_much_larger_file() {
    let (f, vault) = new_vault();
    let mut bigger = std::fs::read(&f.path).expect("read");
    bigger.extend(std::iter::repeat_n(7u8, 1024 * 1024));
    std::fs::write(&f.path, bigger).expect("write");
    assert!(!vault.file_unchanged().expect("check"));
}
