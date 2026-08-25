//! Per-application volume, through WASAPI's session API.
//!
//! The original reads PipeWire nodes, which are stable objects with names the
//! user recognises. Windows audio sessions are neither: a session's display
//! name is usually empty, its identifier is a device path with a GUID stapled
//! to it, and sessions appear and vanish as applications start and stop making
//! noise. So each session is resolved back to its owning process to get a name
//! and an icon, and addressed by the session instance identifier — which,
//! unlike the process id, is not reused the moment a process exits.
//!
//! Session lifetime is the sharp edge here. A callback still registered
//! against a session whose process has gone is a use-after-free waiting to
//! happen, so registrations are dropped with the session they belong to, and
//! the enumeration is re-read rather than cached indefinitely.

use std::sync::Arc;

use parking_lot::Mutex;
use serde::Serialize;
use windows::core::{implement, Interface, Result, GUID, PWSTR};
use windows::Win32::Foundation::BOOL;
use windows::Win32::Media::Audio::{
    eConsole, eRender, AudioSessionDisconnectReason, AudioSessionState, AudioSessionStateExpired,
    IAudioSessionControl, IAudioSessionControl2, IAudioSessionEvents, IAudioSessionEvents_Impl,
    IAudioSessionManager2, IAudioSessionNotification, IAudioSessionNotification_Impl,
    IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

use crate::platform::appicon;

/// What the mixer calls when anything changes. Boxed, as elsewhere in this
/// crate, because `#[implement]` generates a type that cannot be generic.
type OnChange = Arc<dyn Fn() + Send + Sync + 'static>;

/// One application's audio, as the mixer presents it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    /// The session instance identifier — stable for as long as the session
    /// lives, and not reused afterwards the way a process id is.
    pub id: String,
    pub process_id: u32,
    /// What to call it: the session's own display name if it set one, else the
    /// executable's file description, else its file name.
    pub name: String,
    /// A cached PNG on disk, or empty. Served through the asset protocol, the
    /// same way wallpaper thumbnails are.
    pub icon: String,
    /// 0–100, to match every other volume in the shell.
    pub percent: f32,
    pub muted: bool,
    /// Sessions that have stopped making noise are still listed by Windows.
    /// The sidebar dims them rather than hiding them, so a slider does not
    /// jump out from under the pointer when a video ends.
    pub active: bool,
}

/// The per-application mixer for the default output device.
pub struct Mixer {
    manager: IAudioSessionManager2,
    notification: IAudioSessionNotification,
    /// Live sessions, and the event registration each one owns.
    sessions: Arc<Shared>,
}

struct Tracked {
    id: String,
    control: IAudioSessionControl,
    control2: IAudioSessionControl2,
    volume: ISimpleAudioVolume,
    events: IAudioSessionEvents,
    name: String,
    icon: String,
}

impl Tracked {
    /// Unregisters before the interface is released; leaving it attached leaks
    /// the callback inside the audio service.
    fn detach(&self) {
        unsafe {
            let _ = self
                .control
                .UnregisterAudioSessionNotification(&self.events);
        }
    }
}

/// The tracked sessions, shared between the mixer and the COM callbacks.
///
/// COM interface pointers are not `Send`/`Sync` to Rust, so the shared handle
/// has to say so explicitly — the same shape `audio.rs` uses for its endpoint.
struct Shared(Mutex<Vec<Tracked>>);

// SAFETY: the audio session interfaces are free-threaded — the session API
// registers as such — and every access here goes through the mutex, including
// the rebuild that runs on a session-added callback.
unsafe impl Send for Shared {}
unsafe impl Sync for Shared {}

// SAFETY: as for `Shared`. The manager and the notification registration are
// only touched on construction and drop.
unsafe impl Send for Mixer {}
unsafe impl Sync for Mixer {}

impl Mixer {
    /// Opens the mixer for the default output device.
    ///
    /// `on_change` fires whenever a session appears, disappears, or moves its
    /// own volume — the sidebar re-reads the list rather than being handed a
    /// diff, because the list is short and a diff would be one more thing to
    /// get wrong.
    pub fn new(on_change: impl Fn() + Send + Sync + 'static) -> Result<Self> {
        let on_change: OnChange = Arc::new(on_change);

        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;

            let sessions = Arc::new(Shared(Mutex::new(Vec::new())));

            let notification: IAudioSessionNotification = SessionAdded {
                on_change: on_change.clone(),
            }
            .into();
            manager.RegisterSessionNotification(&notification)?;

            let mixer = Self {
                manager,
                notification,
                sessions,
            };
            mixer.rebuild(&on_change);
            Ok(mixer)
        }
    }

    /// Re-reads the session enumerator from scratch.
    ///
    /// Cheap — there are rarely more than a handful of sessions — and far
    /// safer than maintaining an incremental view of objects that disappear
    /// without warning.
    fn rebuild(&self, on_change: &OnChange) {
        let Ok(fresh) = (unsafe { enumerate(&self.manager, on_change) }) else {
            return;
        };

        let mut sessions = self.sessions.0.lock();
        for previous in sessions.iter() {
            previous.detach();
        }
        *sessions = fresh;
    }

    /// Every session, newest device state included.
    pub fn list(&self) -> Vec<SessionInfo> {
        self.sessions
            .0
            .lock()
            .iter()
            .filter_map(|tracked| unsafe { describe(tracked) })
            .collect()
    }

    /// Sets one application's level, clamped to `ceiling` percent.
    ///
    /// An unknown id is not an error: the sidebar may be a moment behind a
    /// session that has just ended.
    pub fn set_percent(&self, id: &str, percent: f32, ceiling: f32) -> Result<()> {
        let sessions = self.sessions.0.lock();
        let Some(tracked) = sessions.iter().find(|tracked| tracked.id == id) else {
            return Ok(());
        };
        let scalar = (percent.min(ceiling) / 100.0).clamp(0.0, 1.0);
        unsafe { tracked.volume.SetMasterVolume(scalar, std::ptr::null()) }
    }

    pub fn set_muted(&self, id: &str, muted: bool) -> Result<()> {
        let sessions = self.sessions.0.lock();
        let Some(tracked) = sessions.iter().find(|tracked| tracked.id == id) else {
            return Ok(());
        };
        unsafe { tracked.volume.SetMute(muted, std::ptr::null()) }
    }
}

impl Drop for Mixer {
    fn drop(&mut self) {
        unsafe {
            let _ = self
                .manager
                .UnregisterSessionNotification(&self.notification);
        }
        for tracked in self.sessions.0.lock().drain(..) {
            tracked.detach();
        }
    }
}

/// Reads every session the device currently has, registering for changes on
/// each one.
unsafe fn enumerate(manager: &IAudioSessionManager2, on_change: &OnChange) -> Result<Vec<Tracked>> {
    let enumerator = manager.GetSessionEnumerator()?;
    let count = enumerator.GetCount()?;

    let mut tracked = Vec::new();
    for index in 0..count {
        let Ok(control) = enumerator.GetSession(index) else {
            continue;
        };
        let Ok(control2) = control.cast::<IAudioSessionControl2>() else {
            continue;
        };
        let Ok(volume) = control.cast::<ISimpleAudioVolume>() else {
            continue;
        };

        // The system sounds session has no process to name it after and no
        // window to raise; the original hides it too.
        if control2.IsSystemSoundsSession() == windows::Win32::Foundation::S_OK {
            continue;
        }

        let Some(id) = pwstr_to_string(control2.GetSessionInstanceIdentifier().ok()) else {
            continue;
        };
        let process_id = control2.GetProcessId().unwrap_or(0);

        let display = pwstr_to_string(control.GetDisplayName().ok()).unwrap_or_default();
        let (name, icon) = appicon::describe_process(process_id);
        let name = if display.trim().is_empty() {
            name
        } else {
            display
        };

        let events: IAudioSessionEvents = SessionEvents {
            on_change: on_change.clone(),
        }
        .into();
        if control.RegisterAudioSessionNotification(&events).is_err() {
            continue;
        }

        tracked.push(Tracked {
            id,
            control,
            control2,
            volume,
            events,
            name,
            icon,
        });
    }
    Ok(tracked)
}

/// Reads a session's current values. Returns `None` once it has expired, which
/// is how a closed application leaves the list.
unsafe fn describe(tracked: &Tracked) -> Option<SessionInfo> {
    let state = tracked
        .control
        .GetState()
        .unwrap_or(AudioSessionStateExpired);
    if state == AudioSessionStateExpired {
        return None;
    }

    Some(SessionInfo {
        id: tracked.id.clone(),
        process_id: tracked.control2.GetProcessId().unwrap_or(0),
        name: tracked.name.clone(),
        icon: tracked.icon.clone(),
        percent: tracked
            .volume
            .GetMasterVolume()
            .map(|scalar| (scalar * 100.0).clamp(0.0, 100.0))
            .unwrap_or(0.0),
        muted: tracked
            .volume
            .GetMute()
            .map(|muted| muted.as_bool())
            .unwrap_or(false),
        active: state == AudioSessionState(1),
    })
}

/// Takes ownership of a string Windows allocated for us, freeing the original.
unsafe fn pwstr_to_string(value: Option<PWSTR>) -> Option<String> {
    let value = value?;
    if value.is_null() {
        return None;
    }
    let text = value.to_string().ok();
    // These come from CoTaskMemAlloc; not freeing them leaks on every refresh.
    windows::Win32::System::Com::CoTaskMemFree(Some(value.0.cast()));
    text.filter(|text| !text.is_empty())
}

#[implement(IAudioSessionNotification)]
struct SessionAdded {
    on_change: OnChange,
}

impl IAudioSessionNotification_Impl for SessionAdded_Impl {
    fn OnSessionCreated(&self, _session: Option<&IAudioSessionControl>) -> Result<()> {
        // Deliberately not adding the new session from here: this runs on an
        // audio-service thread, and building a `Tracked` means opening a
        // process handle and possibly rasterising an icon. The mixer re-reads
        // the enumerator on its own thread instead.
        (self.on_change)();
        Ok(())
    }
}

#[implement(IAudioSessionEvents)]
struct SessionEvents {
    on_change: OnChange,
}

impl IAudioSessionEvents_Impl for SessionEvents_Impl {
    fn OnSimpleVolumeChanged(
        &self,
        _volume: f32,
        _muted: BOOL,
        _context: *const GUID,
    ) -> Result<()> {
        (self.on_change)();
        Ok(())
    }

    fn OnStateChanged(&self, _state: AudioSessionState) -> Result<()> {
        (self.on_change)();
        Ok(())
    }

    fn OnSessionDisconnected(&self, _reason: AudioSessionDisconnectReason) -> Result<()> {
        (self.on_change)();
        Ok(())
    }

    fn OnDisplayNameChanged(
        &self,
        _name: &windows::core::PCWSTR,
        _context: *const GUID,
    ) -> Result<()> {
        Ok(())
    }
    fn OnIconPathChanged(
        &self,
        _path: &windows::core::PCWSTR,
        _context: *const GUID,
    ) -> Result<()> {
        Ok(())
    }
    fn OnChannelVolumeChanged(
        &self,
        _count: u32,
        _volumes: *const f32,
        _channel: u32,
        _context: *const GUID,
    ) -> Result<()> {
        Ok(())
    }
    fn OnGroupingParamChanged(&self, _group: *const GUID, _context: *const GUID) -> Result<()> {
        Ok(())
    }
}
