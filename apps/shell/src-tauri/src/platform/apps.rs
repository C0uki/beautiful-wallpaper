//! Every application the machine will admit to having.
//!
//! There is no single list to ask for. Desktop programs are Start-menu
//! shortcuts — `.lnk` files scattered across two folder trees, each one a
//! structured-storage document that has to be opened through COM to find out
//! what it points at — and Store applications are not files at all but
//! packages, reachable only through WinRT. The launcher needs both, so both
//! are read and the results are merged.
//!
//! It is slow enough to matter: a few hundred shortcuts, each a COM object and
//! an icon extraction. So it happens once on a background thread and the
//! answer is kept, with the Start-menu folders watched so that installing
//! something does not require restarting the shell.

use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bw_core::launcher::{AppEntry, AppKind};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use windows::core::{Interface, HSTRING, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    STGM_READ,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

use crate::platform::appicon;

/// How deep to walk the Start menu.
///
/// Vendors nest a folder or two; nothing legitimate goes deeper, and a bound
/// means a symlink loop cannot hang the scan.
const MAX_DEPTH: usize = 6;

/// Installers write a burst of files; this is how long to wait for it to stop.
const RESCAN_DEBOUNCE: Duration = Duration::from_secs(2);

/// The applications, kept current in the background.
pub struct Catalogue {
    apps: Arc<Mutex<Vec<AppEntry>>>,
    /// Dropping this stops the watch, so it is held for the catalogue's life.
    _watcher: Option<RecommendedWatcher>,
}

impl Catalogue {
    /// Starts empty, fills itself, and calls `on_change` whenever it has.
    ///
    /// Returning before the scan finishes is deliberate: the shell starts in
    /// the time it takes to read a config file, and the overview is usable
    /// with open windows alone until the applications arrive.
    pub fn new(on_change: impl Fn() + Send + 'static) -> Self {
        let apps: Arc<Mutex<Vec<AppEntry>>> = Arc::new(Mutex::new(Vec::new()));
        let (sender, receiver) = mpsc::channel::<()>();

        {
            let apps = Arc::clone(&apps);
            std::thread::spawn(move || {
                // WinRT and the shell link both need COM on this thread, and
                // multi-threaded so that waiting on an async package call does
                // not deadlock the apartment.
                unsafe {
                    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                }

                let refresh = || {
                    let found = list();
                    tracing::debug!(count = found.len(), "scanned the installed applications");
                    *apps.lock() = found;
                    on_change();
                };

                refresh();

                let mut last = Instant::now();
                for () in receiver {
                    if last.elapsed() < RESCAN_DEBOUNCE {
                        continue;
                    }
                    // An install is a burst of writes, and rescanning on the
                    // first one finds a half-populated Start menu.
                    std::thread::sleep(RESCAN_DEBOUNCE);
                    last = Instant::now();
                    refresh();
                }
            });
        }

        let mut watcher = notify::recommended_watcher(move |result: notify::Result<_>| {
            if result.is_ok() {
                let _ = sender.send(());
            }
        })
        .ok();

        if let Some(watcher) = watcher.as_mut() {
            for root in start_menu_roots() {
                if let Err(error) = watcher.watch(&root, RecursiveMode::Recursive) {
                    tracing::debug!(%error, path = %root.display(), "not watching a Start menu folder");
                }
            }
        }

        Self {
            apps,
            _watcher: watcher,
        }
    }

    pub fn items(&self) -> Vec<AppEntry> {
        self.apps.lock().clone()
    }
}

/// Scans everything, once. Blocking, and slow enough to keep off the UI thread.
pub fn list() -> Vec<AppEntry> {
    let mut found = shortcuts();
    found.extend(packaged());

    // A Store application usually has a Start-menu shortcut as well, and
    // without this the launcher offers the same program twice under the same
    // name with no way to tell which is which.
    let mut seen: Vec<String> = Vec::with_capacity(found.len());
    found.retain(|app| {
        let key = app.name.to_lowercase();
        if seen.contains(&key) {
            return false;
        }
        seen.push(key);
        true
    });

    found.sort_by_key(|app| app.name.to_lowercase());
    found
}

/// The two Start-menu trees: everyone's, and this user's.
fn start_menu_roots() -> Vec<PathBuf> {
    ["ProgramData", "AppData"]
        .iter()
        .filter_map(std::env::var_os)
        .map(|base| {
            PathBuf::from(base)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
        })
        .filter(|root| root.is_dir())
        .collect()
}

fn shortcuts() -> Vec<AppEntry> {
    let mut found = Vec::new();
    for root in start_menu_roots() {
        walk(&root, 0, &mut found);
    }
    found
}

fn walk(directory: &Path, depth: usize, found: &mut Vec<AppEntry>) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, depth + 1, found);
            continue;
        }

        let extension = path
            .extension()
            .map(|value| value.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        match extension.as_str() {
            "lnk" => {
                if let Some(app) = describe_shortcut(&path) {
                    found.push(app);
                }
            }
            "url" => {
                if let Some(app) = describe_internet_shortcut(&path) {
                    found.push(app);
                }
            }
            _ => {}
        }
    }
}

/// What a `.lnk` points at, or `None` if it is not worth offering.
fn describe_shortcut(path: &Path) -> Option<AppEntry> {
    let name = path.file_stem()?.to_string_lossy().into_owned();
    if is_noise(&name) {
        return None;
    }

    let (target, icon_location) = unsafe { read_link(path)? };

    // A shortcut whose target is gone is the residue of an uninstall, and
    // offering it means offering an error message.
    if !target.exists() {
        return None;
    }

    let extension = target
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    // Start menus also carry help files, licence text and web links. Those
    // belong to the documents they are, not to a launcher's list of programs.
    if !matches!(
        extension.as_str(),
        "exe" | "com" | "bat" | "cmd" | "msc" | "cpl"
    ) {
        return None;
    }
    if is_noise(&target.to_string_lossy()) {
        return None;
    }

    // The shortcut's own icon first: an installer that puts several programs
    // in one resource dll distinguishes them only by index, and taking the
    // target's first icon would give them all the same picture.
    let icon = icon_location
        .and_then(|(file, index)| appicon::for_executable_at(&file, index))
        .or_else(|| appicon::for_executable(&target))
        .unwrap_or_default();

    Some(AppEntry {
        name,
        target: path.to_string_lossy().into_owned(),
        kind: AppKind::Shortcut,
        icon,
        subtitle: target.to_string_lossy().into_owned(),
    })
}

/// A `.url` file: a browser link the Start menu happens to carry.
fn describe_internet_shortcut(path: &Path) -> Option<AppEntry> {
    let name = path.file_stem()?.to_string_lossy().into_owned();
    if is_noise(&name) {
        return None;
    }

    // These are plain INI files, so there is nothing to open through COM.
    let contents = std::fs::read_to_string(path).ok()?;
    let url = contents
        .lines()
        .find_map(|line| line.strip_prefix("URL="))?
        .trim()
        .to_owned();
    if url.is_empty() {
        return None;
    }

    Some(AppEntry {
        name,
        target: path.to_string_lossy().into_owned(),
        kind: AppKind::Shortcut,
        icon: String::new(),
        subtitle: url,
    })
}

/// Uninstallers, and the rest of what nobody opens on purpose.
fn is_noise(text: &str) -> bool {
    let lowered = text.to_lowercase();
    ["uninstall", "unins000", "readme", "release notes"]
        .iter()
        .any(|term| lowered.contains(term))
}

/// A shortcut's target, and the icon it names if it names one.
unsafe fn read_link(path: &Path) -> Option<(PathBuf, Option<(PathBuf, i32)>)> {
    let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
    let file: IPersistFile = link.cast().ok()?;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // `Load` rather than `Resolve`: resolving a broken shortcut searches the
    // disk for its target and can put a dialog on screen, neither of which
    // belongs in a background scan.
    file.Load(PCWSTR(wide.as_ptr()), STGM_READ).ok()?;

    let mut buffer = [0u16; 260];
    link.GetPath(&mut buffer, std::ptr::null_mut(), 0).ok()?;
    let target = PathBuf::from(from_wide(&buffer));
    if target.as_os_str().is_empty() {
        return None;
    }

    let mut icon_buffer = [0u16; 260];
    let mut index = 0i32;
    let icon = link
        .GetIconLocation(&mut icon_buffer, &mut index)
        .ok()
        .map(|()| from_wide(&icon_buffer))
        .filter(|found| !found.is_empty())
        .map(|found| (PathBuf::from(found), index));

    Some((target, icon))
}

fn from_wide(buffer: &[u16]) -> String {
    let end = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

/// Store applications, which are packages rather than files.
fn packaged() -> Vec<AppEntry> {
    use windows::Management::Deployment::PackageManager;

    let Ok(manager) = PackageManager::new() else {
        tracing::debug!("no package manager; not listing Store applications");
        return Vec::new();
    };

    // An empty security id means this user, which is the only set a desktop
    // application can enumerate without the `packageQuery` capability.
    let Ok(packages) = manager.FindPackagesByUserSecurityId(&HSTRING::new()) else {
        return Vec::new();
    };

    let mut found = Vec::new();
    for package in packages {
        // Frameworks and resource packages carry no application to launch.
        if package.IsFramework().unwrap_or(false) || package.IsResourcePackage().unwrap_or(false) {
            continue;
        }
        let Ok(entries) = package
            .GetAppListEntriesAsync()
            .and_then(|operation| operation.get())
        else {
            continue;
        };

        for entry in entries {
            if let Some(app) = describe_packaged(&entry) {
                found.push(app);
            }
        }
    }
    found
}

fn describe_packaged(entry: &windows::ApplicationModel::Core::AppListEntry) -> Option<AppEntry> {
    let display = entry.DisplayInfo().ok()?;
    let name = display.DisplayName().ok()?.to_string();
    if name.is_empty() {
        return None;
    }
    let aumid = entry.AppUserModelId().ok()?.to_string();
    if aumid.is_empty() {
        return None;
    }

    let icon = packaged_logo(&display, &aumid).unwrap_or_default();

    Some(AppEntry {
        name,
        target: aumid,
        kind: AppKind::Packaged,
        icon,
        // Nothing here is a path the user would recognise, so this says what
        // kind of thing it is instead.
        subtitle: "Microsoft Store".to_owned(),
    })
}

/// A packaged application's logo, cached as a PNG like every other icon.
fn packaged_logo(display: &windows::ApplicationModel::AppDisplayInfo, key: &str) -> Option<String> {
    use windows::Foundation::Size;
    use windows::Storage::Streams::DataReader;

    let logo = display
        .GetLogo(Size {
            Width: 64.0,
            Height: 64.0,
        })
        .ok()?;
    let stream = logo
        .OpenReadAsync()
        .and_then(|operation| operation.get())
        .ok()?;

    let size = u32::try_from(stream.Size().ok()?).ok()?;
    if size == 0 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader
        .LoadAsync(size)
        .and_then(|operation| operation.get())
        .ok()?;

    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes).ok()?;
    appicon::store_image(key, &bytes)
}
