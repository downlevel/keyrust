//! Errors of the vault API. Messages never contain a secret or a path, so they
//! can be logged and shown to the user as they are.

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("wrong password or key")]
    WrongKey,
    #[error("not a valid KeePass database")]
    InvalidFormat,
    #[error("unsupported database version {major}.{minor}")]
    UnsupportedVersion { major: u16, minor: u16 },
    #[error("the file was changed by another application")]
    ExternalChange,
    /// The file asks for more memory, time or size than `Limits` allow.
    #[error("the database asks for more resources than allowed")]
    TooExpensive,
    #[error("the file already exists")]
    AlreadyExists,
    #[error("this vault has no file to save to")]
    NoFile,
    #[error("no such entry or group")]
    NotFound,
    #[error("invalid {0}")]
    Invalid(&'static str),
    #[error("I/O error ({0:?})")]
    Io(io::ErrorKind),
    /// The file was written and replaced, but the system could not confirm
    /// that the change reached the disk (directory sync failed).
    #[error("saved, but the change may not be on disk yet ({0:?})")]
    NotDurable(io::ErrorKind),
}

impl From<io::Error> for VaultError {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::AlreadyExists => VaultError::AlreadyExists,
            kind => VaultError::Io(kind),
        }
    }
}

pub type Result<T, E = VaultError> = std::result::Result<T, E>;
