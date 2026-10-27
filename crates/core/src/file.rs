//! File primitives: content stamps, the save lock and atomic replacement.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest, Sha256};

/// Identity of a file's content: length and SHA-256 of its bytes. Computed
/// from the very bytes that were parsed or written, never read separately.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stamp {
    len: u64,
    hash: [u8; 32],
}

impl Stamp {
    pub(crate) fn of(bytes: &[u8]) -> Self {
        Self {
            len: bytes.len() as u64,
            hash: Sha256::digest(bytes).into(),
        }
    }
}

/// Whether the file at `path` still has the content `expected`. A missing
/// file counts as changed.
pub(crate) fn unchanged(path: &Path, expected: &Stamp) -> io::Result<bool> {
    match fs::read(path) {
        Ok(bytes) => Ok(Stamp::of(&bytes) == *expected),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// Points where tests inject a concurrent writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    OpenAfterRead,
    SaveAfterFirstCheck,
    SaveAfterFsync,
}

#[cfg(test)]
pub(crate) mod hook {
    use std::cell::RefCell;

    use super::Phase;

    type Hook = Box<dyn FnMut(Phase)>;

    thread_local! {
        static HOOK: RefCell<Option<Hook>> = RefCell::new(None);
    }

    pub(crate) fn set(hook: impl FnMut(Phase) + 'static) {
        HOOK.with(|h| *h.borrow_mut() = Some(Box::new(hook)));
    }

    pub(crate) fn clear() {
        HOOK.with(|h| *h.borrow_mut() = None);
    }

    pub(crate) fn fire(phase: Phase) {
        HOOK.with(|h| {
            if let Some(hook) = h.borrow_mut().as_mut() {
                hook(phase);
            }
        });
    }
}

#[cfg(test)]
pub(crate) use hook::fire;

#[cfg(not(test))]
pub(crate) fn fire(_: Phase) {}

/// Advisory lock shared by Keyrust processes saving the same file:
/// `.<name>.keyrust-lock` next to the database (owner-only permissions). It is
/// left in place; removing it would race with another process taking it.
pub(crate) struct SaveLock {
    _file: File,
}

impl SaveLock {
    pub(crate) fn acquire(path: &Path) -> io::Result<Self> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "vault".to_owned());
        let lock = path.with_file_name(format!(".{name}.keyrust-lock"));
        let file = private_options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(lock)?;
        file.lock()?;
        Ok(Self { _file: file })
    }
}

/// New files readable and writable by their owner only (Unix).
fn private_options() -> OpenOptions {
    #[allow(unused_mut)]
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

/// Creates `<name>.<pid>-<n>.tmp` next to `path`, failing rather than reusing
/// a file that already exists.
pub(crate) fn create_unique_tmp(path: &Path) -> io::Result<(PathBuf, File)> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_owned());
    loop {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_file_name(format!("{name}.{}-{n}.tmp", std::process::id()));
        match private_options().write(true).create_new(true).open(&tmp) {
            Ok(file) => return Ok((tmp, file)),
            // Left over by a crashed process that had the same id.
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
}

/// Outcome of [`replace`] when the file on disk is not the expected one.
#[derive(Debug)]
pub(crate) enum ReplaceError {
    Changed,
    Io(io::Error),
    /// Replaced, but the directory could not be synced.
    NotDurable(io::Error),
}

impl From<io::Error> for ReplaceError {
    fn from(e: io::Error) -> Self {
        ReplaceError::Io(e)
    }
}

/// Atomically replaces `path` with `bytes`: temporary file in the same
/// directory, fsync, rename, directory fsync. With `expected`, the current
/// content is checked before writing and again right before the rename;
/// without it, the file must not exist yet (`create`) unless `overwrite`.
pub(crate) fn replace(
    path: &Path,
    bytes: &[u8],
    expected: Option<&Stamp>,
    overwrite: bool,
) -> Result<(), ReplaceError> {
    let _lock = SaveLock::acquire(path)?;
    let check = || -> Result<(), ReplaceError> {
        match expected {
            Some(stamp) if !unchanged(path, stamp)? => Err(ReplaceError::Changed),
            None if !overwrite && path.exists() => Err(ReplaceError::Io(io::Error::from(
                io::ErrorKind::AlreadyExists,
            ))),
            _ => Ok(()),
        }
    };
    check()?;
    fire(Phase::SaveAfterFirstCheck);

    let (tmp, mut file) = create_unique_tmp(path)?;
    let result = (|| -> Result<(), ReplaceError> {
        // The replacement keeps the original's permissions (a 0600 vault
        // stays 0600); a new file stays owner-only.
        if let Ok(meta) = fs::metadata(path) {
            file.set_permissions(meta.permissions())?;
        }
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fire(Phase::SaveAfterFsync);
        // Last check, as close to the rename as possible.
        check()?;
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result?;
    sync_dir(path).map_err(ReplaceError::NotDurable)
}

/// Makes the rename durable (directory entry). Windows has no directory
/// handle to sync: the rename is flushed with the file system metadata.
fn sync_dir(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        let dir = match path.parent() {
            Some(d) if !d.as_os_str().is_empty() => d,
            _ => Path::new("."),
        };
        File::open(dir)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}
