//! Starting things.
//!
//! Three different mechanisms, because Windows has three different kinds of
//! thing to start. A shortcut is a file and opens like one. A packaged
//! application is not a file at all and has to be activated through COM by its
//! application user model id. A typed command line is neither, and has to be
//! taken apart into a program and its arguments first.
//!
//! What they share is that all three can be refused, and a launcher that
//! swallows the refusal leaves the user pressing Enter at a list that does
//! nothing. Every path here reports why it failed.

use std::os::windows::ffi::OsStrExt;

use bw_core::launcher::AppKind;
use windows::core::PCWSTR;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_LOCAL_SERVER};
use windows::Win32::UI::Shell::{
    ApplicationActivationManager, IApplicationActivationManager, ShellExecuteW, AO_NONE,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// Starts an application the launcher offered.
pub fn app(target: &str, kind: AppKind) -> Result<(), String> {
    match kind {
        AppKind::Shortcut => tauri_plugin_opener::open_path(target, None::<&str>)
            .map_err(|error| format!("could not start {target}: {error}")),
        AppKind::Packaged => packaged(target),
    }
}

/// Activates a packaged application by its application user model id.
///
/// There is no file to open — the package is not laid out as one — so this
/// goes through the activation manager, which is what Explorer itself uses.
fn packaged(aumid: &str) -> Result<(), String> {
    let wide = wide(aumid);
    unsafe {
        let manager: IApplicationActivationManager =
            CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_LOCAL_SERVER)
                .map_err(|error| format!("no activation manager: {error}"))?;

        manager
            .ActivateApplication(PCWSTR(wide.as_ptr()), PCWSTR::null(), AO_NONE)
            .map(|_process_id| ())
            .map_err(|error| format!("could not start {aumid}: {error}"))
    }
}

/// Runs a command line the way the Run dialog does.
pub fn command(line: &str) -> Result<(), String> {
    let (program, arguments) = bw_core::launcher::split_command(line)
        .ok_or_else(|| format!("`{line}` is not something to run"))?;

    let program_wide = wide(&program);
    let arguments_wide = wide(&arguments);

    let result = unsafe {
        ShellExecuteW(
            None,
            windows::core::w!("open"),
            PCWSTR(program_wide.as_ptr()),
            if arguments.is_empty() {
                PCWSTR::null()
            } else {
                PCWSTR(arguments_wide.as_ptr())
            },
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    // `ShellExecuteW` returns a fake instance handle, and anything at or below
    // 32 is an error code rather than a handle. It is the only signal there
    // is: the call does not set a last error worth reading.
    if result.0 as usize > 32 {
        return Ok(());
    }
    Err(match result.0 as usize {
        2 => format!("`{program}` was not found"),
        3 => format!("the folder for `{program}` was not found"),
        5 => format!("`{program}` refused to start"),
        8 => "not enough memory to start it".to_owned(),
        31 => format!("nothing on this machine opens `{program}`"),
        code => format!("could not run `{program}` (error {code})"),
    })
}

/// Opens a protocol URI — `ms-settings:display` and the like.
///
/// Not the file opener: there is no file. `ShellExecuteW` resolves the scheme
/// through the same registry lookup Explorer uses, which is what makes the
/// Settings app open on the right page rather than at its front door.
pub fn uri(uri: &str) -> Result<(), String> {
    let wide_uri = wide(uri);
    let result = unsafe {
        ShellExecuteW(
            None,
            windows::core::w!("open"),
            PCWSTR(wide_uri.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    // Same contract as `command`: at or below 32 is an error code, not a handle.
    if result.0 as usize > 32 {
        return Ok(());
    }
    Err(match result.0 as usize {
        // The page the shell asked for is not one this Windows has — the
        // settings app was reorganised more than once.
        2 | 31 => format!("this version of Windows has no `{uri}` page"),
        5 => format!("Windows refused to open `{uri}`"),
        code => format!("could not open `{uri}` (error {code})"),
    })
}

fn wide(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
