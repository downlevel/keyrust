//! Views of a vault for the UI. Lists and details never carry a password; the
//! only exceptions are [`EntryDraft`] (the editor) and `Vault::reveal_field`.

use std::fmt;

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(EntryId);
id_type!(GroupId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Field {
    Username,
    Password,
    Url,
    Notes,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntrySummary {
    pub id: EntryId,
    pub group_id: GroupId,
    pub title: String,
    pub username: String,
    pub url: String,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryDetail {
    pub summary: EntrySummary,
    pub notes: String,
    pub has_password: bool,
    /// ISO 8601 (UTC), when known.
    pub created: Option<String>,
    pub modified: Option<String>,
    pub history_len: usize,
}

/// Editable copy of an entry, password included: only for the editor.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryDraft {
    /// `None` creates a new entry.
    pub id: Option<EntryId>,
    pub group_id: GroupId,
    pub title: String,
    pub username: String,
    pub password: SecretString,
    pub url: String,
    pub notes: String,
}

impl fmt::Debug for EntryDraft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EntryDraft")
            .field("id", &self.id)
            .field("group_id", &self.group_id)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupNode {
    pub id: GroupId,
    pub name: String,
    pub children: Vec<GroupNode>,
    pub entry_count: usize,
    pub is_recycle_bin: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Cipher {
    Aes256,
    ChaCha20,
    /// Readable and preserved; not offered for new databases.
    Twofish,
    /// A cipher this version of keepass-rs reports but Keyrust does not name.
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Argon2Variant {
    Argon2d,
    Argon2id,
}

/// The key derivation of a database, as stored in its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum KdfInfo {
    #[serde(rename_all = "camelCase")]
    AesKdf { rounds: u64 },
    #[serde(rename_all = "camelCase")]
    Argon2 {
        variant: Argon2Variant,
        memory_bytes: u64,
        iterations: u64,
        parallelism: u32,
    },
    /// A KDF this version of keepass-rs reports but Keyrust does not name.
    Other,
}

/// Argon2id parameters Keyrust creates and edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Argon2idParams {
    pub memory_mib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    pub name: String,
    pub cipher: Cipher,
    pub kdf: KdfInfo,
    pub recycle_bin: bool,
    /// KDBX version on disk, e.g. (4, 1). Saving writes KDBX 4.
    pub version: (u16, u16),
}

/// Options for a new database.
pub struct CreateOptions {
    pub name: String,
    pub password: SecretString,
    pub cipher: Cipher,
    pub kdf: Argon2idParams,
}

impl fmt::Debug for CreateOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateOptions")
            .field("cipher", &self.cipher)
            .field("kdf", &self.kdf)
            .finish_non_exhaustive()
    }
}

// Views carry vault content (titles, usernames, URLs, notes, names): their
// Debug output shows identifiers and counts only, so it is safe in logs.

impl fmt::Debug for EntrySummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EntrySummary")
            .field("id", &self.id)
            .field("group_id", &self.group_id)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for EntryDetail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EntryDetail")
            .field("summary", &self.summary)
            .field("has_password", &self.has_password)
            .field("history_len", &self.history_len)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for GroupNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GroupNode")
            .field("id", &self.id)
            .field("entry_count", &self.entry_count)
            .field("is_recycle_bin", &self.is_recycle_bin)
            .field("children", &self.children)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for DatabaseInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseInfo")
            .field("cipher", &self.cipher)
            .field("kdf", &self.kdf)
            .field("recycle_bin", &self.recycle_bin)
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}
