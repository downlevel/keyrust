//! Copying secrets to the system clipboard, entirely on the Rust side.
//!
//! The copied value is not kept: only its SHA-256. After the timeout the
//! clipboard is cleared only if it still holds that value, so whatever the user
//! copied afterwards is left alone. Lock and exit clear it the same way.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

/// The system clipboard, behind a trait so the logic is testable headless.
pub trait Board: Send {
    fn set_secret(&mut self, text: &str) -> Result<(), String>;
    /// `Ok(None)`: no text on the clipboard. `Err`: it could not be read
    /// (e.g. another program holds it on Windows) — not the same thing.
    fn get(&mut self) -> Result<Option<Zeroizing<String>>, String>;
    fn clear(&mut self) -> Result<(), String>;
}

/// Retries when the clipboard is temporarily unavailable: from the timer
/// (background thread) and on lock / exit (kept short, the UI waits).
const TIMER_RETRIES: (u32, Duration) = (5, Duration::from_millis(500));
const IMMEDIATE_RETRIES: (u32, Duration) = (3, Duration::from_millis(50));

enum Attempt {
    /// Cleared, or it holds something else now: Keyrust's job is done.
    Done,
    /// Could not read or clear: try again later.
    Failed,
}

fn digest(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

/// Whether the clipboard may be cleared: it still holds what Keyrust copied.
pub fn should_clear(current: Option<&str>, copied: &[u8; 32]) -> bool {
    current.is_some_and(|c| digest(c) == *copied)
}

struct Inner {
    board: Option<Box<dyn Board>>,
    copied: Option<[u8; 32]>,
    /// Incremented by every copy: a timer only acts for its own copy.
    generation: u64,
}

#[derive(Clone)]
pub struct Clipboard {
    inner: Arc<Mutex<Inner>>,
}

impl Clipboard {
    pub fn new(board: Option<Box<dyn Board>>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                board,
                copied: None,
                generation: 0,
            })),
        }
    }

    /// Copies `value` and clears it after `after` unless something else was
    /// copied meanwhile.
    pub fn copy(&self, value: &str, after: Duration) -> Result<(), String> {
        let generation = {
            let mut inner = self.inner.lock().map_err(|_| "clipboard unavailable")?;
            let board = inner.board.as_mut().ok_or("no clipboard")?;
            board.set_secret(value)?;
            inner.copied = Some(digest(value));
            inner.generation += 1;
            inner.generation
        };
        let this = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(after);
            this.clear_with(Some(generation), TIMER_RETRIES);
        });
        tracing::info!(
            clear_after_secs = after.as_secs(),
            "value copied to the clipboard"
        );
        Ok(())
    }

    /// Clears the clipboard now (lock, exit) if it still holds the last
    /// copied value.
    pub fn clear_if_unchanged(&self, generation: Option<u64>) {
        self.clear_with(generation, IMMEDIATE_RETRIES);
    }

    /// With a `generation`, acts only if no newer copy happened since. If the
    /// clipboard cannot be read or cleared, the hash is kept and the attempt
    /// retried; after the last retry it is still kept, so lock and exit try
    /// again.
    fn clear_with(&self, generation: Option<u64>, (retries, delay): (u32, Duration)) {
        for attempt in 0..retries {
            {
                let Ok(mut inner) = self.inner.lock() else {
                    return;
                };
                if generation.is_some_and(|g| g != inner.generation) || inner.copied.is_none() {
                    return;
                }
                if let Attempt::Done = Self::attempt(&mut inner) {
                    inner.copied = None;
                    return;
                }
            }
            if attempt + 1 < retries {
                std::thread::sleep(delay);
            }
        }
        tracing::warn!("clipboard could not be cleared; will retry on lock or exit");
    }

    fn attempt(inner: &mut Inner) -> Attempt {
        let (Some(copied), Some(board)) = (inner.copied, inner.board.as_mut()) else {
            return Attempt::Done;
        };
        match board.get() {
            Err(_) => Attempt::Failed,
            Ok(current) if should_clear(current.as_ref().map(|c| c.as_str()), &copied) => {
                if board.clear().is_ok() {
                    tracing::info!("clipboard cleared");
                    Attempt::Done
                } else {
                    Attempt::Failed
                }
            }
            // The user copied something else: leave it alone.
            Ok(_) => Attempt::Done,
        }
    }
}

/// The real clipboard (`arboard`), asking the platform to keep copied secrets
/// out of clipboard history and cloud sync where it can.
pub struct SystemBoard(arboard::Clipboard);

impl SystemBoard {
    pub fn new() -> Option<Self> {
        arboard::Clipboard::new().ok().map(Self)
    }
}

impl Board for SystemBoard {
    fn set_secret(&mut self, text: &str) -> Result<(), String> {
        #[allow(unused_mut)]
        let mut set = self.0.set();
        #[cfg(target_os = "linux")]
        {
            use arboard::SetExtLinux;
            set = set.exclude_from_history();
        }
        #[cfg(target_os = "windows")]
        {
            use arboard::SetExtWindows;
            set = set.exclude_from_history();
        }
        #[cfg(target_os = "macos")]
        {
            use arboard::SetExtApple;
            set = set.exclude_from_history();
        }
        set.text(text).map_err(|e| e.to_string())
    }

    fn get(&mut self) -> Result<Option<Zeroizing<String>>, String> {
        match self.0.get_text() {
            Ok(text) => Ok(Some(Zeroizing::new(text))),
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn clear(&mut self) -> Result<(), String> {
        self.0.clear().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU32, Ordering};

    /// A clipboard whose next `get` / `clear` calls can be made to fail.
    #[derive(Clone, Default)]
    struct FakeBoard(Arc<Mutex<Option<String>>>, Arc<AtomicU32>, Arc<AtomicU32>);

    impl FakeBoard {
        fn fail_reads(&self, n: u32) {
            self.1.store(n, Ordering::SeqCst);
        }
        fn fail_clears(&self, n: u32) {
            self.2.store(n, Ordering::SeqCst);
        }
    }

    fn take_failure(counter: &AtomicU32) -> bool {
        counter
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
            .is_ok()
    }

    impl Board for FakeBoard {
        fn set_secret(&mut self, text: &str) -> Result<(), String> {
            *self.0.lock().unwrap() = Some(text.to_owned());
            Ok(())
        }
        fn get(&mut self) -> Result<Option<Zeroizing<String>>, String> {
            if take_failure(&self.1) {
                return Err("busy".into());
            }
            Ok(self.0.lock().unwrap().clone().map(Zeroizing::new))
        }
        fn clear(&mut self) -> Result<(), String> {
            if take_failure(&self.2) {
                return Err("busy".into());
            }
            *self.0.lock().unwrap() = None;
            Ok(())
        }
    }

    fn setup() -> (Clipboard, FakeBoard) {
        let board = FakeBoard::default();
        (Clipboard::new(Some(Box::new(board.clone()))), board)
    }

    fn content(board: &FakeBoard) -> Option<String> {
        board.0.lock().unwrap().clone()
    }

    const SHORT: Duration = Duration::from_millis(50);
    const WAIT: Duration = Duration::from_millis(300);

    #[test]
    fn clears_after_the_timeout_if_unchanged() {
        let (clipboard, board) = setup();
        clipboard.copy("s3cret", SHORT).unwrap();
        assert_eq!(content(&board).as_deref(), Some("s3cret"));
        std::thread::sleep(WAIT);
        assert_eq!(content(&board), None);
    }

    #[test]
    fn leaves_what_the_user_copied_afterwards() {
        let (clipboard, board) = setup();
        clipboard.copy("s3cret", SHORT).unwrap();
        *board.0.lock().unwrap() = Some("user text".into());
        std::thread::sleep(WAIT);
        assert_eq!(content(&board).as_deref(), Some("user text"));
    }

    #[test]
    fn a_new_copy_replaces_the_previous_timer() {
        let (clipboard, board) = setup();
        clipboard.copy("first", SHORT).unwrap();
        clipboard.copy("second", Duration::from_secs(30)).unwrap();
        std::thread::sleep(WAIT);
        // The first timer must not clear the second value.
        assert_eq!(content(&board).as_deref(), Some("second"));
    }

    #[test]
    fn lock_clears_immediately_only_our_value() {
        let (clipboard, board) = setup();
        clipboard.copy("s3cret", Duration::from_secs(30)).unwrap();
        clipboard.clear_if_unchanged(None);
        assert_eq!(content(&board), None);

        clipboard.copy("s3cret", Duration::from_secs(30)).unwrap();
        *board.0.lock().unwrap() = Some("user text".into());
        clipboard.clear_if_unchanged(None);
        assert_eq!(content(&board).as_deref(), Some("user text"));
    }

    #[test]
    fn should_clear_compares_content() {
        let copied = digest("abc");
        assert!(should_clear(Some("abc"), &copied));
        assert!(!should_clear(Some("abd"), &copied));
        assert!(!should_clear(None, &copied));
    }

    #[test]
    fn without_a_clipboard_copy_fails_cleanly() {
        let clipboard = Clipboard::new(None);
        assert!(clipboard.copy("x", SHORT).is_err());
        clipboard.clear_if_unchanged(None);
    }

    #[test]
    fn a_temporarily_unavailable_clipboard_is_retried() {
        let (clipboard, board) = setup();
        clipboard.copy("s3cret", Duration::from_secs(30)).unwrap();
        board.fail_reads(2);
        board.fail_clears(0);
        clipboard.clear_if_unchanged(None);
        assert_eq!(content(&board), None, "cleared on the third attempt");
    }

    #[test]
    fn after_failed_retries_lock_or_exit_still_clear_it() {
        let (clipboard, board) = setup();
        clipboard.copy("s3cret", Duration::from_secs(30)).unwrap();
        board.fail_clears(10);
        clipboard.clear_if_unchanged(None);
        assert_eq!(content(&board).as_deref(), Some("s3cret"));
        board.fail_clears(0);
        // The hash was kept: the next lock / exit clears the password.
        clipboard.clear_if_unchanged(None);
        assert_eq!(content(&board), None);
    }
}
