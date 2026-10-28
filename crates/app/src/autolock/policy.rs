//! The auto-lock decisions, free of Tauri and of the OS: what locks, when the
//! idle time is up, what a clock jump means, and which unlock may install its
//! vault.

use std::time::{Duration, Instant, SystemTime};

use keyrust_core::settings::Settings;
use serde::Serialize;

/// Why the vault was locked automatically, sent to the UI with the `locked`
/// event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Reason {
    Idle,
    ScreenLock,
    Suspend,
    Minimize,
}

/// Whether `reason` locks with these settings. Inactivity and suspend always
/// do (the idle delay has no "never"); screen lock and minimize are options.
pub fn should_lock(reason: Reason, settings: &Settings) -> bool {
    match reason {
        Reason::Idle | Reason::Suspend => true,
        Reason::ScreenLock => settings.lock_on_screen_lock,
        Reason::Minimize => settings.lock_on_minimize,
    }
}

/// Whether `minutes` of inactivity have passed since `last` (monotonic time).
pub fn idle_expired(last: Instant, now: Instant, minutes: u32) -> bool {
    now.saturating_duration_since(last) >= Duration::from_secs(u64::from(minutes) * 60)
}

/// Whether the wall clock moved forward by more than `threshold` between two
/// ticks of a thread that sleeps much less than that: the machine was asleep
/// (the thread does not run meanwhile). A clock set backwards is not a
/// suspend; one set forwards locks needlessly, the safe side.
pub fn suspended(prev: SystemTime, now: SystemTime, threshold: Duration) -> bool {
    now.duration_since(prev).is_ok_and(|gap| gap > threshold)
}

/// The open database, if any (`None` is "locked"), and a counter that every
/// lock increments. An unlock reads the counter before deriving the key and
/// installs its vault only if no lock happened meanwhile, so a screen lock
/// during a slow unlock is not undone when the unlock finishes.
pub struct Session<V> {
    vault: Option<V>,
    generation: u64,
}

impl<V> Default for Session<V> {
    fn default() -> Self {
        Self {
            vault: None,
            generation: 0,
        }
    }
}

impl<V> Session<V> {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn is_open(&self) -> bool {
        self.vault.is_some()
    }

    pub fn vault_mut(&mut self) -> Option<&mut V> {
        self.vault.as_mut()
    }

    /// Installs `vault` if no lock happened since `generation` was read;
    /// otherwise hands it back, to be dropped (zeroized).
    pub fn install(&mut self, vault: V, generation: u64) -> Result<(), V> {
        if generation != self.generation {
            return Err(vault);
        }
        self.vault = Some(vault);
        Ok(())
    }

    /// Takes the vault out (for the caller to drop) and invalidates every
    /// unlock in progress, also when no vault is open.
    pub fn lock(&mut self) -> Option<V> {
        self.generation = self.generation.wrapping_add(1);
        self.vault.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unlock_that_started_before_a_lock_is_discarded() {
        let mut session = Session::default();
        let before = session.generation();
        assert_eq!(session.lock(), None, "nothing open yet");
        assert_eq!(session.install(7, before), Err(7));
        assert!(!session.is_open());

        let now = session.generation();
        assert_eq!(session.install(8, now), Ok(()));
        assert_eq!(session.vault_mut(), Some(&mut 8));
        assert_eq!(session.lock(), Some(8));
        assert!(!session.is_open());
    }

    #[test]
    fn idle_time() {
        let start = Instant::now();
        let at = |secs| start + Duration::from_secs(secs);
        assert!(!idle_expired(start, at(299), 5));
        assert!(idle_expired(start, at(300), 5));
        assert!(idle_expired(start, at(10_000), 5));
        // Activity recorded after `now` (another thread): not expired.
        assert!(!idle_expired(at(10), start, 1));
    }

    #[test]
    fn clock_jumps() {
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let threshold = Duration::from_secs(4);
        assert!(!suspended(t, t + Duration::from_secs(1), threshold));
        assert!(!suspended(t, t + threshold, threshold));
        assert!(suspended(t, t + Duration::from_millis(4_001), threshold));
        assert!(suspended(t, t + Duration::from_secs(3_600), threshold));
        assert!(!suspended(t, t - Duration::from_secs(3_600), threshold));
    }

    #[test]
    fn which_reasons_lock() {
        let mut settings = Settings {
            lock_on_screen_lock: false,
            lock_on_minimize: false,
            ..Settings::default()
        };
        assert!(should_lock(Reason::Idle, &settings));
        assert!(should_lock(Reason::Suspend, &settings));
        assert!(!should_lock(Reason::ScreenLock, &settings));
        assert!(!should_lock(Reason::Minimize, &settings));
        settings.lock_on_screen_lock = true;
        settings.lock_on_minimize = true;
        assert!(should_lock(Reason::ScreenLock, &settings));
        assert!(should_lock(Reason::Minimize, &settings));
    }

    #[test]
    fn reasons_serialize_for_the_ui() {
        let names: Vec<_> = [
            Reason::Idle,
            Reason::ScreenLock,
            Reason::Suspend,
            Reason::Minimize,
        ]
        .iter()
        .map(|r| serde_json::to_value(r).unwrap())
        .collect();
        assert_eq!(
            names,
            ["idle", "screenLock", "suspend", "minimize"].map(serde_json::Value::from)
        );
    }
}
