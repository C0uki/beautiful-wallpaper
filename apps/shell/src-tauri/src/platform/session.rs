//! Keeping the machine awake.
//!
//! The original inhibits idle through the compositor's idle-inhibit protocol.
//! Windows has `SetThreadExecutionState`, which is simpler in every way but
//! one: the request belongs to the *thread* that made it, and lasts only as
//! long as that thread lives. Setting it from a command handler would last
//! until that handler returned, which is to say no time at all.
//!
//! So the inhibitor owns a thread whose entire job is to exist. It sets the
//! flag, parks, and clears the flag on the way out.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use windows::Win32::System::Power::{
    SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
};

/// Holds the display awake for as long as it is switched on.
pub struct IdleInhibitor {
    wanted: Arc<(Mutex<bool>, Condvar)>,
    running: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Default for IdleInhibitor {
    fn default() -> Self {
        Self::new()
    }
}

impl IdleInhibitor {
    pub fn new() -> Self {
        let wanted = Arc::new((Mutex::new(false), Condvar::new()));
        let running = Arc::new(AtomicBool::new(true));

        let thread = {
            let wanted = wanted.clone();
            let running = running.clone();
            std::thread::Builder::new()
                .name("bw-idle-inhibit".to_owned())
                .spawn(move || hold(&wanted, &running))
                .ok()
        };

        Self {
            wanted,
            running,
            thread,
        }
    }

    pub fn is_on(&self) -> bool {
        *self
            .wanted
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn set(&self, on: bool) {
        let (lock, notify) = &*self.wanted;
        let mut wanted = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *wanted = on;
        drop(wanted);
        notify.notify_all();
    }
}

impl Drop for IdleInhibitor {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.wanted.1.notify_all();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The inhibiting thread: it exists so the request has somewhere to live.
fn hold(wanted: &(Mutex<bool>, Condvar), running: &AtomicBool) {
    let (lock, notify) = wanted;
    let mut held = false;
    let mut guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    while running.load(Ordering::Relaxed) {
        if *guard != held {
            held = *guard;
            apply(held);
        }
        // Waiting rather than spinning: the flag belongs to this thread, so it
        // has to stay alive, but it has nothing else to do.
        guard = notify
            .wait(guard)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
    }
    drop(guard);

    // Never leave a standing request behind: the machine would refuse to sleep
    // for the rest of the session, with nothing left to explain why.
    if held {
        apply(false);
    }
}

fn apply(held: bool) {
    unsafe {
        // `ES_CONTINUOUS` alone clears a standing request; combined with the
        // others it establishes one that lasts until changed.
        let flags = if held {
            ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED
        } else {
            ES_CONTINUOUS
        };
        SetThreadExecutionState(flags);
    }
}
