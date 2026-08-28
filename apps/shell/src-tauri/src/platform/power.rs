//! Ending the session, one way or another.
//!
//! Two of these need a privilege the process does not start with. `SeShutdown`
//! is *present* in an ordinary user's token and *disabled*, so restarting or
//! shutting down means enabling it first — and the enabling is where the trap
//! is. **`AdjustTokenPrivileges` reports success when it granted nothing at
//! all**; the only way to find out is to read the last error and look for
//! `ERROR_NOT_ALL_ASSIGNED`. Skip that and the shell believes it has a
//! privilege it does not, and the shutdown fails later with nothing to
//! explain it.
//!
//! Nothing here forces by default. A shutdown that closes programs without
//! letting them save is a data-loss button dressed as a convenience; without
//! forcing, an unsaved document stops the shutdown and Windows says which
//! program is holding it up, which is the useful outcome.

use bw_core::session::{PowerCapabilities, SessionAction};
use windows::Win32::Foundation::{GetLastError, ERROR_NOT_ALL_ASSIGNED, HANDLE, LUID};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::Power::{
    GetPwrCapabilities, SetSuspendState, SYSTEM_POWER_CAPABILITIES,
};
use windows::Win32::System::Shutdown::{
    ExitWindowsEx, InitiateShutdownW, LockWorkStation, EWX_FORCE, EWX_LOGOFF,
    SHTDN_REASON_FLAG_PLANNED, SHTDN_REASON_MAJOR_OTHER, SHTDN_REASON_MINOR_OTHER, SHUTDOWN_FLAGS,
    SHUTDOWN_FORCE_OTHERS, SHUTDOWN_FORCE_SELF, SHUTDOWN_POWEROFF, SHUTDOWN_REASON,
    SHUTDOWN_RESTART,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// What the event log records: a person asked for this, nothing crashed.
const REASON_PLANNED_BY_USER: SHUTDOWN_REASON = SHUTDOWN_REASON(
    SHTDN_REASON_MAJOR_OTHER.0 | SHTDN_REASON_MINOR_OTHER.0 | SHTDN_REASON_FLAG_PLANNED.0,
);

/// What this machine can be asked to do.
pub fn capabilities() -> PowerCapabilities {
    let mut found = SYSTEM_POWER_CAPABILITIES::default();
    let read = unsafe { GetPwrCapabilities(&mut found) };
    if !read.as_bool() {
        // Nothing is claimed rather than everything: an unanswerable machine
        // gets the buttons that always work and none of the ones that might
        // not.
        return PowerCapabilities::default();
    }

    PowerCapabilities {
        standby: found.SystemS1.as_bool() || found.SystemS2.as_bool() || found.SystemS3.as_bool(),
        modern_standby: found.AoAc.as_bool(),
        hibernate_file: found.HiberFilePresent.as_bool(),
    }
}

/// Takes the action, or says why it could not.
pub fn run(action: SessionAction, force: bool) -> Result<(), String> {
    match action {
        SessionAction::Lock => unsafe { LockWorkStation() }
            .map_err(|error| format!("could not lock the screen: {error}")),

        // Logging off needs no privilege; it only affects this session.
        SessionAction::LogOut => unsafe {
            let flags = if force {
                EWX_LOGOFF | EWX_FORCE
            } else {
                EWX_LOGOFF
            };
            ExitWindowsEx(flags, REASON_PLANNED_BY_USER)
        }
        .map_err(|error| format!("could not log out: {error}")),

        SessionAction::Sleep => suspend(false),
        SessionAction::Hibernate => suspend(true),

        SessionAction::Restart => shutdown(SHUTDOWN_RESTART, force, "restart"),
        SessionAction::ShutDown => shutdown(SHUTDOWN_POWEROFF, force, "shut down"),
    }
}

fn suspend(hibernate: bool) -> Result<(), String> {
    // The second argument is documented as having no effect since Windows XP;
    // the third leaves wake events alone.
    let went = unsafe { SetSuspendState(hibernate, false, false) };
    if went.as_bool() {
        return Ok(());
    }
    Err(if hibernate {
        "this machine will not hibernate".to_owned()
    } else {
        "this machine will not sleep".to_owned()
    })
}

/// Restarts or powers off, having first asked for the privilege to do so.
fn shutdown(what: SHUTDOWN_FLAGS, force: bool, verb: &str) -> Result<(), String> {
    enable_shutdown_privilege()?;

    let flags = if force {
        // Both, deliberately: forcing other people's applications and not the
        // shell's own would leave this process as the one thing blocking the
        // shutdown the user just asked for.
        what | SHUTDOWN_FORCE_OTHERS | SHUTDOWN_FORCE_SELF
    } else {
        what
    };

    // `InitiateShutdownW` rather than `ExitWindowsEx`: it returns the error
    // code directly instead of through `GetLastError`, and a zero grace
    // period means "now" without the countdown dialog.
    let result = unsafe { InitiateShutdownW(None, None, 0, flags, REASON_PLANNED_BY_USER) };
    if result == 0 {
        return Ok(());
    }

    Err(match result {
        // ERROR_SHUTDOWN_IS_SCHEDULED
        1190 => format!("a {verb} is already under way"),
        // ERROR_PRIVILEGE_NOT_HELD
        1314 => format!("this account is not allowed to {verb} the machine"),
        // ERROR_SHUTDOWN_USERS_LOGGED_ON
        1191 => format!("somebody else is logged in, so the {verb} was refused"),
        code => format!("could not {verb} the machine (error {code})"),
    })
}

/// Turns on `SeShutdownPrivilege` for this process.
///
/// The privilege is in the token already and switched off; this switches it
/// on. It has to be done once per process, and doing it twice is harmless.
fn enable_shutdown_privilege() -> Result<(), String> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
        .map_err(|error| format!("could not read this process's privileges: {error}"))?;

        let mut id = LUID::default();
        let looked_up = LookupPrivilegeValueW(None, SE_SHUTDOWN_NAME, &mut id);

        let adjusted = looked_up.and_then(|()| {
            let privileges = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES {
                    Luid: id,
                    Attributes: SE_PRIVILEGE_ENABLED,
                }],
            };
            AdjustTokenPrivileges(token, false, Some(&privileges), 0, None, None)
        });

        // Read before the handle goes: closing it would replace the error.
        let granted_everything = GetLastError() != ERROR_NOT_ALL_ASSIGNED;
        let _ = windows::Win32::Foundation::CloseHandle(token);

        adjusted.map_err(|error| format!("could not ask for the shutdown privilege: {error}"))?;

        // **The call above succeeds even when it granted nothing.** Without
        // this check the shell would believe it holds a privilege it does
        // not, and the shutdown would fail later with nothing to explain it.
        if !granted_everything {
            return Err(
                "this account does not have permission to shut the machine down".to_owned(),
            );
        }
        Ok(())
    }
}
