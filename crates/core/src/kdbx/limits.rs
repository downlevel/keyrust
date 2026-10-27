//! Resource limits checked before any key derivation.
//!
//! A database file states how much memory and time its key derivation needs.
//! That part of the file is read before anything is authenticated, so a
//! hostile copy (e.g. altered by a sync provider) could ask for gigabytes of
//! memory or hours of computation. These limits refuse such files from the
//! header alone, at constant cost.

use super::header::{self, HeaderError, Kdf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_file_bytes: u64,
    pub max_argon2_memory_bytes: u64,
    pub max_argon2_iterations: u64,
    pub max_argon2_parallelism: u32,
    pub max_aes_rounds: u64,
}

impl Limits {
    /// Far above any normal database (KeePassXC's defaults are 64 MiB and
    /// about one second), low enough to keep a hostile file from freezing the
    /// computer.
    pub const DEFAULT: Self = Self {
        max_file_bytes: 512 * 1024 * 1024,
        max_argon2_memory_bytes: 2 * 1024 * 1024 * 1024,
        max_argon2_iterations: 1_000,
        max_argon2_parallelism: 256,
        max_aes_rounds: 2_000_000_000,
    };
}

impl Limits {
    /// Tiny limits for fuzzing: every input stays fast, so the fuzzer spends
    /// its time past the key derivation.
    pub const FUZZ: Self = Self {
        max_file_bytes: 1024 * 1024,
        max_argon2_memory_bytes: 1024 * 1024,
        max_argon2_iterations: 2,
        max_argon2_parallelism: 1,
        max_aes_rounds: 1_000,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Which limit a file exceeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exceeded {
    FileSize,
    Argon2Memory,
    Argon2Iterations,
    Argon2Parallelism,
    AesRounds,
}

/// Checks `bytes` (a whole file) against `limits` using only its header.
/// Files whose header cannot be read pass: parsing then fails normally.
pub fn check(bytes: &[u8], limits: &Limits) -> Result<(), Exceeded> {
    if bytes.len() as u64 > limits.max_file_bytes {
        return Err(Exceeded::FileSize);
    }
    match header::peek(bytes) {
        Ok(summary) => check_kdf(&summary.kdf, limits),
        Err(HeaderError::UnsupportedVersion { major: 3, .. }) => {
            match kdbx3_transform_rounds(bytes) {
                Some(rounds) if rounds > limits.max_aes_rounds => Err(Exceeded::AesRounds),
                _ => Ok(()),
            }
        }
        Err(_) => Ok(()),
    }
}

fn check_kdf(kdf: &Kdf, limits: &Limits) -> Result<(), Exceeded> {
    match *kdf {
        Kdf::AesKdf { rounds } if rounds > limits.max_aes_rounds => Err(Exceeded::AesRounds),
        Kdf::Argon2 {
            memory_bytes,
            iterations,
            parallelism,
            ..
        } => {
            if memory_bytes > limits.max_argon2_memory_bytes {
                Err(Exceeded::Argon2Memory)
            } else if iterations > limits.max_argon2_iterations {
                Err(Exceeded::Argon2Iterations)
            } else if parallelism > limits.max_argon2_parallelism {
                Err(Exceeded::Argon2Parallelism)
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

/// `TransformRounds` (AES-KDF) of a KDBX 3.x header: fields are
/// `id: u8, size: u16 LE, data`; id 6 holds a u64 LE; id 0 ends the header.
fn kdbx3_transform_rounds(bytes: &[u8]) -> Option<u64> {
    const TRANSFORM_ROUNDS: u8 = 6;
    let mut pos = 12; // signatures (8) + version (4)
    let end = bytes.len().min(header::MAX_HEADER_LEN);
    while pos + 3 <= end {
        let id = bytes[pos];
        let size = usize::from(u16::from_le_bytes([bytes[pos + 1], bytes[pos + 2]]));
        let data = bytes.get(pos + 3..pos + 3 + size)?;
        match id {
            0 => return None,
            TRANSFORM_ROUNDS => return data.try_into().ok().map(u64::from_le_bytes),
            _ => pos += 3 + size,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures")
                .join(name),
        )
        .expect("fixture")
    }

    #[test]
    fn the_fixtures_are_within_the_default_limits() {
        for name in ["keepassxc-rich.kdbx", "keepassxc-kdbx31.kdbx"] {
            assert_eq!(check(&fixture(name), &Limits::DEFAULT), Ok(()), "{name}");
        }
    }

    #[test]
    fn kdbx3_rounds_are_read_and_limited() {
        let bytes = fixture("keepassxc-kdbx31.kdbx");
        let rounds = kdbx3_transform_rounds(&bytes).expect("rounds");
        assert!(rounds > 0);
        let strict = Limits {
            max_aes_rounds: rounds - 1,
            ..Limits::DEFAULT
        };
        assert_eq!(check(&bytes, &strict), Err(Exceeded::AesRounds));
    }

    #[test]
    fn file_size_is_limited() {
        let strict = Limits {
            max_file_bytes: 10,
            ..Limits::DEFAULT
        };
        assert_eq!(check(&[0u8; 11], &strict), Err(Exceeded::FileSize));
        assert_eq!(check(b"not kdbx", &strict), Ok(()));
    }

    #[test]
    fn truncated_kdbx3_headers_do_not_panic() {
        let bytes = fixture("keepassxc-kdbx31.kdbx");
        for len in 0..200.min(bytes.len()) {
            let _ = check(&bytes[..len], &Limits::DEFAULT);
        }
    }
}
