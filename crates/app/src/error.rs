//! Errors sent to the webview: a kind only, never a path, a value or a secret.

use keyrust_core::error::VaultError;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct IpcError {
    pub kind: &'static str,
}

impl IpcError {
    pub const fn new(kind: &'static str) -> Self {
        Self { kind }
    }
    pub const LOCKED: Self = Self::new("locked");
    pub const CANCELLED: Self = Self::new("cancelled");
    pub const INVALID: Self = Self::new("invalid");
    pub const IO: Self = Self::new("io");
}

impl From<VaultError> for IpcError {
    fn from(e: VaultError) -> Self {
        Self::new(match e {
            VaultError::WrongKey => "wrongKey",
            VaultError::InvalidFormat => "invalidFormat",
            VaultError::UnsupportedVersion { .. } => "unsupportedVersion",
            VaultError::ExternalChange => "externalChange",
            VaultError::AlreadyExists => "alreadyExists",
            VaultError::NotFound => "notFound",
            VaultError::Invalid(_) | VaultError::NoFile => "invalid",
            VaultError::Io(std::io::ErrorKind::NotFound) => "notFound",
            VaultError::Io(_) => "io",
            VaultError::NotDurable(_) => "notDurable",
            // Future variants of the core stay opaque to the webview.
            #[allow(unreachable_patterns)]
            _ => "io",
        })
    }
}

pub type IpcResult<T> = Result<T, IpcError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_carry_a_kind_and_nothing_else() {
        let json = serde_json::to_string(&IpcError::from(VaultError::UnsupportedVersion {
            major: 5,
            minor: 2,
        }))
        .expect("json");
        assert_eq!(json, r#"{"kind":"unsupportedVersion"}"#);
        assert_eq!(IpcError::from(VaultError::WrongKey).kind, "wrongKey");
        assert_eq!(
            IpcError::from(VaultError::Io(std::io::ErrorKind::NotFound)).kind,
            "notFound"
        );
        assert_eq!(IpcError::from(VaultError::Invalid("title")).kind, "invalid");
    }
}
