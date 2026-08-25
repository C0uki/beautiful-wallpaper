//! Display brightness, and the night light.
//!
//! Windows has no single API for this. A laptop panel is driven through WMI
//! (`WmiMonitorBrightnessMethods`), an external display through DDC/CI over the
//! monitor cable, and a display that supports neither can only be faked with a
//! gamma ramp. All three are here, tried in that order, and every one of them
//! can fail on a perfectly ordinary machine — so "no brightness control" is a
//! first-class outcome rather than an error. The readout and the sidebar's
//! slider simply do not appear when it happens.
//!
//! DDC/CI is the awkward one: each call is a round trip over I²C and takes tens
//! of milliseconds, so dragging a slider would queue hundreds of writes behind
//! each other. Everything therefore runs on one worker thread that coalesces
//! pending requests down to the most recent before touching the hardware.
//!
//! The arithmetic — the display's arbitrary raw range, and the colour
//! temperature curve — lives in `bw_core::brightness` so that it is covered by
//! tests that run on Linux. This module is the part that cannot be.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{mpsc, Arc};

use bw_core::brightness as math;
use windows::core::{BSTR, HSTRING, PCWSTR, VARIANT};
use windows::Win32::Devices::Display::{
    DestroyPhysicalMonitors, GetMonitorBrightness, GetNumberOfPhysicalMonitorsFromHMONITOR,
    GetPhysicalMonitorsFromHMONITOR, SetMonitorBrightness, PHYSICAL_MONITOR,
};
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetDC, ReleaseDC, HDC, HMONITOR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoSetProxyBlanket, CLSCTX_INPROC_SERVER,
    COINIT_MULTITHREADED, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Wmi::{
    IWbemClassObject, IWbemLocator, IWbemServices, WbemLocator, WBEM_FLAG_FORWARD_ONLY,
    WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_GENERIC_FLAG_TYPE,
};
use windows::Win32::UI::ColorSystem::SetDeviceGammaRamp;

/// What the watcher calls when the level changes. Boxed for the same reason the
/// audio callback is: the worker holds one and cannot be generic over it.
type OnChange = Arc<dyn Fn(u8) + Send + Sync + 'static>;

/// Milliseconds WMI is allowed to spend applying a level before giving up.
const WMI_TIMEOUT_MS: u32 = 0;

/// The cached level when nothing can report one.
const UNKNOWN: i32 = -1;

/// A request for the worker thread.
enum Request {
    Set(u8),
    /// The tint, as a colour temperature. `None` turns it off.
    NightLight(Option<u32>),
    /// Re-read the hardware, e.g. after the user changed it with a Fn key.
    Refresh,
}

/// Handle on the brightness worker.
///
/// Dropping it stops the thread and restores a neutral gamma ramp, so exiting
/// the shell never leaves the display tinted.
pub struct BrightnessControl {
    requests: Sender<Request>,
    /// The last known level, or [`UNKNOWN`]. Read without touching hardware so
    /// that a command can answer immediately.
    cached: Arc<AtomicI32>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl BrightnessControl {
    /// Starts the worker, calling `on_change` whenever the level moves.
    ///
    /// Always succeeds: a machine with no controllable display is a normal
    /// state, and the shell keeps working without the feature.
    pub fn new(on_change: impl Fn(u8) + Send + Sync + 'static) -> Self {
        let on_change: OnChange = Arc::new(on_change);
        let (requests, incoming) = mpsc::channel();
        let cached = Arc::new(AtomicI32::new(UNKNOWN));

        let worker = {
            let cached = cached.clone();
            std::thread::Builder::new()
                .name("bw-brightness".to_owned())
                .spawn(move || run(incoming, cached, on_change))
                .ok()
        };

        Self {
            requests,
            cached,
            worker,
        }
    }

    /// The level right now, or `None` when no display can report one.
    pub fn read(&self) -> Option<u8> {
        match self.cached.load(Ordering::Relaxed) {
            UNKNOWN => None,
            percent => u8::try_from(percent).ok(),
        }
    }

    pub fn is_supported(&self) -> bool {
        self.read().is_some()
    }

    /// Asks for a new level. Returns without waiting for the hardware.
    pub fn set(&self, percent: u8) {
        let _ = self.requests.send(Request::Set(percent.min(100)));
    }

    /// Applies, or clears, the warm tint.
    pub fn set_night_light(&self, kelvin: Option<u32>) {
        let _ = self.requests.send(Request::NightLight(kelvin));
    }

    pub fn refresh(&self) {
        let _ = self.requests.send(Request::Refresh);
    }
}

impl Drop for BrightnessControl {
    fn drop(&mut self) {
        // Dropping the sender is what ends the worker's `recv` loop; it clears
        // the gamma ramp on the way out.
        let (dead, _) = mpsc::channel();
        let _ = std::mem::replace(&mut self.requests, dead);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// The worker's whole life: probe once, then serve requests until the handle
/// goes away.
fn run(incoming: Receiver<Request>, cached: Arc<AtomicI32>, on_change: OnChange) {
    unsafe {
        // WMI needs an initialised apartment, and this thread is ours alone.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let mut backend = Backend::probe();

    if let Some(percent) = backend.read() {
        cached.store(i32::from(percent), Ordering::Relaxed);
        on_change(percent);
    }

    while let Ok(first) = incoming.recv() {
        // Coalesce: a slider drag arrives as a burst, and only the last value
        // in it is worth the round trip to the hardware.
        let mut set = None;
        let mut night = None;
        let mut refresh = false;
        let mut disconnected = false;

        for request in std::iter::once(first).chain(drain(&incoming, &mut disconnected)) {
            match request {
                Request::Set(percent) => set = Some(percent),
                Request::NightLight(kelvin) => night = Some(kelvin),
                Request::Refresh => refresh = true,
            }
        }

        if let Some(kelvin) = night {
            apply_gamma(kelvin);
        }

        if let Some(percent) = set {
            if backend.set(percent) {
                cached.store(i32::from(percent), Ordering::Relaxed);
                on_change(percent);
            }
        } else if refresh {
            if let Some(percent) = backend.read() {
                if cached.swap(i32::from(percent), Ordering::Relaxed) != i32::from(percent) {
                    on_change(percent);
                }
            }
        }

        if disconnected {
            break;
        }
    }

    // Leaving the display tinted after the shell exits would look like a
    // hardware fault, and there would be nothing left to undo it.
    apply_gamma(None);
}

/// Everything already queued, so a burst is applied once.
fn drain(incoming: &Receiver<Request>, disconnected: &mut bool) -> Vec<Request> {
    let mut rest = Vec::new();
    loop {
        match incoming.try_recv() {
            Ok(request) => rest.push(request),
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                *disconnected = true;
                break;
            }
        }
    }
    rest
}

/// How this machine's brightness is reached, in order of preference.
enum Backend {
    /// A laptop panel, through WMI. The only one that survives a lid close.
    Panel(Wmi),
    /// External displays over DDC/CI. Several may be attached; they move
    /// together, because one slider cannot mean two different levels.
    External(Vec<Physical>),
    /// Nothing can report a level. The feature is hidden rather than faked.
    None,
}

impl Backend {
    fn probe() -> Self {
        if let Some(wmi) = Wmi::open() {
            if wmi.read().is_some() {
                return Self::Panel(wmi);
            }
        }

        let monitors = Physical::enumerate();
        if monitors.iter().any(|monitor| monitor.read().is_some()) {
            return Self::External(monitors);
        }

        tracing::info!("no display reports a brightness level; the control is hidden");
        Self::None
    }

    fn read(&self) -> Option<u8> {
        match self {
            Self::Panel(wmi) => wmi.read(),
            // The first display that answers speaks for the group.
            Self::External(monitors) => monitors.iter().find_map(Physical::read),
            Self::None => None,
        }
    }

    /// Returns whether anything actually changed.
    fn set(&mut self, percent: u8) -> bool {
        match self {
            Self::Panel(wmi) => wmi.set(percent),
            Self::External(monitors) => {
                // Every display is written, and one success is enough to call
                // it applied. Deliberately not `any`: that stops at the first
                // display that answers, leaving the rest at the old level.
                let mut applied = false;
                for monitor in monitors.iter() {
                    applied |= monitor.set(percent);
                }
                applied
            }
            Self::None => false,
        }
    }
}

/// The laptop panel, through `root\WMI`.
struct Wmi {
    services: IWbemServices,
}

// SAFETY: the services pointer never leaves the worker thread that created it,
// and the apartment it was created in is that thread's.
unsafe impl Send for Wmi {}

impl Wmi {
    fn open() -> Option<Self> {
        unsafe {
            let locator: IWbemLocator =
                CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER).ok()?;

            let services = locator
                .ConnectServer(
                    &BSTR::from("root\\WMI"),
                    &BSTR::new(),
                    &BSTR::new(),
                    &BSTR::new(),
                    0,
                    &BSTR::new(),
                    None,
                )
                .ok()?;

            // Without this the proxy refuses the calls below with E_ACCESSDENIED.
            CoSetProxyBlanket(
                &services,
                windows::Win32::System::Rpc::RPC_C_AUTHN_WINNT,
                windows::Win32::System::Rpc::RPC_C_AUTHZ_NONE,
                PCWSTR::null(),
                RPC_C_AUTHN_LEVEL_CALL,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            )
            .ok()?;

            Some(Self { services })
        }
    }

    fn query(&self, wql: &str) -> Option<Vec<IWbemClassObject>> {
        unsafe {
            let enumerator = self
                .services
                .ExecQuery(
                    &BSTR::from("WQL"),
                    &BSTR::from(wql),
                    WBEM_GENERIC_FLAG_TYPE(
                        WBEM_FLAG_FORWARD_ONLY.0 | WBEM_FLAG_RETURN_IMMEDIATELY.0,
                    ),
                    None,
                )
                .ok()?;

            let mut found = Vec::new();
            loop {
                let mut batch = [const { None }; 8];
                let mut returned = 0;
                // A non-success HRESULT here means the end of the results, not
                // a failure worth reporting.
                let _ = enumerator.Next(WMI_QUERY_TIMEOUT_MS, &mut batch, &mut returned);
                if returned == 0 {
                    break;
                }
                found.extend(batch.into_iter().take(returned as usize).flatten());
            }
            Some(found)
        }
    }

    fn read(&self) -> Option<u8> {
        let objects = self.query("SELECT CurrentBrightness FROM WmiMonitorBrightness")?;
        let object = objects.first()?;
        let raw = property_u32(object, "CurrentBrightness")?;
        // WMI already speaks in percent, unlike DDC/CI.
        u8::try_from(raw.min(100)).ok()
    }

    fn set(&self, percent: u8) -> bool {
        let Some(instances) = self.query("SELECT InstanceName FROM WmiMonitorBrightnessMethods")
        else {
            return false;
        };

        let mut applied = false;
        for instance in instances {
            let Some(name) = property_string(&instance, "InstanceName") else {
                continue;
            };
            if self.set_on(&name, percent).is_some() {
                applied = true;
            }
        }
        applied
    }

    /// Calls `WmiSetBrightness` on one panel.
    fn set_on(&self, instance_name: &str, percent: u8) -> Option<()> {
        unsafe {
            let mut class = None;
            self.services
                .GetObject(
                    &BSTR::from("WmiMonitorBrightnessMethods"),
                    WBEM_GENERIC_FLAG_TYPE(0),
                    None,
                    Some(&mut class),
                    None,
                )
                .ok()?;
            let class = class?;

            let mut signature = None;
            class
                .GetMethod(
                    &HSTRING::from("WmiSetBrightness"),
                    0,
                    &mut signature,
                    &mut None,
                )
                .ok()?;
            let arguments = signature?.SpawnInstance(0).ok()?;

            arguments
                .Put(
                    &HSTRING::from("Timeout"),
                    0,
                    &VARIANT::from(WMI_TIMEOUT_MS),
                    0,
                )
                .ok()?;
            arguments
                .Put(&HSTRING::from("Brightness"), 0, &VARIANT::from(percent), 0)
                .ok()?;

            // WMI addresses an instance by a quoted key; a name containing a
            // quote would otherwise break out of the path.
            let path = format!(
                "WmiMonitorBrightnessMethods.InstanceName=\"{}\"",
                instance_name.replace('\\', "\\\\").replace('"', "\\\"")
            );
            self.services
                .ExecMethod(
                    &BSTR::from(path),
                    &BSTR::from("WmiSetBrightness"),
                    WBEM_GENERIC_FLAG_TYPE(0),
                    None,
                    &arguments,
                    None,
                    None,
                )
                .ok()?;
            Some(())
        }
    }
}

/// How long a WMI enumeration step may block. Short: this runs on the worker,
/// and a wedged WMI service must not hold a slider drag hostage.
const WMI_QUERY_TIMEOUT_MS: i32 = 1_000;

fn property_u32(object: &IWbemClassObject, name: &str) -> Option<u32> {
    unsafe {
        let mut value = VARIANT::default();
        object
            .Get(&HSTRING::from(name), 0, &mut value, None, None)
            .ok()?;
        u32::try_from(&value).ok()
    }
}

fn property_string(object: &IWbemClassObject, name: &str) -> Option<String> {
    unsafe {
        let mut value = VARIANT::default();
        object
            .Get(&HSTRING::from(name), 0, &mut value, None, None)
            .ok()?;
        let text = BSTR::try_from(&value).ok()?.to_string();
        (!text.is_empty()).then_some(text)
    }
}

/// One external display, reached over DDC/CI.
struct Physical {
    handle: PHYSICAL_MONITOR,
}

// SAFETY: the handle never leaves the worker thread that opened it.
unsafe impl Send for Physical {}

impl Physical {
    /// Every physical monitor attached to every adapter.
    fn enumerate() -> Vec<Self> {
        let mut handles: Vec<HMONITOR> = Vec::new();

        unsafe extern "system" fn collect(
            monitor: HMONITOR,
            _dc: HDC,
            _rect: *mut RECT,
            data: LPARAM,
        ) -> BOOL {
            let found = &mut *(data.0 as *mut Vec<HMONITOR>);
            found.push(monitor);
            true.into()
        }

        unsafe {
            let _ = EnumDisplayMonitors(
                None,
                None,
                Some(collect),
                LPARAM(std::ptr::addr_of_mut!(handles) as isize),
            );
        }

        handles
            .into_iter()
            .flat_map(|handle| unsafe { physical_for(handle) })
            .map(|handle| Self { handle })
            .collect()
    }

    fn read(&self) -> Option<u8> {
        let (mut min, mut current, mut max) = (0u32, 0u32, 0u32);
        // Returns zero on failure, which is normal: plenty of displays ignore
        // DDC/CI entirely, and a cable can be pulled at any moment.
        let ok = unsafe {
            GetMonitorBrightness(
                self.handle.hPhysicalMonitor,
                &mut min,
                &mut current,
                &mut max,
            )
        };
        (ok != 0).then(|| math::to_percent(current, min, max))
    }

    fn set(&self, percent: u8) -> bool {
        let (mut min, mut current, mut max) = (0u32, 0u32, 0u32);
        let ok = unsafe {
            GetMonitorBrightness(
                self.handle.hPhysicalMonitor,
                &mut min,
                &mut current,
                &mut max,
            )
        };
        if ok == 0 {
            return false;
        }

        let raw = math::from_percent(percent, min, max);
        unsafe { SetMonitorBrightness(self.handle.hPhysicalMonitor, raw) != 0 }
    }
}

impl Drop for Physical {
    fn drop(&mut self) {
        // These are real handles into the display driver; leaking them keeps a
        // reference to hardware the user may be trying to unplug.
        unsafe {
            let _ = DestroyPhysicalMonitors(std::slice::from_ref(&self.handle));
        }
    }
}

/// The physical monitors behind one `HMONITOR` — usually one, but a splitter
/// or a docked multi-head adapter presents several.
unsafe fn physical_for(monitor: HMONITOR) -> Vec<PHYSICAL_MONITOR> {
    let mut count = 0;
    if GetNumberOfPhysicalMonitorsFromHMONITOR(monitor, &mut count).is_err() || count == 0 {
        return Vec::new();
    }

    let mut buffer = vec![PHYSICAL_MONITOR::default(); count as usize];
    if GetPhysicalMonitorsFromHMONITOR(monitor, &mut buffer).is_err() {
        return Vec::new();
    }
    buffer
}

/// Writes the tint to every display, or clears it when `kelvin` is `None`.
///
/// Gamma ramps are refused outright on some configurations — HDR displays and
/// several laptop drivers — so a failure is logged once and otherwise ignored.
fn apply_gamma(kelvin: Option<u32>) {
    let kelvin = kelvin.unwrap_or(math::NEUTRAL_KELVIN);
    let ramp = math::gamma_ramp(kelvin, 1.0);

    unsafe {
        // A null HDC is the whole screen, which is what a tint should cover.
        let dc = GetDC(None);
        if dc.is_invalid() {
            return;
        }
        let ok = SetDeviceGammaRamp(dc, ramp.as_ptr().cast());
        ReleaseDC(None, dc);

        if !ok.as_bool() {
            tracing::debug!(
                kelvin,
                "the display refused a gamma ramp; night light is off"
            );
        }
    }
}
