//! Master volume, through WASAPI.
//!
//! The shell needs two things from audio: the current level, and to be told the
//! moment it changes. Polling would satisfy the first and botch the second — a
//! readout that appears half a second after the volume key is pressed reads as
//! lag, not as feedback. So this registers an `IAudioEndpointVolumeCallback`
//! and lets Windows push.
//!
//! The default output device can be swapped while the shell runs, so the
//! registration follows the device: `IMMNotificationClient` re-attaches the
//! callback to whatever becomes default.

use std::sync::Arc;

use parking_lot::Mutex;
use serde::Serialize;
use windows::core::{implement, Result, PCWSTR};
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    eConsole, eRender, IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl,
    MMDeviceEnumerator, AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;

/// What the watcher calls when the level changes. Boxed rather than generic
/// because `#[implement]` generates a concrete type with no parameters.
type OnChange = Arc<dyn Fn(VolumeReading) + Send + Sync + 'static>;

/// A reading of the default output device.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeReading {
    /// 0–100, as a percentage rather than the scalar Windows works in.
    pub percent: f32,
    pub muted: bool,
}

impl Default for VolumeReading {
    fn default() -> Self {
        Self {
            percent: 0.0,
            muted: false,
        }
    }
}

/// Windows works in a 0.0–1.0 scalar; everything above this layer works in
/// percent, because that is what the readout and the config talk about.
fn to_percent(scalar: f32) -> f32 {
    (scalar * 100.0).clamp(0.0, 100.0)
}

fn to_scalar(percent: f32) -> f32 {
    (percent / 100.0).clamp(0.0, 1.0)
}

/// Watches the default output device and reports every change.
///
/// Holding this alive is what keeps the subscription: dropping it unregisters
/// both callbacks.
pub struct VolumeWatcher {
    enumerator: IMMDeviceEnumerator,
    notification_client: IMMNotificationClient,
    endpoint: Arc<SharedEndpoint>,
}

/// The current default device, plus the callback registered against it.
struct Endpoint {
    volume: IAudioEndpointVolume,
    callback: IAudioEndpointVolumeCallback,
}

/// The endpoint, shared between the watcher and the two COM callbacks.
///
/// COM interface pointers are not `Send`/`Sync` to Rust, so the shared handle
/// has to say so explicitly.
struct SharedEndpoint(Mutex<Option<Endpoint>>);

// SAFETY: `IAudioEndpointVolume` and its callback are free-threaded — the audio
// endpoint API registers as such — and every access here goes through the
// mutex, including the re-attach that runs on a device-change callback.
unsafe impl Send for SharedEndpoint {}
unsafe impl Sync for SharedEndpoint {}

impl SharedEndpoint {
    fn new() -> Arc<Self> {
        Arc::new(Self(Mutex::new(None)))
    }
}

impl Endpoint {
    /// Unregisters before the interface is released; leaving it attached leaks
    /// the callback inside the audio service.
    fn detach(&self) {
        unsafe {
            let _ = self.volume.UnregisterControlChangeNotify(&self.callback);
        }
    }
}

// SAFETY: as for `SharedEndpoint` — the interfaces are free-threaded, and the
// enumerator and notification client are only touched on construction and drop.
unsafe impl Send for VolumeWatcher {}
unsafe impl Sync for VolumeWatcher {}

impl VolumeWatcher {
    /// Starts watching, calling `on_change` for every volume or mute change.
    ///
    /// The callback runs on an audio-service thread, so it must not block.
    pub fn new(on_change: impl Fn(VolumeReading) + Send + Sync + 'static) -> Result<Self> {
        let on_change: OnChange = Arc::new(on_change);
        unsafe {
            // The audio APIs are only usable from an initialised apartment, and
            // the watcher may be built on a thread nobody else initialised.
            // A second call on an already-initialised thread is harmless.
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

            let endpoint = SharedEndpoint::new();

            attach(&enumerator, &endpoint, on_change.clone())?;

            // Following the default device means the readout keeps working when
            // headphones are plugged in, rather than silently reporting the old
            // device forever.
            let notification_client: IMMNotificationClient = DefaultDeviceWatcher {
                enumerator: enumerator.clone(),
                endpoint: endpoint.clone(),
                on_change,
            }
            .into();
            enumerator.RegisterEndpointNotificationCallback(&notification_client)?;

            Ok(Self {
                enumerator,
                notification_client,
                endpoint,
            })
        }
    }

    /// The level right now, without waiting for a change.
    pub fn read(&self) -> crate::providers::VolumeReading {
        let guard = self.endpoint.0.lock();
        let Some(endpoint) = guard.as_ref() else {
            return crate::providers::VolumeReading::default();
        };
        unsafe { read_endpoint(&endpoint.volume) }.into()
    }

    /// Sets the level, clamped to `ceiling` percent.
    ///
    /// The ceiling is the hearing-protection setting: the shell's own controls
    /// will not put the volume somewhere painful, even if asked to.
    pub fn set_percent(&self, percent: f32, ceiling: f32) -> Result<()> {
        let guard = self.endpoint.0.lock();
        let Some(endpoint) = guard.as_ref() else {
            return Ok(());
        };
        let target = to_scalar(percent.min(ceiling));
        unsafe {
            endpoint
                .volume
                .SetMasterVolumeLevelScalar(target, std::ptr::null())
        }
    }

    pub fn set_muted(&self, muted: bool) -> Result<()> {
        let guard = self.endpoint.0.lock();
        let Some(endpoint) = guard.as_ref() else {
            return Ok(());
        };
        unsafe { endpoint.volume.SetMute(muted, std::ptr::null()) }
    }
}

impl Drop for VolumeWatcher {
    fn drop(&mut self) {
        unsafe {
            let _ = self
                .enumerator
                .UnregisterEndpointNotificationCallback(&self.notification_client);
        }
        if let Some(endpoint) = self.endpoint.0.lock().take() {
            endpoint.detach();
        }
    }
}

unsafe fn read_endpoint(volume: &IAudioEndpointVolume) -> VolumeReading {
    let percent = volume
        .GetMasterVolumeLevelScalar()
        .map(to_percent)
        .unwrap_or(0.0);
    let muted = volume
        .GetMute()
        .map(|muted| muted.as_bool())
        .unwrap_or(false);
    VolumeReading { percent, muted }
}

/// Points the callback at the current default output device.
unsafe fn attach(
    enumerator: &IMMDeviceEnumerator,
    endpoint: &Arc<SharedEndpoint>,
    on_change: OnChange,
) -> Result<()> {
    let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
    let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;

    let callback: IAudioEndpointVolumeCallback = VolumeCallback {
        on_change: on_change.clone(),
    }
    .into();
    volume.RegisterControlChangeNotify(&callback)?;

    // Replace rather than add: the old device's callback must come off first,
    // or changes on a disconnected device would keep arriving.
    let mut guard = endpoint.0.lock();
    if let Some(previous) = guard.take() {
        previous.detach();
    }
    let reading = read_endpoint(&volume);
    *guard = Some(Endpoint { volume, callback });
    drop(guard);

    on_change(reading);
    Ok(())
}

#[implement(IAudioEndpointVolumeCallback)]
struct VolumeCallback {
    on_change: OnChange,
}

impl IAudioEndpointVolumeCallback_Impl for VolumeCallback_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> Result<()> {
        // The notification already carries the new level, so there is no need
        // to call back into the endpoint from this thread.
        let reading = unsafe {
            data.as_ref()
                .map(|data| VolumeReading {
                    percent: to_percent(data.fMasterVolume),
                    muted: data.bMuted.as_bool(),
                })
                .unwrap_or_default()
        };
        (self.on_change)(reading);
        Ok(())
    }
}

#[implement(IMMNotificationClient)]
struct DefaultDeviceWatcher {
    enumerator: IMMDeviceEnumerator,
    endpoint: Arc<SharedEndpoint>,
    on_change: OnChange,
}

impl IMMNotificationClient_Impl for DefaultDeviceWatcher_Impl {
    fn OnDefaultDeviceChanged(
        &self,
        flow: windows::Win32::Media::Audio::EDataFlow,
        role: windows::Win32::Media::Audio::ERole,
        _id: &PCWSTR,
    ) -> Result<()> {
        // Only the device this shell reads from, and only the role it uses.
        if flow != eRender || role != eConsole {
            return Ok(());
        }
        unsafe {
            // A failure here means there is no output device at all, which is a
            // normal state; the readout simply reports nothing until one exists.
            let _ = attach(&self.enumerator, &self.endpoint, self.on_change.clone());
        }
        Ok(())
    }

    fn OnDeviceStateChanged(&self, _id: &PCWSTR, _state: DEVICE_STATE) -> Result<()> {
        Ok(())
    }
    fn OnDeviceAdded(&self, _id: &PCWSTR) -> Result<()> {
        Ok(())
    }
    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> Result<()> {
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _id: &PCWSTR, _key: &PROPERTYKEY) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_and_percentages_round_trip() {
        for percent in [0.0f32, 12.5, 50.0, 99.9, 100.0] {
            let round_tripped = to_percent(to_scalar(percent));
            assert!(
                (round_tripped - percent).abs() < 0.01,
                "{percent} became {round_tripped}"
            );
        }
    }

    #[test]
    fn conversions_clamp_rather_than_wrap() {
        // A device reporting slightly out of range must not produce a negative
        // bar or one that overflows its track.
        assert_eq!(to_percent(-0.5), 0.0);
        assert_eq!(to_percent(1.5), 100.0);
        assert_eq!(to_scalar(-10.0), 0.0);
        assert_eq!(to_scalar(150.0), 1.0);
    }
}
