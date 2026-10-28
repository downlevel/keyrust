//! Auto-lock, decided and done in the Rust process: the vault is dropped here
//! even if the webview hangs or misbehaves. The UI only reports activity and
//! is told afterwards, with the `locked` event.
//!
//! Every trigger (inactivity, screen lock, suspend, a clock jump, minimize)
//! sends a [`Reason`] to one locker thread. Triggers never wait on the vault
//! mutex, which a save holds while deriving the key: the Windows power
//! callback must return quickly, and a blocked clock-jump detector would
//! mistake a slow save for a suspend.

mod policy;

use std::sync::Mutex;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

pub use policy::{Reason, Session};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::clipboard::Clipboard;
use crate::commands::{SettingsState, VaultState, lock_session_if};

/// Event sent to the UI after an automatic lock.
pub const LOCKED_EVENT: &str = "locked";

const IDLE_CHECK: Duration = Duration::from_secs(5);
const SCREEN_POLL: Duration = Duration::from_secs(2);
const CLOCK_TICK: Duration = Duration::from_secs(1);
/// A gap this long between clock ticks means the machine was asleep.
const SUSPEND_GAP: Duration = Duration::from_secs(4);

#[derive(Clone, Serialize)]
struct Locked {
    reason: Reason,
    /// The generation the lock started. The event can reach the UI after a
    /// newer unlock (whose `session` is at least this): the UI ignores it then.
    session: u64,
}

/// Last user activity and the way to the locker thread.
pub struct AutoLock {
    last_activity: Mutex<Instant>,
    trigger: Sender<Reason>,
    /// Keeps the OS notifications registered for the life of the app.
    _guards: Vec<keyrust_os::Guard>,
}

impl AutoLock {
    /// Restarts the inactivity delay (input in the UI, a vault just opened).
    pub fn touch(&self) {
        if let Ok(mut last) = self.last_activity.lock() {
            *last = Instant::now();
        }
    }

    pub fn trigger(&self, reason: Reason) {
        let _ = self.trigger.send(reason);
    }

    fn last_activity(&self) -> Instant {
        self.last_activity
            .lock()
            .map(|l| *l)
            .unwrap_or_else(|_| Instant::now())
    }
}

/// Starts the locker, the watchers and the OS notifications, and manages the
/// [`AutoLock`] state. Called once, from `setup`.
pub fn start(app: &AppHandle) {
    let (trigger, reasons) = mpsc::channel();
    let mut guards = Vec::new();

    let sender = trigger.clone();
    match keyrust_os::on_suspend(move || {
        let _ = sender.send(Reason::Suspend);
    }) {
        Some(guard) => guards.push(guard),
        None if cfg!(target_os = "macos") => {}
        None => tracing::warn!("no suspend notification: relying on the clock-jump detector"),
    }

    let polled = keyrust_os::screen_locked().is_some();
    if !polled {
        let sender = trigger.clone();
        match keyrust_os::on_screen_lock(move || {
            let _ = sender.send(Reason::ScreenLock);
        }) {
            Some(guard) => guards.push(guard),
            None => tracing::warn!("screen lock cannot be detected in this session"),
        }
    }

    app.manage(AutoLock {
        last_activity: Mutex::new(Instant::now()),
        trigger: trigger.clone(),
        _guards: guards,
    });

    spawn("keyrust-locker", {
        let app = app.clone();
        move || {
            for reason in reasons {
                locker(&app, reason);
            }
        }
    });
    spawn("keyrust-idle", {
        let app = app.clone();
        let trigger = trigger.clone();
        move || idle_watch(&app, &trigger)
    });
    if polled {
        let trigger = trigger.clone();
        spawn("keyrust-screen", move || screen_watch(&trigger));
    }
    spawn("keyrust-clock", move || clock_watch(&trigger));
}

fn spawn(name: &str, f: impl FnOnce() + Send + 'static) {
    if let Err(e) = thread::Builder::new().name(name.into()).spawn(f) {
        tracing::error!(thread = name, "cannot start auto-lock thread: {e}");
    }
}

fn locker(app: &AppHandle, reason: Reason) {
    let settings = app.state::<SettingsState>();
    let (wanted, minutes) = settings
        .current
        .lock()
        .map(|s| (policy::should_lock(reason, &s), s.idle_lock_mins))
        // Unreadable settings: lock, the safe side.
        .unwrap_or((true, 1));
    if !wanted {
        return;
    }
    // An idle request may be stale (queued while a save held the session, or
    // before another unlock): it acts only if the delay is still over.
    let autolock = app.state::<AutoLock>();
    let still_wanted = || {
        reason != Reason::Idle
            || policy::idle_expired(autolock.last_activity(), Instant::now(), minutes)
    };
    let locked = lock_session_if(
        &app.state::<VaultState>(),
        &app.state::<Clipboard>(),
        still_wanted,
    );
    if let Some(session) = locked {
        tracing::info!(?reason, "vault locked automatically");
        let _ = app.emit(LOCKED_EVENT, Locked { reason, session });
    }
}

fn idle_watch(app: &AppHandle, trigger: &Sender<Reason>) {
    loop {
        thread::sleep(IDLE_CHECK);
        let minutes = app
            .state::<SettingsState>()
            .current
            .lock()
            .map(|s| s.idle_lock_mins)
            .unwrap_or(1);
        let last = app.state::<AutoLock>().last_activity();
        if policy::idle_expired(last, Instant::now(), minutes)
            && app.state::<VaultState>().is_open()
            && trigger.send(Reason::Idle).is_err()
        {
            return;
        }
    }
}

/// Windows and macOS: polls the session state; acts when it becomes locked.
fn screen_watch(trigger: &Sender<Reason>) {
    let mut was_locked = false;
    loop {
        thread::sleep(SCREEN_POLL);
        let locked = keyrust_os::screen_locked() == Some(true);
        if locked && !was_locked && trigger.send(Reason::ScreenLock).is_err() {
            return;
        }
        was_locked = locked;
    }
}

/// Everywhere, and the only suspend detection on macOS: the thread does not
/// run while the machine sleeps, so the wall clock jumps between two ticks.
fn clock_watch(trigger: &Sender<Reason>) {
    let mut prev = SystemTime::now();
    loop {
        thread::sleep(CLOCK_TICK);
        let now = SystemTime::now();
        if policy::suspended(prev, now, SUSPEND_GAP) && trigger.send(Reason::Suspend).is_err() {
            return;
        }
        prev = now;
    }
}
