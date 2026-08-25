//! Wi-Fi and Bluetooth, through WinRT.
//!
//! The roadmap assumed this would need `wlanapi` wrapped by hand and a WinRT
//! Bluetooth stack the platform layer does not have. It does not: everything
//! here is already in the `windows` crate, so the work is calling it correctly
//! rather than binding it.
//!
//! Every entry point can legitimately fail on a healthy machine — a desktop
//! with no Wi-Fi adapter, a radio switched off in firmware, an access request
//! denied by policy — so each returns an empty or `None` result rather than an
//! error, and the sidebar hides the control instead of showing a dead one.
//!
//! These are all blocking: WinRT's async operations are waited on with `get`.
//! Callers must therefore keep them off the async runtime's threads, which the
//! commands do with `spawn_blocking`.

use crate::providers::{BluetoothDeviceInfo, ConnectOutcome, RadiosState, WifiNetwork};
use windows::Devices::Enumeration::DeviceInformation;
use windows::Devices::Radios::{Radio, RadioAccessStatus, RadioKind, RadioState};
use windows::Devices::WiFi::{
    WiFiAdapter, WiFiAvailableNetwork, WiFiConnectionStatus, WiFiReconnectionKind,
};
use windows::Security::Credentials::PasswordCredential;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

/// Which radio a toggle means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    WiFi,
    Bluetooth,
}

impl Kind {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "wifi" => Some(Self::WiFi),
            "bluetooth" => Some(Self::Bluetooth),
            _ => None,
        }
    }

    fn as_radio_kind(self) -> RadioKind {
        match self {
            Self::WiFi => RadioKind::WiFi,
            Self::Bluetooth => RadioKind::Bluetooth,
        }
    }
}

/// Initialises the apartment for whichever thread is calling.
///
/// A second call on an already-initialised thread is harmless, and
/// `spawn_blocking` hands out threads nobody else has prepared.
fn ensure_apartment() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/// The state of both radios.
pub fn state() -> RadiosState {
    ensure_apartment();

    let allowed = matches!(access_status(), Some(RadioAccessStatus::Allowed));
    let mut state = RadiosState {
        can_control: allowed,
        ..RadiosState::default()
    };

    let Some(radios) = all_radios() else {
        return state;
    };

    for radio in radios {
        let Ok(kind) = radio.Kind() else { continue };
        let on = radio.State().map(|state| state == RadioState::On).ok();

        if kind == RadioKind::WiFi {
            // A machine can have several adapters; one being on is enough to
            // call Wi-Fi on.
            state.wifi = Some(state.wifi.unwrap_or(false) || on.unwrap_or(false));
        } else if kind == RadioKind::Bluetooth {
            state.bluetooth = Some(state.bluetooth.unwrap_or(false) || on.unwrap_or(false));
        }
    }
    state
}

/// Turns a radio on or off. Returns whether anything actually changed.
pub fn set(kind: Kind, on: bool) -> bool {
    ensure_apartment();

    if !matches!(access_status(), Some(RadioAccessStatus::Allowed)) {
        tracing::info!("radio access is denied; the toggle is unavailable");
        return false;
    }

    let Some(radios) = all_radios() else {
        return false;
    };

    let target = if on { RadioState::On } else { RadioState::Off };
    let mut changed = false;
    for radio in radios {
        if radio.Kind().ok() != Some(kind.as_radio_kind()) {
            continue;
        }
        // Every matching adapter is set. Deliberately not stopping at the
        // first success: a laptop with two Wi-Fi adapters would otherwise be
        // left half on.
        if let Ok(operation) = radio.SetStateAsync(target) {
            if operation.get().ok() == Some(RadioAccessStatus::Allowed) {
                changed = true;
            }
        }
    }
    changed
}

fn access_status() -> Option<RadioAccessStatus> {
    Radio::RequestAccessAsync().ok()?.get().ok()
}

fn all_radios() -> Option<Vec<Radio>> {
    let list = Radio::GetRadiosAsync().ok()?.get().ok()?;
    Some(list.into_iter().collect())
}

/// Scans for networks.
///
/// Slow — the scan itself takes seconds — so this is only called when the
/// Wi-Fi dialog is opened, never on a timer.
pub fn scan() -> Vec<WifiNetwork> {
    ensure_apartment();

    let Some(adapter) = first_adapter() else {
        return Vec::new();
    };

    // A failed scan still leaves the previous report readable, which is better
    // than an empty list while the radio settles.
    if let Ok(operation) = adapter.ScanAsync() {
        let _ = operation.get();
    }

    let Ok(report) = adapter.NetworkReport() else {
        return Vec::new();
    };
    let Ok(networks) = report.AvailableNetworks() else {
        return Vec::new();
    };

    let mut found: Vec<WifiNetwork> = Vec::new();
    for network in networks {
        let Some(entry) = describe_network(&network) else {
            continue;
        };
        // The same SSID appears once per band and per access point; the user
        // thinks of it as one network, so keep the strongest.
        match found.iter_mut().find(|other| other.ssid == entry.ssid) {
            Some(existing) if existing.bars < entry.bars => *existing = entry,
            Some(_) => {}
            None => found.push(entry),
        }
    }

    found.sort_by(|a, b| b.bars.cmp(&a.bars).then_with(|| a.ssid.cmp(&b.ssid)));
    found
}

fn describe_network(network: &WiFiAvailableNetwork) -> Option<WifiNetwork> {
    let ssid = network.Ssid().ok()?.to_string();
    // Hidden networks report an empty SSID and cannot be joined from a list.
    if ssid.is_empty() {
        return None;
    }

    let secured = network
        .SecuritySettings()
        .and_then(|settings| settings.NetworkAuthenticationType())
        .map(|authentication| {
            authentication
                != windows::Networking::Connectivity::NetworkAuthenticationType::Open80211
        })
        .unwrap_or(true);

    Some(WifiNetwork {
        ssid,
        bars: network.SignalBars().unwrap_or(0),
        secured,
    })
}

/// Joins a network, with a password when it needs one.
pub fn connect(ssid: &str, password: Option<&str>) -> ConnectOutcome {
    ensure_apartment();

    let Some(adapter) = first_adapter() else {
        return ConnectOutcome::Failed;
    };
    let Some(network) = find_network(&adapter, ssid) else {
        return ConnectOutcome::Failed;
    };

    // `Automatic` is what the Windows UI does: the machine rejoins this
    // network by itself next time it is in range.
    let result = match password {
        Some(password) => credential(password).and_then(|credential| {
            adapter
                .ConnectWithPasswordCredentialAsync(
                    &network,
                    WiFiReconnectionKind::Automatic,
                    &credential,
                )
                .ok()?
                .get()
                .ok()
        }),
        None => adapter
            .ConnectAsync(&network, WiFiReconnectionKind::Automatic)
            .ok()
            .and_then(|operation| operation.get().ok()),
    };

    match result.and_then(|result| result.ConnectionStatus().ok()) {
        Some(WiFiConnectionStatus::Success) => ConnectOutcome::Connected,
        Some(WiFiConnectionStatus::InvalidCredential) => ConnectOutcome::BadPassword,
        _ => ConnectOutcome::Failed,
    }
}

pub fn disconnect() {
    ensure_apartment();
    if let Some(adapter) = first_adapter() {
        adapter.Disconnect().ok();
    }
}

fn credential(password: &str) -> Option<PasswordCredential> {
    let credential = PasswordCredential::new().ok()?;
    credential
        .SetPassword(&windows::core::HSTRING::from(password))
        .ok()?;
    Some(credential)
}

fn find_network(adapter: &WiFiAdapter, ssid: &str) -> Option<WiFiAvailableNetwork> {
    let networks = adapter.NetworkReport().ok()?.AvailableNetworks().ok()?;
    networks
        .into_iter()
        .find(|network| network.Ssid().map(|found| found.to_string()).as_deref() == Ok(ssid))
}

/// The first Wi-Fi adapter the machine has, if any.
///
/// Several adapters is vanishingly rare outside a lab, and a network picker
/// that asks which one to use would be worse than one that picks.
fn first_adapter() -> Option<WiFiAdapter> {
    // Access can be refused by policy, in which case enumeration succeeds but
    // every operation on the adapter fails.
    if WiFiAdapter::RequestAccessAsync().ok()?.get().ok()?
        != windows::Devices::WiFi::WiFiAccessStatus::Allowed
    {
        return None;
    }
    let adapters = WiFiAdapter::FindAllAdaptersAsync().ok()?.get().ok()?;
    adapters.into_iter().next()
}

/// Paired Bluetooth devices.
///
/// Only paired ones: pairing a new device needs a PIN exchange with a UI of
/// its own, and Windows already has one. Connecting an already-paired device
/// is largely the stack's decision rather than ours, so the sidebar shows
/// state and offers Windows' own settings for the rest.
pub fn paired_devices() -> Vec<BluetoothDeviceInfo> {
    ensure_apartment();

    let selector =
        match windows::Devices::Bluetooth::BluetoothDevice::GetDeviceSelectorFromPairingState(true)
        {
            Ok(selector) => selector,
            Err(_) => return Vec::new(),
        };

    let Ok(operation) = DeviceInformation::FindAllAsyncAqsFilter(&selector) else {
        return Vec::new();
    };
    let Ok(found) = operation.get() else {
        return Vec::new();
    };

    found
        .into_iter()
        .filter_map(|information| {
            let id = information.Id().ok()?.to_string();
            let name = information.Name().ok()?.to_string();
            if name.is_empty() {
                return None;
            }
            // `IsEnabled` is the closest thing DeviceInformation offers to
            // "connected" without opening the device itself, which would wake
            // it up just to draw a list.
            let connected = information.IsEnabled().unwrap_or(false);
            Some(BluetoothDeviceInfo {
                id,
                name,
                connected,
            })
        })
        .collect()
}
