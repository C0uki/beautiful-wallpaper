//! Registering the shell to start with Windows.
//!
//! The per-user Run key, because the installer's default is a per-user
//! install: `HKCU` needs no elevation, follows the user to whichever machine
//! their profile roams to, and is undone by deleting one value.
//!
//! What goes *in* the value — the quoting, and whether an entry that is
//! already there is still ours — is in `bw_core::autostart` under tests, since
//! it is where the failures are and none of them need a registry to reproduce.

use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, MAX_PATH};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
};

/// Where Windows looks for what to start at login.
const RUN_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");

/// The value name. Not the executable name: an entry called `bw` in a list
/// beside Steam and OneDrive says nothing about what it is.
const VALUE: PCWSTR = w!("beautiful-wallpaper");

/// This process's own path, which is what an entry has to point at.
fn executable() -> Result<String, String> {
    std::env::current_exe()
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| format!("could not find this executable: {error}"))
}

/// Whether the shell is registered to start with Windows.
///
/// An entry that points somewhere else counts as *not* registered: it is a
/// leftover from an installation that has moved, and reporting it as on would
/// leave the user with a switch that is lying and a login that errors.
pub fn is_enabled() -> bool {
    let Ok(exe) = executable() else {
        return false;
    };
    read().is_some_and(|entry| bw_core::autostart::is_ours(&entry, &exe))
}

/// Adds or removes the entry so it matches `wanted`.
///
/// Writing it again when it is already right is deliberate: the path changes
/// when the shell is reinstalled elsewhere, and rewriting is how a stale entry
/// stops being an error dialog at every login.
pub fn apply(wanted: bool) -> Result<(), String> {
    if !wanted {
        return remove();
    }
    write(&bw_core::autostart::command_line(&executable()?))
}

fn open(access: u32) -> Result<HKEY, String> {
    let mut key = HKEY::default();
    // The Run key exists on every Windows installation, so this is opened
    // rather than created: failing here means something is wrong with the
    // hive, not that the shell has to make it.
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            0,
            windows::Win32::System::Registry::REG_SAM_FLAGS(access),
            &mut key,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!("could not open the Run key: error {}", status.0));
    }
    Ok(key)
}

fn read() -> Option<String> {
    let key = open(KEY_READ.0).ok()?;
    // Room for a full path plus the two quotes, in UTF-16 units.
    let mut buffer = [0u16; MAX_PATH as usize + 2];
    let mut size = std::mem::size_of_val(&buffer) as u32;
    let mut kind = REG_SZ;

    let status = unsafe {
        RegQueryValueExW(
            key,
            VALUE,
            None,
            Some(&mut kind),
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    unsafe {
        let _ = RegCloseKey(key);
    }

    if status != ERROR_SUCCESS {
        return None;
    }

    // `size` is bytes and includes the terminator when the writer stored one,
    // which not every writer does.
    let units = (size as usize / 2).min(buffer.len());
    let text: String = String::from_utf16_lossy(&buffer[..units]);
    Some(text.trim_end_matches('\0').to_owned())
}

fn write(value: &str) -> Result<(), String> {
    let key = open(KEY_WRITE.0)?;
    let wide = HSTRING::from(value);
    // The terminator is included: `RegSetValueExW` stores exactly the bytes it
    // is given, and a `REG_SZ` without one is what makes other readers run off
    // the end of the value.
    let bytes = unsafe {
        std::slice::from_raw_parts(
            wide.as_ptr().cast::<u8>(),
            (wide.len() + 1) * std::mem::size_of::<u16>(),
        )
    };

    let status = unsafe { RegSetValueExW(key, VALUE, 0, REG_SZ, Some(bytes)) };
    unsafe {
        let _ = RegCloseKey(key);
    }

    if status != ERROR_SUCCESS {
        return Err(format!(
            "could not register the shell to start with Windows: error {}",
            status.0
        ));
    }
    Ok(())
}

fn remove() -> Result<(), String> {
    let key = open(KEY_WRITE.0)?;
    let status = unsafe { RegDeleteValueW(key, VALUE) };
    unsafe {
        let _ = RegCloseKey(key);
    }

    // Already gone is the state that was asked for, not a failure.
    if status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND {
        return Err(format!(
            "could not stop the shell starting with Windows: error {}",
            status.0
        ));
    }
    Ok(())
}
