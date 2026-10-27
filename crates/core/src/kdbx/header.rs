//! Reads the plaintext outer header of a KDBX4 file without decrypting it.
//!
//! The header tells which cipher and which key derivation parameters a file
//! asks for. Reading it first lets the vault refuse files whose KDF settings
//! would exhaust memory or time *before* running the KDF, and lets the UI show
//! a database's encryption settings. This is format parsing only: no
//! cryptographic operation happens here.
//!
//! Input is untrusted: every length is checked before slicing, the header size
//! is bounded and nothing is allocated from a declared length. The parser is
//! fuzzed (`fuzz/fuzz_targets/kdbx_header_peek.rs`).

use std::fmt;

/// Upper bound for the outer header. Real headers are a few hundred bytes;
/// anything larger is rejected rather than scanned.
pub const MAX_HEADER_LEN: usize = 64 * 1024;

const SIGNATURE_1: [u8; 4] = [0x03, 0xd9, 0xa2, 0x9a];
const SIGNATURE_2: [u8; 4] = [0x67, 0xfb, 0x4b, 0xb5];

const FIELD_END: u8 = 0;
const FIELD_CIPHER_ID: u8 = 2;
const FIELD_COMPRESSION: u8 = 3;
const FIELD_KDF_PARAMETERS: u8 = 11;

const CIPHER_AES256: [u8; 16] = uuid(0x31c1f2e6_bf71_4350_be58_05216afc5aff);
const CIPHER_CHACHA20: [u8; 16] = uuid(0xd6038a2b_8b6f_4cb5_a524_339a31dbb59a);
const CIPHER_TWOFISH: [u8; 16] = uuid(0xad68f29f_576f_4bb9_a36a_d47af965346c);

const KDF_AES: [u8; 16] = uuid(0xc9d9f39a_628a_4460_bf74_0d08c18a4fea);
const KDF_ARGON2D: [u8; 16] = uuid(0xef636ddf_8c29_444b_91f7_a9a403e30a0c);
const KDF_ARGON2ID: [u8; 16] = uuid(0x9e298b19_56db_4773_b23d_fc3ec6f0a1e6);

const fn uuid(value: u128) -> [u8; 16] {
    value.to_be_bytes()
}

/// What a KDBX4 file declares in its outer header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderSummary {
    pub major_version: u16,
    pub minor_version: u16,
    pub cipher: Cipher,
    pub compression: Compression,
    pub kdf: Kdf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cipher {
    Aes256,
    ChaCha20,
    Twofish,
    Unknown([u8; 16]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Gzip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Argon2Variant {
    D,
    Id,
}

/// Key derivation settings. Salts and seeds are deliberately not exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kdf {
    AesKdf {
        rounds: u64,
    },
    Argon2 {
        variant: Argon2Variant,
        memory_bytes: u64,
        iterations: u64,
        parallelism: u32,
        version: u32,
    },
    Unknown([u8; 16]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderError {
    /// Not a KeePass 2 file.
    InvalidSignature,
    /// A KDBX version other than 4 (e.g. KDBX 3.1).
    UnsupportedVersion { major: u16, minor: u16 },
    /// The data ends before the header does.
    Truncated,
    /// The header is larger than [`MAX_HEADER_LEN`].
    TooLarge,
    /// A required header field is absent.
    MissingField(&'static str),
    /// A field has an invalid length or value.
    InvalidField(&'static str),
}

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSignature => write!(f, "not a KeePass database"),
            Self::UnsupportedVersion { major, minor } => {
                write!(f, "unsupported KDBX version {major}.{minor}")
            }
            Self::Truncated => write!(f, "truncated database header"),
            Self::TooLarge => write!(f, "database header too large"),
            Self::MissingField(name) => write!(f, "missing header field: {name}"),
            Self::InvalidField(name) => write!(f, "invalid header field: {name}"),
        }
    }
}

impl std::error::Error for HeaderError {}

/// Bounds-checked little-endian reader over a byte slice.
struct Reader<'a> {
    data: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], HeaderError> {
        if len > self.data.len() {
            return Err(HeaderError::Truncated);
        }
        let (head, tail) = self.data.split_at(len);
        self.data = tail;
        Ok(head)
    }

    fn u8(&mut self) -> Result<u8, HeaderError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, HeaderError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, HeaderError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], HeaderError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    /// A length-prefixed (u32) slice, rejected if it overruns the data.
    fn sized(&mut self) -> Result<&'a [u8], HeaderError> {
        let len = usize::try_from(self.u32()?).map_err(|_| HeaderError::TooLarge)?;
        self.take(len)
    }
}

/// Reads the outer header of a KDBX4 file. Only the first [`MAX_HEADER_LEN`]
/// bytes are ever looked at.
pub fn peek(data: &[u8]) -> Result<HeaderSummary, HeaderError> {
    let mut r = Reader {
        data: &data[..data.len().min(MAX_HEADER_LEN)],
    };
    if r.take(4)? != SIGNATURE_1 || r.take(4)? != SIGNATURE_2 {
        return Err(HeaderError::InvalidSignature);
    }
    let minor_version = r.u16()?;
    let major_version = r.u16()?;
    if major_version != 4 {
        return Err(HeaderError::UnsupportedVersion {
            major: major_version,
            minor: minor_version,
        });
    }

    let (mut cipher, mut compression, mut kdf) = (None, None, None);
    loop {
        let id = match r.u8() {
            Ok(id) => id,
            // Ran past the size cap without finding the end of the header.
            Err(_) if data.len() > MAX_HEADER_LEN => return Err(HeaderError::TooLarge),
            Err(e) => return Err(e),
        };
        let value = match r.sized() {
            Err(HeaderError::Truncated) if data.len() > MAX_HEADER_LEN => {
                return Err(HeaderError::TooLarge);
            }
            other => other?,
        };
        match id {
            FIELD_END => break,
            FIELD_CIPHER_ID => {
                let id: [u8; 16] = value
                    .try_into()
                    .map_err(|_| HeaderError::InvalidField("cipher"))?;
                cipher = Some(match id {
                    CIPHER_AES256 => Cipher::Aes256,
                    CIPHER_CHACHA20 => Cipher::ChaCha20,
                    CIPHER_TWOFISH => Cipher::Twofish,
                    other => Cipher::Unknown(other),
                });
            }
            FIELD_COMPRESSION => {
                let flags: [u8; 4] = value
                    .try_into()
                    .map_err(|_| HeaderError::InvalidField("compression"))?;
                compression = Some(match u32::from_le_bytes(flags) {
                    0 => Compression::None,
                    1 => Compression::Gzip,
                    _ => return Err(HeaderError::InvalidField("compression")),
                });
            }
            FIELD_KDF_PARAMETERS => kdf = Some(parse_kdf(value)?),
            // Seeds, IVs and public custom data: not needed to describe the file.
            _ => {}
        }
    }

    Ok(HeaderSummary {
        major_version,
        minor_version,
        cipher: cipher.ok_or(HeaderError::MissingField("cipher"))?,
        compression: compression.ok_or(HeaderError::MissingField("compression"))?,
        kdf: kdf.ok_or(HeaderError::MissingField("kdf"))?,
    })
}

/// Value of a KeePass `VariantDictionary` item, limited to the types the KDF
/// parameters use.
enum Variant<'a> {
    U32(u32),
    U64(u64),
    Bytes(&'a [u8]),
    Other,
}

const VARIANT_END: u8 = 0x00;
const VARIANT_U32: u8 = 0x04;
const VARIANT_U64: u8 = 0x05;
const VARIANT_BYTES: u8 = 0x42;

fn parse_kdf(data: &[u8]) -> Result<Kdf, HeaderError> {
    const FIELD: &str = "kdf";
    let mut r = Reader { data };
    // Version 1.x: the high byte is the major version.
    if r.u16().map_err(|_| HeaderError::InvalidField(FIELD))? >> 8 != 1 {
        return Err(HeaderError::InvalidField(FIELD));
    }

    let (mut uuid, mut rounds, mut memory, mut iterations, mut parallelism, mut version) =
        (None, None, None, None, None, None);
    loop {
        let kind = r.u8().map_err(|_| HeaderError::InvalidField(FIELD))?;
        if kind == VARIANT_END {
            break;
        }
        let name = r.sized().map_err(|_| HeaderError::InvalidField(FIELD))?;
        let raw = r.sized().map_err(|_| HeaderError::InvalidField(FIELD))?;
        let value = match kind {
            VARIANT_U32 => Variant::U32(u32::from_le_bytes(
                raw.try_into()
                    .map_err(|_| HeaderError::InvalidField(FIELD))?,
            )),
            VARIANT_U64 => Variant::U64(u64::from_le_bytes(
                raw.try_into()
                    .map_err(|_| HeaderError::InvalidField(FIELD))?,
            )),
            VARIANT_BYTES => Variant::Bytes(raw),
            _ => Variant::Other,
        };
        match (name, value) {
            (b"$UUID", Variant::Bytes(b)) => {
                uuid = Some(<[u8; 16]>::try_from(b).map_err(|_| HeaderError::InvalidField(FIELD))?)
            }
            (b"R", Variant::U64(v)) => rounds = Some(v),
            (b"M", Variant::U64(v)) => memory = Some(v),
            (b"I", Variant::U64(v)) => iterations = Some(v),
            (b"P", Variant::U32(v)) => parallelism = Some(v),
            (b"V", Variant::U32(v)) => version = Some(v),
            _ => {}
        }
    }

    let missing = || HeaderError::InvalidField(FIELD);
    Ok(match uuid.ok_or(HeaderError::MissingField("kdf uuid"))? {
        KDF_AES => Kdf::AesKdf {
            rounds: rounds.ok_or_else(missing)?,
        },
        id @ (KDF_ARGON2D | KDF_ARGON2ID) => Kdf::Argon2 {
            variant: if id == KDF_ARGON2ID {
                Argon2Variant::Id
            } else {
                Argon2Variant::D
            },
            memory_bytes: memory.ok_or_else(missing)?,
            iterations: iterations.ok_or_else(missing)?,
            parallelism: parallelism.ok_or_else(missing)?,
            version: version.ok_or_else(missing)?,
        },
        other => Kdf::Unknown(other),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal KDBX4 header from raw fields.
    fn header(fields: &[(u8, Vec<u8>)]) -> Vec<u8> {
        let mut out = [SIGNATURE_1, SIGNATURE_2].concat();
        out.extend(1u16.to_le_bytes());
        out.extend(4u16.to_le_bytes());
        for (id, value) in fields {
            out.push(*id);
            out.extend((value.len() as u32).to_le_bytes());
            out.extend(value);
        }
        out
    }

    fn variant(items: &[(u8, &[u8], Vec<u8>)]) -> Vec<u8> {
        let mut out = 0x0100u16.to_le_bytes().to_vec();
        for (kind, name, value) in items {
            out.push(*kind);
            out.extend((name.len() as u32).to_le_bytes());
            out.extend(*name);
            out.extend((value.len() as u32).to_le_bytes());
            out.extend(value);
        }
        out.push(VARIANT_END);
        out
    }

    fn argon2id_params() -> Vec<u8> {
        variant(&[
            (VARIANT_BYTES, b"$UUID", KDF_ARGON2ID.to_vec()),
            (VARIANT_BYTES, b"S", vec![7; 32]),
            (VARIANT_U32, b"P", 2u32.to_le_bytes().to_vec()),
            (VARIANT_U64, b"M", (64u64 << 20).to_le_bytes().to_vec()),
            (VARIANT_U64, b"I", 3u64.to_le_bytes().to_vec()),
            (VARIANT_U32, b"V", 0x13u32.to_le_bytes().to_vec()),
        ])
    }

    fn valid() -> Vec<u8> {
        header(&[
            (FIELD_CIPHER_ID, CIPHER_CHACHA20.to_vec()),
            (FIELD_COMPRESSION, 1u32.to_le_bytes().to_vec()),
            (FIELD_KDF_PARAMETERS, argon2id_params()),
            (FIELD_END, b"\r\n\r\n".to_vec()),
        ])
    }

    #[test]
    fn reads_a_well_formed_header() {
        assert_eq!(
            peek(&valid()),
            Ok(HeaderSummary {
                major_version: 4,
                minor_version: 1,
                cipher: Cipher::ChaCha20,
                compression: Compression::Gzip,
                kdf: Kdf::Argon2 {
                    variant: Argon2Variant::Id,
                    memory_bytes: 64 << 20,
                    iterations: 3,
                    parallelism: 2,
                    version: 0x13,
                },
            })
        );
    }

    #[test]
    fn every_truncation_is_an_error_not_a_panic() {
        let data = valid();
        for len in 0..data.len() {
            assert!(
                peek(&data[..len]).is_err(),
                "prefix of length {len} accepted"
            );
        }
    }

    #[test]
    fn rejects_bad_signature_and_versions() {
        let mut data = valid();
        data[0] ^= 1;
        assert_eq!(peek(&data), Err(HeaderError::InvalidSignature));

        let mut data = valid();
        data[10] = 3; // major version 3
        assert_eq!(
            peek(&data),
            Err(HeaderError::UnsupportedVersion { major: 3, minor: 1 })
        );
    }

    #[test]
    fn rejects_lengths_beyond_the_data() {
        let mut data = header(&[(FIELD_CIPHER_ID, CIPHER_AES256.to_vec())]);
        data.push(FIELD_COMPRESSION);
        data.extend(u32::MAX.to_le_bytes());
        assert_eq!(peek(&data), Err(HeaderError::Truncated));
    }

    #[test]
    fn rejects_oversized_headers() {
        let data = header(&[(99, vec![0; MAX_HEADER_LEN])]);
        assert_eq!(peek(&data), Err(HeaderError::TooLarge));
    }

    #[test]
    fn reports_missing_and_invalid_fields() {
        let data = header(&[
            (FIELD_CIPHER_ID, CIPHER_AES256.to_vec()),
            (FIELD_COMPRESSION, 0u32.to_le_bytes().to_vec()),
            (FIELD_END, vec![]),
        ]);
        assert_eq!(peek(&data), Err(HeaderError::MissingField("kdf")));

        let data = header(&[(FIELD_CIPHER_ID, vec![1, 2, 3]), (FIELD_END, vec![])]);
        assert_eq!(peek(&data), Err(HeaderError::InvalidField("cipher")));

        let data = header(&[(FIELD_COMPRESSION, 7u32.to_le_bytes().to_vec())]);
        assert_eq!(peek(&data), Err(HeaderError::InvalidField("compression")));

        let params = variant(&[(VARIANT_BYTES, b"$UUID", KDF_ARGON2D.to_vec())]);
        let data = header(&[
            (FIELD_CIPHER_ID, CIPHER_AES256.to_vec()),
            (FIELD_COMPRESSION, 0u32.to_le_bytes().to_vec()),
            (FIELD_KDF_PARAMETERS, params),
            (FIELD_END, vec![]),
        ]);
        assert_eq!(peek(&data), Err(HeaderError::InvalidField("kdf")));
    }

    #[test]
    fn unknown_cipher_and_kdf_are_reported_not_rejected() {
        let params = variant(&[(VARIANT_BYTES, b"$UUID", vec![9; 16])]);
        let data = header(&[
            (FIELD_CIPHER_ID, vec![8; 16]),
            (FIELD_COMPRESSION, 0u32.to_le_bytes().to_vec()),
            (FIELD_KDF_PARAMETERS, params),
            (FIELD_END, vec![]),
        ]);
        let summary = peek(&data).expect("valid structure");
        assert_eq!(summary.cipher, Cipher::Unknown([8; 16]));
        assert_eq!(summary.kdf, Kdf::Unknown([9; 16]));
    }
}
