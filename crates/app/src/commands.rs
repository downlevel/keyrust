//! IPC commands: a thin layer over `keyrust_core::vault::Vault`.
//!
//! Lists, details, the tree and database info never carry a password. A
//! password crosses the boundary only when the user reveals it, edits an
//! entry, or types one (unlock, create, save, change master password), and it
//! is deserialised straight into a `SecretString`.
//!
//! Paths reach `unlock` and `create` only if they came from a native dialog or
//! the recent-files list: the webview cannot make Keyrust open or create files
//! at paths of its choosing.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use keyrust_core::generator::{self, GeneratorOptions};
use keyrust_core::model::{
    Argon2idParams, Cipher, CreateOptions, DatabaseInfo, EntryDetail, EntryDraft, EntryId,
    EntrySummary, Field, GroupId, GroupNode,
};
use keyrust_core::settings::{self, Settings};
use keyrust_core::strength::{self, StrengthEstimate};
use keyrust_core::vault::Vault;
use keyrust_core::{kdf, settings::MAX_RECENT_FILES};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::autolock::{AutoLock, Session};
use crate::clipboard::Clipboard;
use crate::error::{IpcError, IpcResult};

// ---------------------------------------------------------------- state

/// The open database, if any, and its lock generation (see [`Session`]).
#[derive(Default)]
pub struct VaultState(pub Mutex<Session<Vault>>);

impl VaultState {
    pub fn is_open(&self) -> bool {
        self.0.lock().is_ok_and(|s| s.is_open())
    }
}

/// Settings held in memory and the file they persist to.
pub struct SettingsState {
    pub path: PathBuf,
    pub current: Mutex<Settings>,
}

/// Paths the user chose in a dialog or that are in the recent files.
#[derive(Default)]
pub struct AllowedPaths(pub Mutex<HashSet<PathBuf>>);

fn lock<T>(m: &Mutex<T>) -> IpcResult<MutexGuard<'_, T>> {
    m.lock().map_err(|_| IpcError::IO)
}

fn with_vault<T>(state: &VaultState, f: impl FnOnce(&mut Vault) -> IpcResult<T>) -> IpcResult<T> {
    let mut guard = lock(&state.0)?;
    let vault = guard.vault_mut().ok_or(IpcError::LOCKED)?;
    f(vault)
}

/// Runs blocking work (key derivation, dialogs) off the async runtime.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> IpcResult<T> + Send + 'static,
) -> IpcResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|_| IpcError::IO)?
}

// ---------------------------------------------------------------- views

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseView {
    #[serde(flatten)]
    pub info: DatabaseInfo,
    pub dirty: bool,
}

fn view(vault: &Vault) -> DatabaseView {
    DatabaseView {
        info: vault.info(),
        dirty: vault.is_dirty(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentFile {
    pub name: String,
    pub path: String,
    pub missing: bool,
}

fn recent(path: &Path) -> RecentFile {
    RecentFile {
        name: path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path: path.display().to_string(),
        missing: !path.exists(),
    }
}

// ---------------------------------------------------------------- settings

/// Records `path` as the most recent file and persists the settings.
pub fn remember_recent(state: &SettingsState, path: &Path) {
    let Ok(mut current) = state.current.lock() else {
        return;
    };
    let mut next = current.clone();
    next.push_recent_file(path.to_path_buf());
    if settings::save(&state.path, &next).is_ok() {
        *current = next;
    }
}

/// Replaces the user preferences. `recent_files` belongs to the backend: the
/// value sent by the webview is ignored, so a stale snapshot cannot erase a
/// file opened meanwhile.
pub fn apply_settings(state: &SettingsState, incoming: Settings) -> IpcResult<Settings> {
    let mut current = lock(&state.current)?;
    let mut next = incoming;
    next.recent_files = current.recent_files.clone();
    let next = next.normalized();
    settings::save(&state.path, &next).map_err(|_| IpcError::IO)?;
    *current = next.clone();
    Ok(next)
}

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> IpcResult<Settings> {
    Ok(lock(&state.current)?.clone())
}

#[tauri::command]
pub fn update_settings(state: State<'_, SettingsState>, settings: Settings) -> IpcResult<Settings> {
    apply_settings(&state, settings)
}

// ---------------------------------------------------------------- files

#[tauri::command]
pub fn recent_files(
    settings: State<'_, SettingsState>,
    allowed: State<'_, AllowedPaths>,
) -> IpcResult<Vec<RecentFile>> {
    let paths = lock(&settings.current)?.recent_files.clone();
    let mut allowed = lock(&allowed.0)?;
    Ok(paths
        .iter()
        .take(MAX_RECENT_FILES)
        .map(|p| {
            allowed.insert(p.clone());
            recent(p)
        })
        .collect())
}

const KDBX_FILTER: (&str, &[&str]) = ("KeePass database", &["kdbx"]);

#[tauri::command]
pub async fn choose_database_file(app: AppHandle) -> IpcResult<RecentFile> {
    let dialog = app.clone();
    let path = blocking(move || {
        dialog
            .dialog()
            .file()
            .add_filter(KDBX_FILTER.0, KDBX_FILTER.1)
            .blocking_pick_file()
            .ok_or(IpcError::CANCELLED)?
            .into_path()
            .map_err(|_| IpcError::INVALID)
    })
    .await?;
    lock(&app.state::<AllowedPaths>().0)?.insert(path.clone());
    Ok(recent(&path))
}

#[tauri::command]
pub async fn choose_new_database_path(app: AppHandle, name: String) -> IpcResult<RecentFile> {
    let dialog = app.clone();
    let file_name = format!("{}.kdbx", sanitize_file_name(&name));
    let path = blocking(move || {
        dialog
            .dialog()
            .file()
            .add_filter(KDBX_FILTER.0, KDBX_FILTER.1)
            .set_file_name(file_name)
            .blocking_save_file()
            .ok_or(IpcError::CANCELLED)?
            .into_path()
            .map_err(|_| IpcError::INVALID)
    })
    .await?;
    let path = with_kdbx_extension(path);
    lock(&app.state::<AllowedPaths>().0)?.insert(path.clone());
    Ok(recent(&path))
}

pub fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || " -_.".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('.').trim();
    if cleaned.is_empty() {
        "Passwords".to_owned()
    } else {
        cleaned.to_owned()
    }
}

pub fn with_kdbx_extension(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("kdbx"))
    {
        path
    } else {
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(".kdbx");
        path.with_file_name(name)
    }
}

fn allowed_path(app: &AppHandle, path: &str) -> IpcResult<PathBuf> {
    let path = PathBuf::from(path);
    if lock(&app.state::<AllowedPaths>().0)?.contains(&path) {
        Ok(path)
    } else {
        Err(IpcError::INVALID)
    }
}

// ---------------------------------------------------------------- session

#[tauri::command]
pub async fn unlock(
    app: AppHandle,
    path: String,
    password: SecretString,
) -> IpcResult<DatabaseView> {
    let path = allowed_path(&app, &path)?;
    let opened = path.clone();
    let generation = lock(&app.state::<VaultState>().0)?.generation();
    let vault = blocking(move || Vault::open(&opened, password).map_err(IpcError::from)).await?;
    remember_recent(&app.state::<SettingsState>(), &path);
    install(&app, vault, generation)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    pub path: String,
    pub name: String,
    pub password: SecretString,
    pub cipher: Cipher,
    pub kdf: Argon2idParams,
}

#[tauri::command]
pub async fn create(app: AppHandle, request: CreateRequest) -> IpcResult<DatabaseView> {
    let path = allowed_path(&app, &request.path)?;
    let target = path.clone();
    let generation = lock(&app.state::<VaultState>().0)?.generation();
    let vault = blocking(move || {
        Vault::create(
            &target,
            CreateOptions {
                name: request.name,
                password: request.password,
                cipher: request.cipher,
                kdf: request.kdf,
            },
        )
        .map_err(IpcError::from)
    })
    .await?;
    remember_recent(&app.state::<SettingsState>(), &path);
    install(&app, vault, generation)
}

/// Makes `vault` the open database unless a lock happened since `generation`
/// was read: then it is dropped (zeroized) and the answer is `locked`.
fn install(app: &AppHandle, vault: Vault, generation: u64) -> IpcResult<DatabaseView> {
    let result = view(&vault);
    let state = app.state::<VaultState>();
    let mut session = lock(&state.0)?;
    if let Err(refused) = session.install(vault, generation) {
        drop(session);
        drop(refused);
        tracing::info!("locked while opening: database discarded");
        return Err(IpcError::LOCKED);
    }
    // Under the session mutex, so a pending idle lock sees the fresh activity.
    app.state::<AutoLock>().touch();
    Ok(result)
}

/// Drops the vault (its secrets are zeroized), invalidates any unlock in
/// progress and clears the clipboard if it still holds a copied value.
/// Returns whether a vault was open. A poisoned mutex does not stop a lock.
pub fn lock_session(state: &VaultState, clipboard: &Clipboard) -> bool {
    lock_session_if(state, clipboard, || true)
}

/// Like [`lock_session`], if `still_wanted` holds once the session mutex is
/// taken (an idle lock re-checks the activity there: a save may have held the
/// mutex while the user went on working).
pub fn lock_session_if(
    state: &VaultState,
    clipboard: &Clipboard,
    still_wanted: impl FnOnce() -> bool,
) -> bool {
    let mut session = state.0.lock().unwrap_or_else(PoisonError::into_inner);
    if !still_wanted() {
        return false;
    }
    let vault = session.lock();
    drop(session);
    let was_open = vault.is_some();
    drop(vault);
    clipboard.clear_if_unchanged(None);
    was_open
}

#[tauri::command]
pub fn lock_vault(state: State<'_, VaultState>, clipboard: State<'_, Clipboard>) {
    lock_session(&state, &clipboard);
    tracing::info!("vault locked");
}

/// User input in the UI (throttled there): restarts the inactivity delay.
#[tauri::command]
pub fn activity(autolock: State<'_, AutoLock>) {
    autolock.touch();
}

#[tauri::command]
pub fn database_info(state: State<'_, VaultState>) -> IpcResult<DatabaseView> {
    with_vault(&state, |v| Ok(view(v)))
}

// ---------------------------------------------------------------- reading

#[tauri::command]
pub fn tree(state: State<'_, VaultState>) -> IpcResult<GroupNode> {
    with_vault(&state, |v| Ok(v.tree()))
}

#[tauri::command]
pub fn list_entries(state: State<'_, VaultState>) -> IpcResult<Vec<EntrySummary>> {
    with_vault(&state, |v| Ok(v.list_entries()))
}

#[tauri::command]
pub fn get_entry(state: State<'_, VaultState>, id: EntryId) -> IpcResult<EntryDetail> {
    with_vault(&state, |v| Ok(v.get_entry(id)?))
}

/// The editor's copy of an entry, password included.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftView {
    pub id: Option<EntryId>,
    pub group_id: GroupId,
    pub title: String,
    pub username: String,
    pub password: String,
    pub url: String,
    pub notes: String,
}

#[tauri::command]
pub fn get_entry_for_edit(state: State<'_, VaultState>, id: EntryId) -> IpcResult<DraftView> {
    with_vault(&state, |v| {
        let d = v.entry_for_edit(id)?;
        Ok(DraftView {
            id: d.id,
            group_id: d.group_id,
            title: d.title,
            username: d.username,
            password: d.password.expose_secret().to_owned(),
            url: d.url,
            notes: d.notes,
        })
    })
}

#[tauri::command]
pub fn reveal_field(state: State<'_, VaultState>, id: EntryId, field: Field) -> IpcResult<String> {
    with_vault(&state, |v| {
        Ok(v.reveal_field(id, field)?.expose_secret().to_owned())
    })
}

/// Copies a field on the Rust side; returns the seconds until it is cleared.
#[tauri::command]
pub fn copy_field(
    state: State<'_, VaultState>,
    settings: State<'_, SettingsState>,
    clipboard: State<'_, Clipboard>,
    id: EntryId,
    field: Field,
) -> IpcResult<u32> {
    let secs = lock(&settings.current)?.clipboard_clear_secs;
    with_vault(&state, |v| {
        let value = v.reveal_field(id, field)?;
        clipboard
            .copy(value.expose_secret(), Duration::from_secs(u64::from(secs)))
            .map_err(|_| IpcError::IO)?;
        Ok(secs)
    })
}

// ---------------------------------------------------------------- changes

#[tauri::command]
pub fn save_entry(state: State<'_, VaultState>, draft: EntryDraft) -> IpcResult<EntryId> {
    with_vault(&state, |v| Ok(v.save_entry(&draft)?))
}

#[tauri::command]
pub fn delete_entry(state: State<'_, VaultState>, id: EntryId) -> IpcResult<()> {
    with_vault(&state, |v| Ok(v.delete_entry(id)?))
}

#[tauri::command]
pub fn restore_entry(state: State<'_, VaultState>, id: EntryId) -> IpcResult<()> {
    with_vault(&state, |v| Ok(v.restore_entry(id)?))
}

#[tauri::command]
pub fn create_group(
    state: State<'_, VaultState>,
    parent: GroupId,
    name: String,
) -> IpcResult<GroupId> {
    with_vault(&state, |v| Ok(v.create_group(parent, &name)?))
}

#[tauri::command]
pub fn rename_group(state: State<'_, VaultState>, id: GroupId, name: String) -> IpcResult<()> {
    with_vault(&state, |v| Ok(v.rename_group(id, &name)?))
}

#[tauri::command]
pub fn delete_group(state: State<'_, VaultState>, id: GroupId) -> IpcResult<()> {
    with_vault(&state, |v| Ok(v.delete_group(id)?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabasePatch {
    pub name: Option<String>,
    pub cipher: Option<Cipher>,
    pub recycle_bin: Option<bool>,
    pub kdf: Option<Argon2idParams>,
}

#[tauri::command]
pub fn update_database(
    state: State<'_, VaultState>,
    patch: DatabasePatch,
) -> IpcResult<DatabaseView> {
    with_vault(&state, |v| {
        if let Some(name) = &patch.name {
            v.rename_database(name)?;
        }
        if let Some(cipher) = patch.cipher {
            v.set_cipher(cipher)?;
        }
        if let Some(enabled) = patch.recycle_bin {
            v.set_recycle_bin_enabled(enabled);
        }
        if let Some(kdf) = patch.kdf {
            v.set_kdf(kdf)?;
        }
        Ok(view(v))
    })
}

#[tauri::command]
pub fn change_master_password(
    state: State<'_, VaultState>,
    password: SecretString,
) -> IpcResult<()> {
    with_vault(&state, |v| Ok(v.change_master_password(password)?))
}

// ---------------------------------------------------------------- saving

/// Saves; key derivation runs off the async runtime.
async fn save_with(app: AppHandle, overwrite: bool) -> IpcResult<DatabaseView> {
    blocking(move || {
        let state = app.state::<VaultState>();
        with_vault(&state, |v| {
            let result = if overwrite {
                v.save_overwrite()
            } else {
                v.save()
            };
            result.map_err(IpcError::from)?;
            Ok(view(v))
        })
    })
    .await
}

#[tauri::command]
pub async fn save(app: AppHandle) -> IpcResult<DatabaseView> {
    save_with(app, false).await
}

#[tauri::command]
pub async fn save_overwrite(app: AppHandle) -> IpcResult<DatabaseView> {
    save_with(app, true).await
}

#[tauri::command]
pub async fn reload(app: AppHandle) -> IpcResult<DatabaseView> {
    blocking(move || {
        let state = app.state::<VaultState>();
        with_vault(&state, |v| {
            v.reload()?;
            Ok(view(v))
        })
    })
    .await
}

/// Whether the file still holds what Keyrust last read or wrote.
#[tauri::command]
pub fn check_file(state: State<'_, VaultState>) -> IpcResult<bool> {
    with_vault(&state, |v| Ok(v.file_unchanged()?))
}

// ---------------------------------------------------------------- tools

#[tauri::command]
pub fn generate_password(options: GeneratorOptions) -> IpcResult<String> {
    generator::generate(&options)
        .map(|p| p.as_str().to_owned())
        .map_err(|_| IpcError::IO)
}

#[tauri::command]
pub fn estimate_strength(password: SecretString) -> StrengthEstimate {
    strength::estimate(password.expose_secret())
}

#[tauri::command]
pub async fn benchmark_kdf(memory_mib: u32, parallelism: u32) -> IpcResult<u32> {
    blocking(move || {
        kdf::calibrate(memory_mib, parallelism, kdf::TARGET).map_err(|_| IpcError::INVALID)
    })
    .await
}

/// Only plain web links open in the system browser.
pub fn web_url(url: &str) -> IpcResult<tauri::Url> {
    let parsed = tauri::Url::parse(url.trim()).map_err(|_| IpcError::INVALID)?;
    match parsed.scheme() {
        "http" | "https" if parsed.host_str().is_some() => Ok(parsed),
        _ => Err(IpcError::INVALID),
    }
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> IpcResult<()> {
    let url = web_url(&url)?;
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|_| IpcError::IO)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn only_http_links_open() {
        assert!(web_url("https://github.com/downlevel/keyrust").is_ok());
        assert!(web_url(" http://bank.example ").is_ok());
        for bad in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "ssh://host",
            "smb://server/share",
            "https:/",
            "not a url",
        ] {
            assert!(web_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_stale_settings_snapshot_keeps_the_recent_files() {
        let dir = tempfile::tempdir().unwrap();
        let state = SettingsState {
            path: dir.path().join("settings.json"),
            current: Mutex::new(Settings::default()),
        };
        let stale = Settings::default();
        remember_recent(&state, Path::new("/vaults/opened-meanwhile.kdbx"));
        let saved = apply_settings(
            &state,
            Settings {
                clipboard_clear_secs: 45,
                ..stale
            },
        )
        .unwrap();
        assert_eq!(saved.clipboard_clear_secs, 45);
        assert_eq!(
            saved.recent_files,
            [PathBuf::from("/vaults/opened-meanwhile.kdbx")]
        );
        assert_eq!(settings::load(&state.path), saved);
    }

    #[test]
    fn file_names_and_extensions() {
        assert_eq!(sanitize_file_name("My: Vault/2026"), "My_ Vault_2026");
        assert_eq!(sanitize_file_name(" .. "), "Passwords");
        assert_eq!(
            with_kdbx_extension(PathBuf::from("/a/Personal")),
            PathBuf::from("/a/Personal.kdbx")
        );
        assert_eq!(
            with_kdbx_extension(PathBuf::from("/a/Personal.KDBX")),
            PathBuf::from("/a/Personal.KDBX")
        );
    }

    #[test]
    fn frontend_payloads_deserialize_into_secrets() {
        let draft: EntryDraft = serde_json::from_str(
            r#"{"id":null,"groupId":"6f1c2b5a-0000-4000-8000-000000000001","title":"Bank",
                "username":"alice","password":"s3cret","url":"","notes":""}"#,
        )
        .unwrap();
        assert_eq!(draft.password.expose_secret(), "s3cret");
        assert!(!format!("{draft:?}").contains("s3cret"));

        let request: CreateRequest = serde_json::from_str(
            r#"{"path":"/a/b.kdbx","name":"B","password":"pw","cipher":"chaCha20",
                "kdf":{"memoryMib":64,"iterations":3,"parallelism":2}}"#,
        )
        .unwrap();
        assert_eq!(request.cipher, Cipher::ChaCha20);
        assert_eq!(request.kdf.memory_mib, 64);
    }

    #[test]
    fn views_serialize_without_secrets() {
        let json = serde_json::to_value(RecentFile {
            name: "Personal".into(),
            path: "/a/Personal.kdbx".into(),
            missing: true,
        })
        .unwrap();
        assert_eq!(json["missing"], true);
    }
}
