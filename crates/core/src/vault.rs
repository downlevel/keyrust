//! An open KeePass database.
//!
//! [`Vault`] wraps keepass-rs's `Database` and works on its model directly, so
//! everything Keyrust does not show (custom fields, attachments, custom data,
//! auto-type, icons…) is kept as it is. Passwords leave the vault only through
//! [`Vault::reveal_field`] and [`Vault::entry_for_edit`].

use std::cell::OnceCell;
use std::fmt;
use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;
use keepass::config::{
    DatabaseConfig, DatabaseVersion, InnerCipherConfig, KdfConfig, OuterCipherConfig,
};
use keepass::db::{DatabaseOpenError, Entry, EntryRef, GroupRef, History, Times, Value, fields};
use keepass::error::DatabaseKeyError;
use keepass::{Database, DatabaseKey};
use secrecy::{ExposeSecret, SecretString};

use crate::error::{Result, VaultError};
use crate::file::{self, Phase, ReplaceError, Stamp};
use crate::kdbx::header::{self, HeaderError};
use crate::kdbx::limits::{self, Limits};
use crate::model::{
    Argon2Variant, Argon2idParams, Cipher, CreateOptions, DatabaseInfo, EntryDetail, EntryDraft,
    EntryId, EntrySummary, Field, GroupId, GroupNode, KdfInfo,
};

/// Built-in KeePass icon of the recycle bin.
const RECYCLE_BIN_ICON: usize = 43;
/// KeePass defaults when the database does not set its own limits.
const DEFAULT_HISTORY_MAX_ITEMS: usize = 10;
const DEFAULT_HISTORY_MAX_SIZE: usize = 6 * 1024 * 1024;

pub struct Vault {
    db: Database,
    path: Option<PathBuf>,
    /// The key the file on disk is encrypted with.
    committed: SecretString,
    /// A new master password, used from the next successful save.
    pending: Option<SecretString>,
    /// Content of the file this model was read from or last written to.
    stamp: Option<Stamp>,
    dirty: bool,
    /// Lower-cased searchable text per entry, rebuilt after any change.
    index: OnceCell<Vec<(EntryId, String)>>,
}

impl fmt::Debug for Vault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Neither the key, nor the path, nor any content.
        f.debug_struct("Vault")
            .field("entries", &self.db.num_entries())
            .field("groups", &self.db.num_groups())
            .field("dirty", &self.dirty)
            .finish_non_exhaustive()
    }
}

fn key(secret: &SecretString) -> DatabaseKey {
    DatabaseKey::new().with_password(secret.expose_secret())
}

fn now() -> NaiveDateTime {
    Times::now()
}

fn iso(time: Option<NaiveDateTime>) -> Option<String> {
    time.map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// Rejects files whose version Keyrust cannot read, from the header alone.
fn check_version(bytes: &[u8]) -> Result<()> {
    match header::peek(bytes) {
        Ok(_) => Ok(()),
        // KDBX 3.1 is read by keepass-rs (and written back as KDBX 4), once
        // its header is known to be one keepass-rs reads without panicking.
        Err(HeaderError::UnsupportedVersion { major: 3, .. }) => {
            header::kdbx3_transform_rounds(bytes).map_err(|_| VaultError::InvalidFormat)?;
            Ok(())
        }
        Err(HeaderError::UnsupportedVersion { major, minor }) => {
            Err(VaultError::UnsupportedVersion { major, minor })
        }
        Err(_) => Err(VaultError::InvalidFormat),
    }
}

fn check_limits(bytes: &[u8], limits: &Limits) -> Result<()> {
    limits::check(bytes, limits).map_err(|exceeded| {
        tracing::warn!(?exceeded, "database refused: exceeds resource limits");
        VaultError::TooExpensive
    })
}

fn map_open_error(e: DatabaseOpenError) -> VaultError {
    match e {
        DatabaseOpenError::Key(DatabaseKeyError::IncorrectKey | DatabaseKeyError::EmptyKey) => {
            VaultError::WrongKey
        }
        DatabaseOpenError::Io(e) => VaultError::Io(e.kind()),
        _ => VaultError::InvalidFormat,
    }
}

fn kdf_info(config: &KdfConfig) -> KdfInfo {
    match config {
        KdfConfig::Aes { rounds } => KdfInfo::AesKdf { rounds: *rounds },
        KdfConfig::Argon2 {
            iterations,
            memory,
            parallelism,
            ..
        } => KdfInfo::Argon2 {
            variant: Argon2Variant::Argon2d,
            memory_bytes: *memory,
            iterations: *iterations,
            parallelism: *parallelism,
        },
        KdfConfig::Argon2id {
            iterations,
            memory,
            parallelism,
            ..
        } => KdfInfo::Argon2 {
            variant: Argon2Variant::Argon2id,
            memory_bytes: *memory,
            iterations: *iterations,
            parallelism: *parallelism,
        },
        _ => KdfInfo::Other,
    }
}

fn argon2id(params: Argon2idParams) -> Result<KdfConfig> {
    crate::kdf::validate(&params).map_err(|_| VaultError::Invalid("key derivation parameters"))?;
    Ok(KdfConfig::Argon2id {
        iterations: u64::from(params.iterations),
        memory: u64::from(params.memory_mib) * 1024 * 1024,
        parallelism: params.parallelism,
        version: argon2::Version::Version13,
    })
}

fn outer_cipher(cipher: Cipher) -> Result<OuterCipherConfig> {
    match cipher {
        Cipher::Aes256 => Ok(OuterCipherConfig::AES256),
        Cipher::ChaCha20 => Ok(OuterCipherConfig::ChaCha20),
        Cipher::Twofish => Ok(OuterCipherConfig::Twofish),
        Cipher::Other => Err(VaultError::Invalid("cipher")),
    }
}

/// Size of an entry version for the history limit: its fields and the data
/// of its attachments (KeePass counts binaries too).
fn version_size(entry: &EntryRef<'_>) -> usize {
    let fields: usize = entry
        .fields
        .iter()
        .map(|(k, v)| k.len() + v.get().len())
        .sum();
    let attachments: usize = entry.attachments().map(|a| a.data.get().len()).sum();
    fields + attachments
}

/// Moves an entry without keepass-rs's tracked move, whose drop handler would
/// prepend a history snapshot behind Keyrust's back.
fn move_untracked(
    entry: &mut keepass::db::EntryMut<'_>,
    target: keepass::db::GroupId,
) -> Result<()> {
    entry
        .move_to(target)
        .map_err(|_| VaultError::Invalid("group"))?;
    entry.times.location_changed = Some(now());
    Ok(())
}

impl Vault {
    // ------------------------------------------------------------ open / create

    /// Opens a database file. The file is read once; the model and the stamp
    /// used to detect external changes both come from those bytes.
    pub fn open(path: &Path, password: SecretString) -> Result<Self> {
        // Saving replaces a file by rename: through a symbolic link that would
        // replace the link, so the vault keeps the link's target instead.
        let path = &std::fs::canonicalize(path)?;
        let bytes = file::read_limited(path, Limits::DEFAULT.max_file_bytes)?
            .ok_or(VaultError::TooExpensive)?;
        file::fire(Phase::OpenAfterRead);
        let mut vault = Self::open_bytes(&bytes, password)?;
        vault.path = Some(path.to_path_buf());
        vault.stamp = Some(Stamp::of(&bytes));
        tracing::info!(entries = vault.db.num_entries(), "vault opened");
        tracing::debug!(path = %path.display(), "vault file");
        Ok(vault)
    }

    /// Opens a database from memory (no file to save to).
    pub fn open_bytes(bytes: &[u8], password: SecretString) -> Result<Self> {
        Self::open_bytes_with_limits(bytes, password, &Limits::DEFAULT)
    }

    /// Like `open_bytes`, with explicit resource limits (fuzzing, tests).
    /// The limits are checked on the header before any key derivation.
    pub fn open_bytes_with_limits(
        bytes: &[u8],
        password: SecretString,
        limits: &Limits,
    ) -> Result<Self> {
        check_limits(bytes, limits)?;
        check_version(bytes)?;
        let db = Database::parse(bytes, key(&password)).map_err(map_open_error)?;
        Ok(Self {
            db,
            path: None,
            committed: password,
            pending: None,
            stamp: None,
            dirty: false,
            index: OnceCell::new(),
        })
    }

    /// Creates a new KDBX 4 database at `path`, which must not exist yet.
    pub fn create(path: &Path, options: CreateOptions) -> Result<Self> {
        if options.name.trim().is_empty() {
            return Err(VaultError::Invalid("database name"));
        }
        if options.password.expose_secret().is_empty() {
            return Err(VaultError::Invalid("master password"));
        }
        // Resolve the folder (it may be a link); the file itself is new.
        let path = &match (path.parent(), path.file_name()) {
            (Some(dir), Some(name)) if !dir.as_os_str().is_empty() => {
                std::fs::canonicalize(dir)?.join(name)
            }
            _ => std::path::absolute(path)?,
        };
        let mut config = DatabaseConfig::default();
        config.outer_cipher_config = outer_cipher(options.cipher)?;
        config.inner_cipher_config = InnerCipherConfig::ChaCha20;
        config.kdf_config = argon2id(options.kdf)?;
        let mut db = Database::with_config(config);
        let name = options.name.trim().to_owned();
        db.meta.database_name = Some(name.clone());
        db.meta.database_name_changed = Some(now());
        db.meta.recyclebin_enabled = Some(true);
        db.root_mut().name = name;

        let mut vault = Self {
            db,
            path: Some(path.to_path_buf()),
            committed: options.password,
            pending: None,
            stamp: None,
            dirty: true,
            index: OnceCell::new(),
        };
        vault.write(None, false)?;
        tracing::info!("vault created");
        Ok(vault)
    }

    // ------------------------------------------------------------ saving

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Saves unless the file changed on disk since it was opened or last saved
    /// (then [`VaultError::ExternalChange`], and nothing is written).
    pub fn save(&mut self) -> Result<()> {
        let stamp = self.stamp.ok_or(VaultError::NoFile)?;
        self.write(Some(stamp), false)
    }

    /// Saves over whatever is on disk ("Keep mine").
    pub fn save_overwrite(&mut self) -> Result<()> {
        self.write(None, true)
    }

    fn write(&mut self, expected: Option<Stamp>, overwrite: bool) -> Result<()> {
        let path = self.path.clone().ok_or(VaultError::NoFile)?;
        // keepass-rs writes KDBX 4.1 only: KDBX 3.1 and 4.0 files (KeePassXC
        // writes 4.0 when no 4.1 feature is used) are upgraded on save.
        let latest = DatabaseConfig::default().version;
        if self.db.config.version != latest {
            self.db.config.version = latest;
        }
        let secret = self.pending.as_ref().unwrap_or(&self.committed);
        let mut bytes = Vec::new();
        self.db.save(&mut bytes, key(secret)).map_err(|e| {
            tracing::warn!(error = %e, "cannot serialise vault");
            VaultError::Io(std::io::ErrorKind::Other)
        })?;
        let mut durability = Ok(());
        match file::replace(&path, &bytes, expected.as_ref(), overwrite) {
            Ok(()) => {}
            // The file was replaced but the directory sync failed: the state
            // below must follow the file on disk, and the caller is told.
            Err(ReplaceError::NotDurable(e)) => {
                tracing::warn!(error_kind = ?e.kind(), "saved, but not confirmed durable");
                durability = Err(VaultError::NotDurable(e.kind()));
            }
            Err(ReplaceError::Changed) => {
                tracing::warn!("save refused: file changed by another application");
                return Err(VaultError::ExternalChange);
            }
            Err(ReplaceError::Io(e)) => {
                tracing::warn!(error_kind = ?e.kind(), "cannot save vault");
                return Err(e.into());
            }
        }
        self.stamp = Some(Stamp::of(&bytes));
        if let Some(new) = self.pending.take() {
            self.committed = new;
        }
        self.dirty = false;
        tracing::info!(entries = self.db.num_entries(), "vault saved");
        durability
    }

    /// Re-reads the file with the committed key ("Reload"), dropping unsaved
    /// changes. On failure the vault is left exactly as it was.
    pub fn reload(&mut self) -> Result<()> {
        let path = self.path.clone().ok_or(VaultError::NoFile)?;
        let bytes = file::read_limited(&path, Limits::DEFAULT.max_file_bytes)?
            .ok_or(VaultError::TooExpensive)?;
        check_limits(&bytes, &Limits::DEFAULT)?;
        check_version(&bytes)?;
        let db = Database::parse(&bytes, key(&self.committed)).map_err(map_open_error)?;
        self.db = db;
        self.stamp = Some(Stamp::of(&bytes));
        self.pending = None;
        self.dirty = false;
        self.touch_index();
        tracing::info!("vault reloaded");
        Ok(())
    }

    /// Whether the file on disk still matches what this vault last read or
    /// wrote (false also if it disappeared).
    pub fn file_unchanged(&self) -> Result<bool> {
        let (Some(path), Some(stamp)) = (&self.path, &self.stamp) else {
            return Err(VaultError::NoFile);
        };
        Ok(file::unchanged(path, stamp)?)
    }

    fn changed(&mut self) {
        self.dirty = true;
        self.touch_index();
    }

    fn touch_index(&mut self) {
        self.index = OnceCell::new();
    }

    // ------------------------------------------------------------ database

    pub fn info(&self) -> DatabaseInfo {
        let version = match self.db.config.version {
            DatabaseVersion::KDB(v) => (1, v),
            DatabaseVersion::KDB2(v) => (2, v),
            DatabaseVersion::KDB3(v) => (3, v),
            DatabaseVersion::KDB4(v) => (4, v),
        };
        DatabaseInfo {
            name: self
                .db
                .meta
                .database_name
                .clone()
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| self.db.root().name.clone()),
            cipher: match self.db.config.outer_cipher_config {
                OuterCipherConfig::AES256 => Cipher::Aes256,
                OuterCipherConfig::ChaCha20 => Cipher::ChaCha20,
                OuterCipherConfig::Twofish => Cipher::Twofish,
                _ => Cipher::Other,
            },
            kdf: kdf_info(&self.db.config.kdf_config),
            recycle_bin: self.recycle_bin_enabled(),
            version,
        }
    }

    pub fn rename_database(&mut self, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(VaultError::Invalid("database name"));
        }
        self.db.meta.database_name = Some(name.to_owned());
        self.db.meta.database_name_changed = Some(now());
        self.changed();
        Ok(())
    }

    /// Switches the key derivation to Argon2id with these parameters.
    pub fn set_kdf(&mut self, params: Argon2idParams) -> Result<()> {
        self.db.config.kdf_config = argon2id(params)?;
        self.changed();
        Ok(())
    }

    pub fn set_cipher(&mut self, cipher: Cipher) -> Result<()> {
        self.db.config.outer_cipher_config = outer_cipher(cipher)?;
        self.changed();
        Ok(())
    }

    /// Takes effect with the next successful save; until then the file (and
    /// `reload`) still use the current password.
    pub fn change_master_password(&mut self, password: SecretString) -> Result<()> {
        if password.expose_secret().is_empty() {
            return Err(VaultError::Invalid("master password"));
        }
        self.pending = Some(password);
        self.db.meta.master_key_changed = Some(now());
        self.changed();
        Ok(())
    }

    pub fn set_recycle_bin_enabled(&mut self, enabled: bool) {
        self.db.meta.recyclebin_enabled = Some(enabled);
        self.db.meta.recyclebin_changed = Some(now());
        self.changed();
    }

    fn recycle_bin_enabled(&self) -> bool {
        self.db.meta.recyclebin_enabled.unwrap_or(true)
    }

    // ------------------------------------------------------------ groups

    fn recycle_bin_id(&self) -> Option<keepass::db::GroupId> {
        self.db.recycle_bin().map(|g| g.id())
    }

    /// Whether `group` is the recycle bin or inside it.
    fn in_recycle_bin(&self, group: keepass::db::GroupId) -> bool {
        let Some(bin) = self.recycle_bin_id() else {
            return false;
        };
        let mut current = Some(group);
        while let Some(id) = current {
            if id == bin {
                return true;
            }
            current = self.db.group(id).and_then(|g| g.parent().map(|p| p.id()));
        }
        false
    }

    /// The recycle bin, created like KeePassXC does on first use.
    fn ensure_recycle_bin(&mut self) -> keepass::db::GroupId {
        if let Some(id) = self.recycle_bin_id() {
            return id;
        }
        let time = now();
        let mut root = self.db.root_mut();
        let mut bin = root.add_group();
        bin.name = "Recycle Bin".to_owned();
        bin.set_icon_builtin(RECYCLE_BIN_ICON);
        bin.enable_autotype = Some(false);
        bin.enable_searching = Some(false);
        let id = bin.id();
        self.db.meta.recyclebin_uuid = Some(id.uuid());
        self.db.meta.recyclebin_changed = Some(time);
        id
    }

    fn group_ref(&self, id: GroupId) -> Result<GroupRef<'_>> {
        self.db
            .group(keepass::db::GroupId::from_uuid(id.0))
            .ok_or(VaultError::NotFound)
    }

    pub fn tree(&self) -> GroupNode {
        let bin = self.recycle_bin_id();
        fn node(g: GroupRef<'_>, bin: Option<keepass::db::GroupId>) -> GroupNode {
            GroupNode {
                id: GroupId(g.id().uuid()),
                name: g.name.clone(),
                entry_count: g.entries().count(),
                is_recycle_bin: Some(g.id()) == bin,
                children: g.groups().map(|c| node(c, bin)).collect(),
            }
        }
        node(self.db.root(), bin)
    }

    pub fn create_group(&mut self, parent: GroupId, name: &str) -> Result<GroupId> {
        let name = name.trim();
        if name.is_empty() {
            return Err(VaultError::Invalid("group name"));
        }
        let parent = self.group_ref(parent)?.id();
        if self.in_recycle_bin(parent) {
            return Err(VaultError::Invalid("parent group"));
        }
        let mut parent = self.db.group_mut(parent).ok_or(VaultError::NotFound)?;
        let mut group = parent.add_group();
        group.name = name.to_owned();
        let id = GroupId(group.id().uuid());
        self.changed();
        Ok(id)
    }

    pub fn rename_group(&mut self, id: GroupId, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(VaultError::Invalid("group name"));
        }
        let gid = self.group_ref(id)?.id();
        let mut group = self.db.group_mut(gid).ok_or(VaultError::NotFound)?;
        if group.name != name {
            group.name = name.to_owned();
            group.times.last_modification = Some(now());
            self.changed();
        }
        Ok(())
    }

    pub fn move_group(&mut self, id: GroupId, new_parent: GroupId) -> Result<()> {
        let gid = self.group_ref(id)?.id();
        let target = self.group_ref(new_parent)?.id();
        if gid == self.db.root().id() {
            return Err(VaultError::Invalid("group"));
        }
        if self.in_recycle_bin(target) {
            return Err(VaultError::Invalid("destination group"));
        }
        let mut group = self.db.group_mut(gid).ok_or(VaultError::NotFound)?;
        group
            .track_changes()
            .move_to(target)
            .map_err(|_| VaultError::Invalid("destination group"))?;
        self.changed();
        Ok(())
    }

    /// To the recycle bin, or permanently (with tombstones for the group and
    /// everything in it) when it is already there or the bin is disabled.
    pub fn delete_group(&mut self, id: GroupId) -> Result<()> {
        let gid = self.group_ref(id)?.id();
        if gid == self.db.root().id() || Some(gid) == self.recycle_bin_id() {
            return Err(VaultError::Invalid("group"));
        }
        if self.recycle_bin_enabled() && !self.in_recycle_bin(gid) {
            let bin = self.ensure_recycle_bin();
            let mut group = self.db.group_mut(gid).ok_or(VaultError::NotFound)?;
            group
                .track_changes()
                .move_to(bin)
                .map_err(|_| VaultError::Invalid("group"))?;
        } else {
            let mut group = self.db.group_mut(gid).ok_or(VaultError::NotFound)?;
            group
                .track_changes()
                .remove()
                .map_err(|_| VaultError::Invalid("group"))?;
        }
        self.changed();
        Ok(())
    }

    // ------------------------------------------------------------ entries

    fn entry_ref(&self, id: EntryId) -> Result<EntryRef<'_>> {
        self.db
            .entry(keepass::db::EntryId::from_uuid(id.0))
            .ok_or(VaultError::NotFound)
    }

    fn summary(e: &EntryRef<'_>) -> EntrySummary {
        EntrySummary {
            id: EntryId(e.id().uuid()),
            group_id: GroupId(e.parent().id().uuid()),
            title: e.get_title().unwrap_or_default().to_owned(),
            username: e.get_username().unwrap_or_default().to_owned(),
            url: e.get_url().unwrap_or_default().to_owned(),
        }
    }

    /// Every current entry (not history versions), recycle bin included.
    pub fn list_entries(&self) -> Vec<EntrySummary> {
        let mut list: Vec<_> = self
            .db
            .iter_all_entries()
            .map(|e| Self::summary(&e))
            .collect();
        list.sort_by_key(|e| e.title.to_lowercase());
        list
    }

    pub fn get_entry(&self, id: EntryId) -> Result<EntryDetail> {
        let e = self.entry_ref(id)?;
        Ok(EntryDetail {
            summary: Self::summary(&e),
            notes: e.get(fields::NOTES).unwrap_or_default().to_owned(),
            has_password: e.get_password().is_some_and(|p| !p.is_empty()),
            created: iso(e.times.creation),
            modified: iso(e.times.last_modification),
            history_len: e.history.as_ref().map_or(0, |h| h.get_entries().len()),
        })
    }

    pub fn reveal_field(&self, id: EntryId, field: Field) -> Result<SecretString> {
        let e = self.entry_ref(id)?;
        let key = match field {
            Field::Username => fields::USERNAME,
            Field::Password => fields::PASSWORD,
            Field::Url => fields::URL,
            Field::Notes => fields::NOTES,
        };
        Ok(SecretString::from(
            e.get(key).unwrap_or_default().to_owned(),
        ))
    }

    pub fn entry_for_edit(&self, id: EntryId) -> Result<EntryDraft> {
        let e = self.entry_ref(id)?;
        Ok(EntryDraft {
            id: Some(id),
            group_id: GroupId(e.parent().id().uuid()),
            title: e.get_title().unwrap_or_default().to_owned(),
            username: e.get_username().unwrap_or_default().to_owned(),
            password: SecretString::from(e.get_password().unwrap_or_default().to_owned()),
            url: e.get_url().unwrap_or_default().to_owned(),
            notes: e.get(fields::NOTES).unwrap_or_default().to_owned(),
        })
    }

    /// Creates (`draft.id == None`) or updates an entry. Only the standard
    /// fields and the group are touched; an update that changes something
    /// appends the previous version to the end of the history (oldest first,
    /// as KeePass stores it), within the database's history limits.
    pub fn save_entry(&mut self, draft: &EntryDraft) -> Result<EntryId> {
        let title = draft.title.trim();
        if title.is_empty() {
            return Err(VaultError::Invalid("title"));
        }
        let group = self.group_ref(draft.group_id)?.id();
        if self.in_recycle_bin(group) {
            return Err(VaultError::Invalid("group"));
        }
        let values = [
            (fields::TITLE, title, false),
            (fields::USERNAME, draft.username.as_str(), false),
            (fields::PASSWORD, draft.password.expose_secret(), true),
            (fields::URL, draft.url.as_str(), false),
            (fields::NOTES, draft.notes.as_str(), false),
        ];

        let Some(id) = draft.id else {
            let time = now();
            let mut parent = self.db.group_mut(group).ok_or(VaultError::NotFound)?;
            let mut entry = parent.add_entry();
            for (key, value, protected) in values {
                if protected {
                    entry.set_protected(key, value);
                } else {
                    entry.set_unprotected(key, value);
                }
            }
            entry.times.creation = Some(time);
            entry.times.last_modification = Some(time);
            entry.times.last_access = Some(time);
            let id = EntryId(entry.id().uuid());
            self.changed();
            tracing::info!("entry created");
            return Ok(id);
        };

        let (kid, current_group, mut sizes, current_size) = {
            let e = self.entry_ref(id)?;
            let count = e.history.as_ref().map_or(0, |h| h.get_entries().len());
            let sizes: Vec<usize> = (0..count)
                .filter_map(|i| e.historical(i))
                .map(|v| version_size(&v))
                .collect();
            (e.id(), e.parent().id(), sizes, version_size(&e))
        };
        let (max_items, max_size) = self.history_limits();
        let mut entry = self.db.entry_mut(kid).ok_or(VaultError::NotFound)?;
        let mut previous: Entry = (*entry).clone();
        previous.history = None;

        let mut modified = false;
        for (key, value, protected) in values {
            let current = entry.fields.get(key);
            let same = current.is_some_and(|v| v.get() == value)
                || (current.is_none() && value.is_empty());
            if same {
                continue;
            }
            // A field keeps its protection flag; the password is always protected.
            let protect = protected || current.is_some_and(Value::is_protected);
            if protect {
                entry.set_protected(key, value);
            } else {
                entry.set_unprotected(key, value);
            }
            modified = true;
        }
        // The password is always stored protected, even if it arrived
        // unprotected from another client and its text did not change.
        let mut protected_now = false;
        if let Some(value) = entry.fields.get(fields::PASSWORD)
            && !value.is_protected()
        {
            let value = value.get().clone();
            entry.set_protected(fields::PASSWORD, value);
            protected_now = true;
        }
        if modified {
            let time = now();
            entry.times.last_modification = Some(time);
            entry.times.last_access = Some(time);
            let mut versions: Vec<Entry> = entry
                .history
                .as_ref()
                .map(|h| h.get_entries().clone())
                .unwrap_or_default();
            versions.push(previous);
            sizes.push(current_size);
            // Oldest versions go first; the limits apply even if that leaves
            // no history at all (history_max_items/max_size = 0).
            while !versions.is_empty()
                && (versions.len() > max_items || sizes.iter().sum::<usize>() > max_size)
            {
                versions.remove(0);
                sizes.remove(0);
            }
            // `History::add_entry` inserts at the front: add newest first so
            // the final order is oldest first.
            let mut history = History::default();
            for version in versions.into_iter().rev() {
                history.add_entry(version);
            }
            entry.history = Some(history);
        }
        let moved = current_group != group;
        if moved {
            move_untracked(&mut entry, group)?;
        }
        if modified || moved || protected_now {
            self.changed();
            tracing::info!(modified, moved, "entry updated");
        }
        Ok(id)
    }

    fn history_limits(&self) -> (usize, usize) {
        let items = self
            .db
            .meta
            .history_max_items
            .map_or(DEFAULT_HISTORY_MAX_ITEMS, |n| {
                usize::try_from(n).unwrap_or(usize::MAX)
            });
        let size = self
            .db
            .meta
            .history_max_size
            .map_or(DEFAULT_HISTORY_MAX_SIZE, |n| {
                usize::try_from(n).unwrap_or(usize::MAX)
            });
        (items, size)
    }

    pub fn move_entry(&mut self, id: EntryId, group: GroupId) -> Result<()> {
        let (kid, current_group) = {
            let e = self.entry_ref(id)?;
            (e.id(), e.parent().id())
        };
        let target = self.group_ref(group)?.id();
        if self.in_recycle_bin(target) {
            return Err(VaultError::Invalid("group"));
        }
        let mut entry = self.db.entry_mut(kid).ok_or(VaultError::NotFound)?;
        if current_group != target {
            move_untracked(&mut entry, target)?;
            self.changed();
        }
        Ok(())
    }

    /// To the recycle bin, or permanently (with a tombstone) when it is
    /// already there or the bin is disabled.
    pub fn delete_entry(&mut self, id: EntryId) -> Result<()> {
        let e = self.entry_ref(id)?;
        let kid = e.id();
        let parent = e.parent().id();
        if self.recycle_bin_enabled() && !self.in_recycle_bin(parent) {
            let bin = self.ensure_recycle_bin();
            let mut entry = self.db.entry_mut(kid).ok_or(VaultError::NotFound)?;
            move_untracked(&mut entry, bin)?;
            tracing::info!("entry moved to the recycle bin");
        } else {
            let mut entry = self.db.entry_mut(kid).ok_or(VaultError::NotFound)?;
            entry.track_changes().remove();
            tracing::info!("entry deleted");
        }
        self.changed();
        Ok(())
    }

    /// Back to the group it was deleted from, or the root if that is gone.
    pub fn restore_entry(&mut self, id: EntryId) -> Result<()> {
        let e = self.entry_ref(id)?;
        let kid = e.id();
        if !self.in_recycle_bin(e.parent().id()) {
            return Err(VaultError::Invalid("entry"));
        }
        let target = e
            .previous_parent()
            .map(|g| g.id())
            .filter(|g| !self.in_recycle_bin(*g))
            .unwrap_or_else(|| self.db.root().id());
        let mut entry = self.db.entry_mut(kid).ok_or(VaultError::NotFound)?;
        move_untracked(&mut entry, target)?;
        self.changed();
        Ok(())
    }

    // ------------------------------------------------------------ search

    /// Entries whose title, username, URL or notes contain every word of
    /// `query` (case-insensitive). Passwords and protected fields are never
    /// searched; nor are entries in the recycle bin.
    pub fn search(&self, query: &str) -> Vec<EntryId> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let index = self.index.get_or_init(|| self.build_index());
        index
            .iter()
            .filter(|(_, text)| words.iter().all(|w| text.contains(w.as_str())))
            .map(|(id, _)| *id)
            .collect()
    }

    fn build_index(&self) -> Vec<(EntryId, String)> {
        let mut entries: Vec<_> = self
            .db
            .iter_all_entries()
            .filter(|e| !self.in_recycle_bin(e.parent().id()))
            .map(|e| {
                let text = [fields::TITLE, fields::USERNAME, fields::URL, fields::NOTES]
                    .into_iter()
                    .filter(|k| !e.fields.get(*k).is_some_and(Value::is_protected))
                    .filter_map(|k| e.get(k))
                    .collect::<Vec<_>>()
                    .join("\n")
                    .to_lowercase();
                (
                    e.get_title().unwrap_or_default().to_lowercase(),
                    EntryId(e.id().uuid()),
                    text,
                )
            })
            .collect();
        entries.sort();
        entries
            .into_iter()
            .map(|(_, id, text)| (id, text))
            .collect()
    }
}

#[cfg(test)]
mod tests;
