//! Application settings, persisted as `settings.json` in the OS config dir.
//!
//! Settings never contain secrets: there is no field that could hold a
//! password, a key or vault content. Loading is forgiving (a missing or
//! corrupt file yields defaults) and saving is atomic, so a crash never leaves
//! a half-written file behind.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Deserializer, Serialize};

pub const FILE_NAME: &str = "settings.json";
pub const CURRENT_VERSION: u32 = 1;
pub const MAX_RECENT_FILES: usize = 10;

pub const CLIPBOARD_CLEAR_SECS: std::ops::RangeInclusive<u32> = 5..=300;
pub const IDLE_LOCK_MINS: std::ops::RangeInclusive<u32> = 1..=240;
pub const PASSWORD_LENGTH: std::ops::RangeInclusive<u32> = 8..=128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(deserialize_with = "saturating_u32")]
    pub version: u32,
    /// Most recent first, without duplicates.
    pub recent_files: Vec<PathBuf>,
    #[serde(deserialize_with = "saturating_u32")]
    pub clipboard_clear_secs: u32,
    #[serde(deserialize_with = "saturating_u32")]
    pub idle_lock_mins: u32,
    pub lock_on_screen_lock: bool,
    pub lock_on_minimize: bool,
    pub generator: GeneratorDefaults,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneratorDefaults {
    #[serde(deserialize_with = "saturating_u32")]
    pub length: u32,
    pub lowercase: bool,
    pub uppercase: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            recent_files: Vec::new(),
            clipboard_clear_secs: 20,
            idle_lock_mins: 5,
            lock_on_screen_lock: true,
            lock_on_minimize: false,
            generator: GeneratorDefaults::default(),
        }
    }
}

impl Default for GeneratorDefaults {
    fn default() -> Self {
        Self {
            length: 20,
            lowercase: true,
            uppercase: true,
            digits: true,
            symbols: true,
            exclude_ambiguous: false,
        }
    }
}

impl Settings {
    /// Brings every value back into its valid range.
    pub fn normalized(mut self) -> Self {
        self.version = CURRENT_VERSION;
        self.clipboard_clear_secs = clamp(self.clipboard_clear_secs, &CLIPBOARD_CLEAR_SECS);
        self.idle_lock_mins = clamp(self.idle_lock_mins, &IDLE_LOCK_MINS);
        let g = &mut self.generator;
        g.length = clamp(g.length, &PASSWORD_LENGTH);
        if !(g.lowercase || g.uppercase || g.digits || g.symbols) {
            g.lowercase = true;
        }
        let mut seen = Vec::with_capacity(self.recent_files.len());
        self.recent_files.retain(|p| {
            let new = !seen.contains(p);
            seen.push(p.clone());
            new
        });
        self.recent_files.truncate(MAX_RECENT_FILES);
        self
    }

    /// Moves `path` to the front of the recent files.
    pub fn push_recent_file(&mut self, path: PathBuf) {
        self.recent_files.retain(|p| *p != path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(MAX_RECENT_FILES);
    }
}

fn clamp(value: u32, range: &std::ops::RangeInclusive<u32>) -> u32 {
    value.clamp(*range.start(), *range.end())
}

/// Reads any JSON number into a `u32`, saturating (negative → 0, too large →
/// `u32::MAX`, fractions truncated), so that an out-of-range value is clamped
/// by [`Settings::normalized`] instead of making the whole file invalid.
fn saturating_u32<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let n = serde_json::Number::deserialize(deserializer)?;
    Ok(if let Some(v) = n.as_u64() {
        u32::try_from(v).unwrap_or(u32::MAX)
    } else if n.as_i64().is_some() {
        0 // a negative integer
    } else {
        // `as` saturates and maps NaN to 0.
        n.as_f64().map_or(0, |v| v as u32)
    })
}

/// Loads settings, falling back to defaults instead of failing.
///
/// A file that cannot be parsed is renamed to `<name>.corrupt` (so the user
/// can inspect it) and defaults are returned. Its content is never logged.
pub fn load(path: &Path) -> Settings {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            tracing::info!("no settings file, using defaults");
            return Settings::default();
        }
        Err(e) => {
            tracing::warn!(error_kind = ?e.kind(), "cannot read settings, using defaults");
            return Settings::default();
        }
    };
    match serde_json::from_slice::<Settings>(&bytes) {
        Ok(settings) => {
            tracing::info!("settings loaded");
            settings.normalized()
        }
        Err(_) => {
            let backup = path.with_extension("json.corrupt");
            let renamed = fs::rename(path, &backup).is_ok();
            tracing::warn!(renamed, "settings file is not valid, using defaults");
            tracing::debug!(path = %path.display(), "invalid settings file");
            Settings::default()
        }
    }
}

/// Saves settings atomically: write a temporary file next to the target,
/// flush it to disk, then rename it over the target.
///
/// The temporary file is created exclusively under a name unique to this save
/// (process id and counter), so concurrent saves — even from two app
/// processes — never write into the same file; the last rename wins whole.
pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(dir)?;
    let json =
        serde_json::to_vec_pretty(&settings.clone().normalized()).map_err(io::Error::other)?;

    let (tmp, mut file) = create_unique_tmp(path)?;
    let result = (|| {
        file.write_all(&json)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    match &result {
        Ok(()) => tracing::info!("settings saved"),
        Err(e) => tracing::warn!(error_kind = ?e.kind(), "cannot save settings"),
    }
    tracing::debug!(path = %path.display(), "settings file");
    result
}

/// Creates `<name>.<pid>-<n>.tmp` next to `path`, failing rather than reusing
/// a file that already exists.
fn create_unique_tmp(path: &Path) -> io::Result<(PathBuf, fs::File)> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let name = path
        .file_name()
        .unwrap_or(FILE_NAME.as_ref())
        .to_string_lossy();
    loop {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_file_name(format!("{name}.{}-{n}.tmp", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(file) => return Ok((tmp, file)),
            // Left over by a crashed process that had the same id.
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join(FILE_NAME);
        (dir, path)
    }

    #[test]
    fn missing_file_gives_defaults() {
        let (_dir, path) = temp();
        assert_eq!(load(&path), Settings::default());
    }

    #[test]
    fn round_trip_creates_directory_and_leaves_no_temp_file() {
        let (_dir, path) = temp();
        let mut settings = Settings {
            clipboard_clear_secs: 45,
            ..Settings::default()
        };
        settings.push_recent_file("/vaults/a.kdbx".into());
        save(&path, &settings).expect("save");
        assert_eq!(load(&path), settings);
        let names: Vec<_> = fs::read_dir(path.parent().expect("dir"))
            .expect("read dir")
            .map(|e| e.expect("entry").file_name())
            .collect();
        assert_eq!(names, [FILE_NAME]);
    }

    #[test]
    fn corrupt_file_gives_defaults_and_is_kept_aside() {
        let (_dir, path) = temp();
        fs::create_dir_all(path.parent().expect("dir")).expect("mkdir");
        fs::write(&path, b"{ not json").expect("write");
        assert_eq!(load(&path), Settings::default());
        assert!(!path.exists());
        assert_eq!(
            fs::read(path.with_extension("json.corrupt")).expect("backup"),
            b"{ not json"
        );
    }

    #[test]
    fn out_of_range_values_are_clamped_and_unknown_fields_ignored() {
        let (_dir, path) = temp();
        fs::create_dir_all(path.parent().expect("dir")).expect("mkdir");
        fs::write(
            &path,
            br#"{"version": 99, "clipboard_clear_secs": 0, "idle_lock_mins": 100000,
                "future_option": true,
                "generator": {"length": 4000, "lowercase": false, "uppercase": false,
                              "digits": false, "symbols": false}}"#,
        )
        .expect("write");
        let s = load(&path);
        assert_eq!(s.version, CURRENT_VERSION);
        assert_eq!(s.clipboard_clear_secs, 5);
        assert_eq!(s.idle_lock_mins, 240);
        assert_eq!(s.generator.length, 128);
        assert!(
            s.generator.lowercase,
            "at least one character set stays enabled"
        );
        // Fields absent from the file take their defaults.
        assert!(s.lock_on_screen_lock);
    }

    #[test]
    fn negative_and_oversized_numbers_are_clamped_without_losing_other_settings() {
        let (_dir, path) = temp();
        fs::create_dir_all(path.parent().expect("dir")).expect("mkdir");
        fs::write(
            &path,
            br#"{"clipboard_clear_secs": -1, "idle_lock_mins": 99999999999999999999999,
                "lock_on_minimize": true, "generator": {"length": 12.7}}"#,
        )
        .expect("write");
        let s = load(&path);
        assert_eq!(s.clipboard_clear_secs, 5);
        assert_eq!(s.idle_lock_mins, 240);
        assert_eq!(s.generator.length, 12);
        assert!(s.lock_on_minimize, "unrelated settings are kept");
        assert!(path.exists(), "the file is not treated as corrupt");
    }

    #[test]
    fn concurrent_saves_never_mix_contents() {
        let (_dir, path) = temp();
        let variants: Vec<Settings> = (0..8)
            .map(|i| Settings {
                clipboard_clear_secs: 10 + i,
                recent_files: (0..10)
                    .map(|n| format!("/vaults/{i}-{n}.kdbx").into())
                    .collect(),
                ..Settings::default()
            })
            .collect();
        std::thread::scope(|scope| {
            for settings in &variants {
                let path = &path;
                scope.spawn(move || {
                    for _ in 0..20 {
                        save(path, settings).expect("save");
                    }
                });
            }
        });
        let raw = fs::read(&path).expect("read");
        let saved: Settings = serde_json::from_slice(&raw).expect("whole JSON");
        assert!(variants.contains(&saved), "the file is one of the saves");
        let names: Vec<_> = fs::read_dir(path.parent().expect("dir"))
            .expect("read dir")
            .map(|e| e.expect("entry").file_name())
            .collect();
        assert_eq!(names, [FILE_NAME], "no temporary file is left behind");
    }

    #[test]
    fn recent_files_are_deduplicated_most_recent_first_and_capped() {
        let mut s = Settings::default();
        for i in 0..15 {
            s.push_recent_file(format!("/v/{i}.kdbx").into());
        }
        s.push_recent_file("/v/3.kdbx".into());
        assert_eq!(s.recent_files.len(), MAX_RECENT_FILES);
        assert_eq!(s.recent_files[0], PathBuf::from("/v/3.kdbx"));
        assert_eq!(
            s.recent_files
                .iter()
                .filter(|p| p.as_path() == Path::new("/v/3.kdbx"))
                .count(),
            1
        );

        let s = Settings {
            recent_files: vec!["/a".into(), "/b".into(), "/a".into()],
            ..s
        }
        .normalized();
        assert_eq!(s.recent_files, [PathBuf::from("/a"), PathBuf::from("/b")]);
    }
}
