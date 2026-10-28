//! The operating-system integration Keyrust needs that the Rust standard
//! library and the other dependencies do not offer: whether the session's
//! screen is locked (Windows, macOS, polled), screen-lock signals (Linux) and
//! suspend notifications (Windows, Linux).
//!
//! This is the **only** crate of Keyrust where `unsafe` code is allowed. It is
//! denied crate-wide and allowed function by function on the FFI calls of
//! Windows and macOS, each with a `SAFETY:` comment (enforced by clippy).
//! Linux needs none: it uses D-Bus through `zbus`.

/// Whether the screen of the current session is locked. `None` when it cannot
/// be determined (Linux, or no graphical session): there, use
/// [`on_screen_lock`].
pub fn screen_locked() -> Option<bool> {
    platform::screen_locked()
}

/// Calls `callback` when the system is about to suspend (Windows, Linux via
/// logind) or has just resumed (Windows). `None` when there is no such
/// notification (macOS) or it could not be registered (e.g. no system bus).
/// The callback runs on a system or dedicated thread: keep it short. Dropping
/// the guard stops the calls.
pub fn on_suspend(callback: impl Fn() + Send + Sync + 'static) -> Option<Guard> {
    platform::on_suspend(Box::new(callback))
}

/// Calls `callback` when the screen gets locked (Linux: the `ActiveChanged`
/// signal of the session's screen saver). `None` elsewhere, where
/// [`screen_locked`] is polled instead, or without a session bus.
pub fn on_screen_lock(callback: impl Fn() + Send + Sync + 'static) -> Option<Guard> {
    platform::on_screen_lock(Box::new(callback))
}

pub use platform::Guard;

type Callback = Box<dyn Fn() + Send + Sync>;

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;

    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Power::{
        DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS, PowerRegisterSuspendResumeNotification,
        PowerUnregisterSuspendResumeNotification,
    };
    use windows_sys::Win32::System::StationsAndDesktops::{
        CloseDesktop, DESKTOP_SWITCHDESKTOP, OpenInputDesktop,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DEVICE_NOTIFY_CALLBACK, PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND, PBT_APMSUSPEND,
    };

    use crate::Callback;

    /// The input desktop cannot be opened while the session is locked (or on
    /// the secure desktop of a UAC prompt, which then also counts as locked:
    /// the safe side of the error).
    #[allow(unsafe_code)]
    pub fn screen_locked() -> Option<bool> {
        // SAFETY: plain Win32 call with constant arguments; it returns a
        // desktop handle or null and has no other effect.
        let desktop = unsafe { OpenInputDesktop(0, 0, DESKTOP_SWITCHDESKTOP) };
        if desktop.is_null() {
            return Some(true);
        }
        // SAFETY: `desktop` is the non-null handle just returned by
        // OpenInputDesktop and is closed exactly once.
        unsafe { CloseDesktop(desktop) };
        Some(false)
    }

    pub struct Guard {
        handle: *mut c_void,
        /// Owned here, so it outlives the registration that points to it.
        callback: *mut Callback,
        /// Read by the system during registration; kept for its lifetime.
        _params: Box<DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS>,
    }

    #[allow(unsafe_code)]
    // SAFETY: the raw pointers are only used by Drop (unregister, then free)
    // and by the system callback, which requires `Callback: Send + Sync`.
    unsafe impl Send for Guard {}
    #[allow(unsafe_code)]
    // SAFETY: see `Send`; no method exposes the pointers.
    unsafe impl Sync for Guard {}

    #[allow(unsafe_code)]
    unsafe extern "system" fn notify(context: *const c_void, kind: u32, _: *const c_void) -> u32 {
        if matches!(
            kind,
            PBT_APMSUSPEND | PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND
        ) {
            // SAFETY: `context` is the `*mut Callback` registered in
            // `on_suspend`; it stays valid until the guard has unregistered.
            let callback = unsafe { &*(context as *const Callback) };
            callback();
        }
        0
    }

    #[allow(unsafe_code)]
    pub fn on_suspend(callback: Callback) -> Option<Guard> {
        let callback = Box::into_raw(Box::new(callback));
        let params = Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
            Callback: Some(notify),
            Context: callback.cast(),
        });
        let mut handle: *mut c_void = std::ptr::null_mut();
        // SAFETY: `params` points to a valid DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS
        // kept alive by the guard; `handle` is a valid out-pointer.
        let status = unsafe {
            PowerRegisterSuspendResumeNotification(
                DEVICE_NOTIFY_CALLBACK,
                (&*params as *const DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS)
                    .cast_mut()
                    .cast(),
                &mut handle,
            )
        };
        if status != ERROR_SUCCESS {
            // SAFETY: never registered: the box from into_raw is reclaimed once.
            drop(unsafe { Box::from_raw(callback) });
            return None;
        }
        Some(Guard {
            handle,
            callback,
            _params: params,
        })
    }

    pub fn on_screen_lock(_: Callback) -> Option<Guard> {
        None
    }

    #[allow(unsafe_code)]
    impl Drop for Guard {
        fn drop(&mut self) {
            // SAFETY: `handle` came from a successful registration and is
            // unregistered once; after that the system no longer calls back,
            // so the callback box can be freed.
            unsafe {
                // windows-sys types the registered handle as a pointer and
                // HPOWERNOTIFY as isize: same value.
                PowerUnregisterSuspendResumeNotification(self.handle as isize);
                drop(Box::from_raw(self.callback));
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::CFString;

    #[allow(unsafe_code)]
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGSessionCopyCurrentDictionary() -> CFDictionaryRef;
    }

    /// `CGSSessionScreenIsLocked` is present and true while the screen is
    /// locked. No dictionary: no graphical session (e.g. over SSH).
    #[allow(unsafe_code)]
    pub fn screen_locked() -> Option<bool> {
        // SAFETY: no arguments; returns a new (+1) dictionary or null.
        let raw = unsafe { CGSessionCopyCurrentDictionary() };
        if raw.is_null() {
            return None;
        }
        // SAFETY: `raw` is a non-null CFDictionary returned by a Copy function
        // (create rule): wrapping it transfers that reference, released on drop.
        let dict: CFDictionary<CFString, CFType> = unsafe { TCFType::wrap_under_create_rule(raw) };
        let locked = dict
            .find(CFString::from_static_string("CGSSessionScreenIsLocked"))
            .and_then(|v| v.downcast::<CFBoolean>())
            .is_some_and(bool::from);
        Some(locked)
    }

    pub struct Guard;

    pub fn on_suspend(_: crate::Callback) -> Option<Guard> {
        None
    }

    pub fn on_screen_lock(_: crate::Callback) -> Option<Guard> {
        None
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use zbus::blocking::{Connection, MessageIterator};
    use zbus::message::Type;
    use zbus::{MatchRule, Message};

    use crate::Callback;

    pub fn screen_locked() -> Option<bool> {
        None
    }

    /// Stops the listening thread's calls. The thread itself ends with the
    /// process (it waits on the bus), which is the lifetime of these guards.
    pub struct Guard(Arc<AtomicBool>);

    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    /// logind's `PrepareForSleep(true)` comes before the system suspends.
    pub fn on_suspend(callback: Callback) -> Option<Guard> {
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.login1.Manager")
            .ok()?
            .member("PrepareForSleep")
            .ok()?
            .build();
        watch(Connection::system().ok()?, rule, |_| true, callback)
    }

    /// `ActiveChanged(true)` of `org.freedesktop.ScreenSaver` (KDE and
    /// others) or a desktop's own `*.ScreenSaver` (GNOME, MATE, Cinnamon).
    pub fn on_screen_lock(callback: Callback) -> Option<Guard> {
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .member("ActiveChanged")
            .ok()?
            .build();
        let screen_saver = |m: &Message| {
            m.header()
                .interface()
                .is_some_and(|i| i.as_str().ends_with(".ScreenSaver"))
        };
        watch(Connection::session().ok()?, rule, screen_saver, callback)
    }

    /// The signal's single boolean argument; anything else is ignored.
    fn is_true(message: &Message) -> bool {
        message.body().deserialize::<bool>().unwrap_or(false)
    }

    /// Subscribes now (so failures give `None`) and calls `callback` from a
    /// dedicated thread for each matching signal whose boolean is true.
    fn watch(
        connection: Connection,
        rule: MatchRule<'static>,
        accept: impl Fn(&Message) -> bool + Send + 'static,
        callback: Callback,
    ) -> Option<Guard> {
        let messages = MessageIterator::for_match_rule(rule, &connection, Some(16)).ok()?;
        let stopped = Arc::new(AtomicBool::new(false));
        let flag = stopped.clone();
        std::thread::Builder::new()
            .name("keyrust-dbus".into())
            .spawn(move || {
                let _connection = connection;
                for message in messages.flatten() {
                    if flag.load(Ordering::SeqCst) {
                        break;
                    }
                    if accept(&message) && is_true(&message) {
                        callback();
                    }
                }
            })
            .ok()?;
        Some(Guard(stopped))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn signal<B>(body: &B) -> Message
        where
            B: serde::Serialize + zbus::zvariant::DynamicType,
        {
            Message::signal(
                "/org/gnome/ScreenSaver",
                "org.gnome.ScreenSaver",
                "ActiveChanged",
            )
            .unwrap()
            .build(body)
            .unwrap()
        }

        #[test]
        fn only_a_true_boolean_signal_fires() {
            assert!(is_true(&signal(&(true,))));
            assert!(!is_true(&signal(&(false,))));
            assert!(!is_true(&signal(&("yes",))));
            assert!(!is_true(&signal(&())));
        }
    }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
mod platform {
    pub fn screen_locked() -> Option<bool> {
        None
    }

    pub struct Guard;

    pub fn on_suspend(_: crate::Callback) -> Option<Guard> {
        None
    }

    pub fn on_screen_lock(_: crate::Callback) -> Option<Guard> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unlocked_test_session_is_not_reported_locked() {
        // CI runners have no locked screen: never Some(true).
        assert_ne!(screen_locked(), Some(true));
    }

    #[test]
    fn suspend_registration_and_release() {
        // The guard is dropped (unregistered) at the end of the test.
        let guard = on_suspend(|| {});
        if cfg!(windows) {
            assert!(guard.is_some(), "registration succeeds on Windows");
        }
    }

    #[test]
    fn screen_lock_subscription_does_not_panic() {
        // Some on a Linux desktop session, None on headless runners.
        let _guard = on_screen_lock(|| {});
    }
}
