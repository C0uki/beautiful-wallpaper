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

/// Bluetooth devices: the paired ones, or with `paired` false, the unpaired
/// ones in range — asking for those is what makes the radio look for them.
///
/// Connection state and whether a device is audio come from the stack's own
/// record of it; opening that record does not page the device.
pub fn devices(paired: bool) -> Vec<BluetoothDeviceInfo> {
    use windows::Devices::Bluetooth::{
        BluetoothConnectionStatus, BluetoothDevice, BluetoothMajorClass,
    };
    ensure_apartment();

    let Ok(selector) = BluetoothDevice::GetDeviceSelectorFromPairingState(paired) else {
        return Vec::new();
    };
    let Ok(found) =
        DeviceInformation::FindAllAsyncAqsFilter(&selector).and_then(|operation| operation.get())
    else {
        return Vec::new();
    };

    found
        .into_iter()
        .filter_map(|information| {
            let id = information.Id().ok()?;
            let name = information.Name().ok()?.to_string();
            if name.is_empty() {
                return None;
            }
            let device = BluetoothDevice::FromIdAsync(&id)
                .and_then(|operation| operation.get())
                .ok();
            let connected = device
                .as_ref()
                .and_then(|device| device.ConnectionStatus().ok())
                == Some(BluetoothConnectionStatus::Connected);
            let audio = device
                .as_ref()
                .and_then(|device| device.ClassOfDevice().ok())
                .and_then(|class| class.MajorClass().ok())
                == Some(BluetoothMajorClass::AudioVideo);
            Some(BluetoothDeviceInfo {
                id: id.to_string(),
                name,
                connected,
                paired,
                audio,
            })
        })
        .collect()
}

/// What pairing needs the person to see or answer. `kind` is `displayPin`
/// (type this on the device), `confirmPin` (does the device show this?) or
/// `providePin` (type the device's PIN here).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingPrompt {
    pub kind: &'static str,
    pub pin: String,
}

/// Where a pairing that is waiting on the person hears back: `Some` with the
/// PIN typed (or anything, for a yes), `None` for a no.
static ANSWER: parking_lot::Mutex<Option<std::sync::mpsc::Sender<Option<String>>>> =
    parking_lot::Mutex::new(None);

/// Pairs with a device, asking the person through `ask` when the device wants
/// a PIN shown, confirmed or typed. Blocks until it is done.
pub fn pair(id: &str, ask: impl Fn(PairingPrompt) + Send + Sync + 'static) -> bool {
    use windows::Devices::Enumeration::{
        DeviceInformationCustomPairing, DevicePairingKinds, DevicePairingRequestedEventArgs,
        DevicePairingResultStatus,
    };
    use windows::Foundation::TypedEventHandler;
    ensure_apartment();

    let attempt = || -> windows::core::Result<bool> {
        let information =
            DeviceInformation::CreateFromIdAsync(&windows::core::HSTRING::from(id))?.get()?;
        let custom = information.Pairing()?.Custom()?;
        let handler = TypedEventHandler::<
            DeviceInformationCustomPairing,
            DevicePairingRequestedEventArgs,
        >::new(move |_, args| {
            let Some(args) = args.as_ref() else {
                return Ok(());
            };
            let kind = args.PairingKind()?;
            if kind == DevicePairingKinds::ConfirmOnly {
                return args.Accept();
            }
            let pin = args.Pin()?.to_string();
            if kind == DevicePairingKinds::DisplayPin {
                ask(PairingPrompt {
                    kind: "displayPin",
                    pin,
                });
                return args.Accept();
            }
            let (send, receive) = std::sync::mpsc::channel();
            *ANSWER.lock() = Some(send);
            let providing = kind == DevicePairingKinds::ProvidePin;
            ask(PairingPrompt {
                kind: if providing {
                    "providePin"
                } else {
                    "confirmPin"
                },
                pin,
            });
            // A minute to answer. Not accepting is how pairing is refused.
            match receive.recv_timeout(std::time::Duration::from_secs(60)) {
                Ok(Some(typed)) if providing => {
                    args.AcceptWithPin(&windows::core::HSTRING::from(typed))
                }
                Ok(Some(_)) => args.Accept(),
                _ => Ok(()),
            }
        });
        let token = custom.PairingRequested(&handler)?;
        let kinds = DevicePairingKinds::ConfirmOnly
            | DevicePairingKinds::DisplayPin
            | DevicePairingKinds::ProvidePin
            | DevicePairingKinds::ConfirmPinMatch;
        let result = custom
            .PairAsync(kinds)
            .and_then(|operation| operation.get());
        let _ = custom.RemovePairingRequested(token);
        let status = result?.Status()?;
        Ok(status == DevicePairingResultStatus::Paired
            || status == DevicePairingResultStatus::AlreadyPaired)
    };
    let paired = attempt().unwrap_or_else(|error| {
        tracing::warn!(%error, "could not pair a Bluetooth device");
        false
    });
    ANSWER.lock().take();
    paired
}

/// The person's answer to the pairing waiting on them, if one is.
pub fn answer_pairing(answer: Option<String>) {
    if let Some(send) = ANSWER.lock().take() {
        let _ = send.send(answer);
    }
}

/// Forgets a paired device.
pub fn unpair(id: &str) -> bool {
    use windows::Devices::Enumeration::DeviceUnpairingResultStatus;
    ensure_apartment();
    DeviceInformation::CreateFromIdAsync(&windows::core::HSTRING::from(id))
        .and_then(|operation| operation.get())
        .and_then(|information| information.Pairing())
        .and_then(|pairing| pairing.UnpairAsync())
        .and_then(|operation| operation.get())
        .and_then(|result| result.Status())
        .is_ok_and(|status| {
            status == DeviceUnpairingResultStatus::Unpaired
                || status == DeviceUnpairingResultStatus::AlreadyUnpaired
        })
}

/// Connects or disconnects a paired audio device.
///
/// There is no API for this: whether a headset is connected is the stack's
/// decision. What Windows' own Settings does is ask the device's audio
/// driver, through a kernel-streaming property its Bluetooth audio filters
/// answer, to reconnect or let go once. The filters are found among the audio
/// device interfaces by the device's address in their paths.
pub fn connect_audio(id: &str, connect: bool) -> bool {
    use windows::Devices::Bluetooth::BluetoothDevice;
    ensure_apartment();

    let Ok(address) = BluetoothDevice::FromIdAsync(&windows::core::HSTRING::from(id))
        .and_then(|operation| operation.get())
        .and_then(|device| device.BluetoothAddress())
    else {
        return false;
    };
    let address = format!("{address:012x}");

    // `KSCATEGORY_AUDIO` interfaces, present and enabled.
    let selector = "System.Devices.InterfaceClassGuid:=\"{6994AD04-93EF-11D0-A3CC-00A0C9223196}\" \
                    AND System.Devices.InterfaceEnabled:=System.StructuredQueryType.Boolean#True";
    let Ok(interfaces) =
        DeviceInformation::FindAllAsyncAqsFilter(&windows::core::HSTRING::from(selector))
            .and_then(|operation| operation.get())
    else {
        return false;
    };

    let mut answered = false;
    for interface in interfaces {
        let Ok(path) = interface.Id().map(|id| id.to_string()) else {
            continue;
        };
        if path.to_lowercase().contains(&address) {
            answered |= unsafe { ks_one_shot(&path, if connect { 0 } else { 1 }) };
        }
    }
    answered
}

/// Sends `KSPROPERTY_ONESHOT_RECONNECT` (0) or `_DISCONNECT` (1) of
/// `KSPROPSETID_BtAudio` to one kernel-streaming filter.
unsafe fn ks_one_shot(path: &str, property: u32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::Win32::System::IO::DeviceIoControl;

    /// `KSPROPERTY`: which set, which property, and that this is a get.
    #[repr(C)]
    struct KsProperty {
        set: windows::core::GUID,
        id: u32,
        flags: u32,
    }
    const BT_AUDIO: windows::core::GUID =
        windows::core::GUID::from_u128(0x7fa06c40_b8f6_4c7e_8556_e8c33a12e54d);
    const KSPROPERTY_TYPE_GET: u32 = 1;
    // CTL_CODE(FILE_DEVICE_KS, 0, METHOD_NEITHER, FILE_ANY_ACCESS)
    const IOCTL_KS_PROPERTY: u32 = 0x002f_0003;

    let Ok(handle) = CreateFileW(
        &windows::core::HSTRING::from(path),
        GENERIC_READ.0 | GENERIC_WRITE.0,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        None,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        HANDLE::default(),
    ) else {
        return false;
    };
    let request = KsProperty {
        set: BT_AUDIO,
        id: property,
        flags: KSPROPERTY_TYPE_GET,
    };
    let mut returned = 0u32;
    let sent = DeviceIoControl(
        handle,
        IOCTL_KS_PROPERTY,
        Some(std::ptr::addr_of!(request).cast()),
        std::mem::size_of::<KsProperty>() as u32,
        None,
        0,
        Some(std::ptr::addr_of_mut!(returned)),
        None,
    )
    .is_ok();
    let _ = CloseHandle(handle);
    sent
}
