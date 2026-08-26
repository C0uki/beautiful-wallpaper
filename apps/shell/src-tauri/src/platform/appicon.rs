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
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

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
pub fn for_executable_at(executable: &std::path::Path, index: i32) -> Option<String> {
    let target = cache_path(&format!("{}#{index}", executable.to_string_lossy()))?;
    if target.exists() {
        return Some(target.to_string_lossy().into_owned());
    }

    let pixels = unsafe { rasterise(executable, index)? };
    let image: image::RgbaImage = image::ImageBuffer::from_raw(ICON_SIZE, ICON_SIZE, pixels)?;
    image.save(&target).ok()?;
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

/// Reads an icon's colour bitmap as straight RGBA.
unsafe fn icon_pixels(icon: HICON) -> Option<Vec<u8>> {
    let mut info = ICONINFO::default();
    GetIconInfo(icon, &mut info).ok()?;

    // Both bitmaps are ours to free once we are done with them, whatever
    // happens below.
    let colour = info.hbmColor;
    let mask = info.hbmMask;
    let pixels = read_bitmap(colour);
    if !colour.is_invalid() {
        let _ = DeleteObject(colour);
    }
    if !mask.is_invalid() {
        let _ = DeleteObject(mask);
    }
    pixels
}

/// Pulls a GDI bitmap's pixels out as RGBA, at [`ICON_SIZE`].
unsafe fn read_bitmap(bitmap: HBITMAP) -> Option<Vec<u8>> {
    if bitmap.is_invalid() {
        return None;
    }

    let dc = CreateCompatibleDC(None);
    if dc.is_invalid() {
        return None;
    }

    let mut header = BITMAPINFO {
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

    let mut buffer = vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize];
    let copied = GetDIBits(
        dc,
        bitmap,
        0,
        ICON_SIZE,
        Some(buffer.as_mut_ptr().cast()),
        &mut header,
        DIB_RGB_COLORS,
    );
    let _ = DeleteDC(dc);

    if copied == 0 {
        return None;
    }

    // GDI hands back BGRA; every consumer of this wants RGBA.
    for pixel in buffer.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    Some(buffer)
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
