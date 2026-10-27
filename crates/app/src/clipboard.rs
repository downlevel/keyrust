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
    fn get(&mut self) -> Option<Zeroizing<String>>;
    fn clear(&mut self) -> Result<(), String>;
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
            this.clear_if_unchanged(Some(generation));
        });
        tracing::info!(
            clear_after_secs = after.as_secs(),
            "value copied to the clipboard"
        );
        Ok(())
    }

    /// Clears the clipboard if it still holds the last copied value. With a
    /// `generation`, only if no newer copy happened since.
    pub fn clear_if_unchanged(&self, generation: Option<u64>) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        if generation.is_some_and(|g| g != inner.generation) {
            return;
        }
        let Some(copied) = inner.copied.take() else {
            return;
        };
        if let Some(board) = inner.board.as_mut() {
            let current = board.get();
            if should_clear(current.as_ref().map(|c| c.as_str()), &copied) {
                let cleared = board.clear().is_ok();
                tracing::info!(cleared, "clipboard cleared");
            }
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

    fn get(&mut self) -> Option<Zeroizing<String>> {
        self.0.get_text().ok().map(Zeroizing::new)
    }

    fn clear(&mut self) -> Result<(), String> {
        self.0.clear().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct FakeBoard(Arc<Mutex<Option<String>>>);

    impl Board for FakeBoard {
        fn set_secret(&mut self, text: &str) -> Result<(), String> {
            *self.0.lock().unwrap() = Some(text.to_owned());
            Ok(())
        }
        fn get(&mut self) -> Option<Zeroizing<String>> {
            self.0.lock().unwrap().clone().map(Zeroizing::new)
        }
        fn clear(&mut self) -> Result<(), String> {
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
}
