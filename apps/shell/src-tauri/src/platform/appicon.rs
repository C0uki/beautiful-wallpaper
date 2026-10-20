//! Names and icons for a running process.
//!
//! The volume mixer, and later the dock, need to show an application rather
//! than a process id. Windows will hand over both, but the icon comes back as
//! an `HICON` — a GDI handle, not an image — so it has to be drawn into a
//! bitmap and saved before anything in a webview can display it.
//!
//! Icons are cached on disk as PNGs under the shell's cache directory and
//! served through the asset protocol, exactly as wallpaper thumbnails are.
//! That avoids base64-inlining an image into every event payload, and means an
//! icon is extracted once per executable rather than once per refresh.

use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL, HICON};

/// The size icons are rasterised at. Large enough for the mixer's 36px rows on
/// a 200% display, small enough that caching them all costs nothing.
const ICON_SIZE: u32 = 64;

/// A display name and an icon path for a process.
///
/// Both degrade to something usable: an unreadable process still gets its
/// process id as a name, and a missing icon is an empty string that the
/// frontend renders as a generic glyph.
pub fn describe_process(process_id: u32) -> (String, String) {
    let Some(executable) = executable_path(process_id) else {
        return (format!("PID {process_id}"), String::new());
    };

    let name = executable
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("PID {process_id}"));

    let icon = for_executable(&executable).unwrap_or_default();
    (name, icon)
}

/// The full path of a running process's executable, as a string.
///
/// The dock identifies applications by this, so it is public; everything else
/// here only wants the icon.
pub fn executable_for(process_id: u32) -> Option<String> {
    executable_path(process_id).map(|path| path.to_string_lossy().into_owned())
}

/// The full path of a running process's executable.
///
/// `PROCESS_QUERY_LIMITED_INFORMATION` is deliberate: it works for processes
/// at a higher integrity level, where the fuller access right is refused, and
/// the shell only ever wants the path.
fn executable_path(process_id: u32) -> Option<PathBuf> {
    if process_id == 0 {
        return None;
    }

    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;

        let mut buffer = [0u16; MAX_PATH as usize];
        let mut length = buffer.len() as u32;
        let queried = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut length,
        );
        let _ = CloseHandle(process);

        queried.ok()?;
        let path = String::from_utf16_lossy(&buffer[..length as usize]);
        (!path.is_empty()).then(|| PathBuf::from(path))
    }
}

/// The PNG for a file's icon, extracting it on first use.
///
/// The launcher wants this for the executable a Start-menu shortcut points
/// at, which is why it takes a path rather than a process id.
pub fn for_executable(executable: &std::path::Path) -> Option<String> {
    for_executable_at(executable, 0)
}

/// The PNG for one particular icon inside a file.
///
/// A shortcut may name an icon by file and index — an installer that packs
/// several into one resource dll — and taking index zero would give every one
/// of them the same picture.
///
/// Keyed by when the file last changed as well as by its path: Chrome rewrites
/// a profile's icon in place when its picture changes, and an application
/// update does the same to its executable, so the path alone kept showing the
/// picture from the first time it was seen.
pub fn for_executable_at(executable: &std::path::Path, index: i32) -> Option<String> {
    let modified = std::fs::metadata(executable)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_secs());
    let target = cache_path(&format!(
        "{}#{index}#{modified}",
        executable.to_string_lossy()
    ))?;
    if target.exists() {
        return Some(target.to_string_lossy().into_owned());
    }

    let pixels = unsafe { rasterise(executable, index)? };
    let image: image::RgbaImage = image::ImageBuffer::from_raw(ICON_SIZE, ICON_SIZE, pixels)?;
    image.save(&target).ok()?;
    Some(target.to_string_lossy().into_owned())
}

/// The PNG for an icon we hold a handle to, named after its pixels.
///
/// The notification area hands over `HICON`s rather than paths, and a handle
/// is not an identity: an application changes its icon by sending a new one.
/// Named after what it looks like, a changed icon is a new file the bar
/// cannot show a stale copy of, and an animation's frames are each written
/// once rather than on every turn.
pub fn for_hicon(icon: HICON) -> Option<String> {
    let pixels = unsafe { icon_pixels(icon)? };
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in &pixels {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    let target = cache_path(&format!("hicon#{hash:016x}"))?;
    if !target.exists() {
        let image: image::RgbaImage = image::ImageBuffer::from_raw(ICON_SIZE, ICON_SIZE, pixels)?;
        image.save(&target).ok()?;
    }
    Some(target.to_string_lossy().into_owned())
}

/// Puts an already-encoded image into the same cache, under `key`.
///
/// Packaged applications hand over a logo as a stream rather than an `HICON`,
/// so those bytes come in here instead of through the GDI path. They are
/// decoded and re-encoded rather than written through: the stream is a PNG in
/// practice, but nothing documents that it has to be.
pub fn store_image(key: &str, bytes: &[u8]) -> Option<String> {
    let target = cache_path(key)?;
    if target.exists() {
        return Some(target.to_string_lossy().into_owned());
    }

    let decoded = image::load_from_memory(bytes).ok()?;
    decoded.save(&target).ok()?;
    Some(target.to_string_lossy().into_owned())
}

/// Everything in a WinRT stream — a packaged logo, a track's artwork — read
/// into memory. Nothing, for an empty or unreadable one.
pub fn stream_bytes(
    stream: &windows::Storage::Streams::IRandomAccessStreamWithContentType,
) -> Option<Vec<u8>> {
    use windows::Storage::Streams::DataReader;

    let size = u32::try_from(stream.Size().ok()?).ok()?;
    if size == 0 {
        return None;
    }
    let reader = DataReader::CreateDataReader(stream).ok()?;
    reader
        .LoadAsync(size)
        .and_then(|operation| operation.get())
        .ok()?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes).ok()?;
    Some(bytes)
}

/// The playing track's artwork, saved as a PNG named for the track so a new
/// track is a new address and the webview cannot show the last one from its
/// cache. Only the current track's is kept: the folder is emptied of every
/// other file on each new one.
pub fn store_artwork(track: &str, bytes: &[u8]) -> Option<String> {
    let folder = bw_core::paths::cache_dir().join("artwork");
    std::fs::create_dir_all(&folder).ok()?;
    let target = folder.join(format!("{}.png", hash_key(track)));
    if !target.exists() {
        image::load_from_memory(bytes).ok()?.save(&target).ok()?;
    }
    for entry in std::fs::read_dir(&folder).ok()?.flatten() {
        if entry.path() != target {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    Some(target.to_string_lossy().into_owned())
}

/// Where a cache key's PNG lives, creating the directory on the way.
fn cache_path(key: &str) -> Option<std::path::PathBuf> {
    let cache = bw_core::paths::cache_dir().join("appIcons");
    std::fs::create_dir_all(&cache).ok()?;
    Some(cache.join(format!("{}.png", hash_key(key))))
}

/// Draws one of a file's icons into RGBA pixels.
unsafe fn rasterise(executable: &std::path::Path, index: i32) -> Option<Vec<u8>> {
    let wide: Vec<u16> = executable
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut large = HICON::default();
    // The large icon is what a 64px raster wants; asking for the small one and
    // scaling up looks exactly as bad as it sounds.
    let extracted = ExtractIconExW(PCWSTR(wide.as_ptr()), index, Some(&mut large), None, 1);
    if extracted == 0 || large.is_invalid() {
        return None;
    }

    let pixels = icon_pixels(large);
    let _ = DestroyIcon(large);
    pixels
}

/// Draws an icon into RGBA pixels at [`ICON_SIZE`], whatever size it really is.
///
/// `DrawIconEx` rather than `GetDIBits` off the colour bitmap, for two reasons
/// that both show up as a broken picture rather than an error:
///
///   * it scales. `GetDIBits` does not — asking it for 64 rows of a 32×32 icon
///     reads the rows it has against the wrong stride and garbles the result,
///     and a notification-area icon is 16×16 or 32×32 far more often than 64.
///   * it composites the mask. The colour bitmap on its own has no
///     transparency, so every icon with a cut-out came back on a black square.
unsafe fn icon_pixels(icon: HICON) -> Option<Vec<u8>> {
    if icon.is_invalid() {
        return None;
    }

    let dc = CreateCompatibleDC(None);
    if dc.is_invalid() {
        return None;
    }

    let header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: ICON_SIZE as i32,
            // Negative height asks for a top-down bitmap; the default is
            // bottom-up, which would deliver the icon upside down.
            biHeight: -(ICON_SIZE as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
    let Ok(bitmap) = CreateDIBSection(dc, &header, DIB_RGB_COLORS, &mut bits, None, 0) else {
        let _ = DeleteDC(dc);
        return None;
    };

    let previous = SelectObject(dc, HGDIOBJ::from(bitmap));
    let drawn = DrawIconEx(
        dc,
        0,
        0,
        icon,
        ICON_SIZE as i32,
        ICON_SIZE as i32,
        0,
        None,
        DI_NORMAL,
    );
    SelectObject(dc, previous);

    let pixels = drawn.is_ok().then(|| {
        let count = (ICON_SIZE * ICON_SIZE * 4) as usize;
        let mut buffer = std::slice::from_raw_parts(bits.cast::<u8>(), count).to_vec();
        // GDI hands back BGRA; every consumer of this wants RGBA.
        for pixel in buffer.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
        buffer
    });

    let _ = DeleteObject(HGDIOBJ::from(bitmap));
    let _ = DeleteDC(dc);
    pixels
}

/// FNV-1a over the lowercased key: not cryptographic, but stable across runs,
/// which is all a cache needs. The same choice the thumbnail cache makes.
fn hash_key(key: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in key.to_lowercase().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{hash:016x}")
}
