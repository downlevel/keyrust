//! The few operating-system calls Keyrust needs that the Rust standard library
//! and the other dependencies do not offer: whether the session's screen is
//! locked (Windows, macOS) and suspend notifications (Windows).
//!
//! This is the **only** crate of Keyrust where `unsafe` code is allowed. It is
//! denied crate-wide and allowed function by function on the FFI calls, each
//! with a `SAFETY:` comment (enforced by clippy). Linux needs none of it: the
//! app uses D-Bus there.

/// Whether the screen of the current session is locked. `None` when it cannot
/// be determined (Linux, or no graphical session).
pub fn screen_locked() -> Option<bool> {
    platform::screen_locked()
}

/// Calls `callback` when the system is about to suspend or has just resumed.
/// Only Windows has an implementation; elsewhere the result is `None`. The
/// callback runs on a system thread: keep it short. Dropping the guard
/// unregisters it.
pub fn on_suspend(callback: impl Fn() + Send + Sync + 'static) -> Option<SuspendGuard> {
    platform::on_suspend(Box::new(callback))
}

pub use platform::SuspendGuard;

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

    type Callback = Box<dyn Fn() + Send + Sync>;

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

    pub struct SuspendGuard {
        handle: *mut c_void,
        /// Owned here, so it outlives the registration that points to it.
        callback: *mut Callback,
        /// Read by the system during registration; kept for its lifetime.
        _params: Box<DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS>,
    }

    #[allow(unsafe_code)]
    // SAFETY: the raw pointers are only used by Drop (unregister, then free)
    // and by the system callback, which requires `Callback: Send + Sync`.
    unsafe impl Send for SuspendGuard {}
    #[allow(unsafe_code)]
    // SAFETY: see `Send`; no method exposes the pointers.
    unsafe impl Sync for SuspendGuard {}

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
    pub fn on_suspend(callback: Callback) -> Option<SuspendGuard> {
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
        Some(SuspendGuard {
            handle,
            callback,
            _params: params,
        })
    }

    #[allow(unsafe_code)]
    impl Drop for SuspendGuard {
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

    pub struct SuspendGuard;

    pub fn on_suspend(_: Box<dyn Fn() + Send + Sync>) -> Option<SuspendGuard> {
        None
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod platform {
    pub fn screen_locked() -> Option<bool> {
        None
    }

    pub struct SuspendGuard;

    pub fn on_suspend(_: Box<dyn Fn() + Send + Sync>) -> Option<SuspendGuard> {
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
}
