//! Package identity, and the sparse package that grants it.
//!
//! Most of what a shell does needs no identity at all: WASAPI, the app bar,
//! the tray, `SetWindowRgn`. One thing does. `UserNotificationListener` — the
//! only supported way to read what *other* applications have posted to the
//! Action Center — refuses to hand anything to a process Windows cannot name,
//! and an ordinary installed program is exactly that.
//!
//! A **sparse package** is the way out: an MSIX containing a manifest and
//! nothing else, declaring `AllowExternalContent` and pointing at the folder
//! the program is already installed in. Registering it gives this executable
//! identity while leaving the installation where it is.
//!
//! It has a price, and it is not one this code can pay on the user's behalf:
//! **the package has to be signed, and the certificate has to be trusted by
//! the machine.** That is a decision about the computer rather than about this
//! shell, so nothing here installs a certificate, and every failure below says
//! which step is missing rather than reporting the feature as broken.

use windows::core::{HSTRING, PWSTR};
use windows::Foundation::Uri;
use windows::Management::Deployment::{AddPackageOptions, PackageManager, RemovalOptions};
use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;

/// The `<Identity Name>` in `AppxManifest.xml`.
///
/// The *family* name is this plus a hash of the publisher, which comes from
/// the signing certificate and so is not knowable here — packages are matched
/// on this instead, which is exact enough because nothing else on the machine
/// carries this identity name.
const IDENTITY_NAME: &str = "C0uki.beautiful-wallpaper";

/// `APPMODEL_ERROR_NO_PACKAGE`, from `appmodel.h`.
///
/// Spelled out rather than imported: this is the one answer that means "no
/// identity", and every other error means the question could not be asked.
const NO_PACKAGE: WIN32_ERROR = WIN32_ERROR(15700);

/// Whether this process is running with package identity.
///
/// The documented test: `GetCurrentPackageFullName` answers
/// `APPMODEL_ERROR_NO_PACKAGE` when there is none. Asked rather than
/// remembered, because registering the package does **not** give identity to
/// the process that registered it — identity is decided when a process starts,
/// so the shell has to be restarted before the listener can work.
pub fn has_identity() -> bool {
    let mut length = 0u32;
    // A null buffer with a zero length: the answer that matters is the error
    // code, not the name. `ERROR_INSUFFICIENT_BUFFER` here means there *is* a
    // package and it would not fit — which is a yes.
    let status = unsafe { GetCurrentPackageFullName(&mut length, PWSTR::null()) };
    status != NO_PACKAGE
}

/// Registers the sparse package sitting beside the executable.
///
/// The package declares external content, so Windows also needs to be told
/// where that content is — the folder the shell is installed in.
pub fn register() -> Result<(), String> {
    let package = sparse_package()?;
    let folder = install_folder()?;

    let options = AddPackageOptions::new()
        .map_err(|error| format!("could not prepare the package registration: {error}"))?;
    options
        .SetExternalLocationUri(
            &Uri::CreateUri(&HSTRING::from(folder.to_string_lossy().as_ref())).map_err(
                |error| format!("{} is not a usable location: {error}", folder.display()),
            )?,
        )
        .map_err(|error| format!("could not point the package at the install folder: {error}"))?;

    let manager = PackageManager::new()
        .map_err(|error| format!("could not reach the package manager: {error}"))?;
    let uri = Uri::CreateUri(&HSTRING::from(package.to_string_lossy().as_ref()))
        .map_err(|error| format!("{} is not a usable path: {error}", package.display()))?;

    let operation = manager
        .AddPackageByUriAsync(&uri, &options)
        .map_err(|error| format!("could not register the package: {error}"))?;
    let result = operation
        .get()
        .map_err(|error| format!("registering the package did not finish: {error}"))?;

    let code = result
        .ExtendedErrorCode()
        .unwrap_or_else(|error| error.code());
    if code.is_err() {
        // Nearly always the certificate: unsigned, or signed by one the
        // machine does not trust. Saying so beats the raw HRESULT, which sends
        // people looking for a bug in the shell.
        let text = result.ErrorText().unwrap_or_default().to_string_lossy();
        return Err(format!(
            "Windows refused the package ({code:?}). This usually means its certificate is not \
             installed as a trusted root on this machine. {text}"
        ));
    }

    Ok(())
}

/// Removes the sparse package again.
///
/// Leaving it behind would keep granting identity to an executable that may no
/// longer be there, and the point of switching the feature off is that nothing
/// of it remains.
pub fn unregister() -> Result<(), String> {
    let manager = PackageManager::new()
        .map_err(|error| format!("could not reach the package manager: {error}"))?;

    // The current user's packages, which needs no elevation — `FindPackages`
    // covers every user on the machine and does.
    let packages = manager
        .FindPackagesByUserSecurityId(&HSTRING::new())
        .map_err(|error| format!("could not look for the package: {error}"))?;

    for package in packages {
        let Ok(id) = package.Id() else { continue };
        if id.Name().is_ok_and(|name| name == IDENTITY_NAME) {
            let full_name = id
                .FullName()
                .map_err(|error| format!("could not read the package's name: {error}"))?;

            manager
                .RemovePackageWithOptionsAsync(&full_name, RemovalOptions::None)
                .map_err(|error| format!("could not remove the package: {error}"))?
                .get()
                .map_err(|error| format!("removing the package did not finish: {error}"))?;
        }
    }

    Ok(())
}

fn install_folder() -> Result<std::path::PathBuf, String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("could not find this executable: {error}"))?;
    exe.parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| "this executable has no folder".to_owned())
}

/// Where the installer put the sparse package.
fn sparse_package() -> Result<std::path::PathBuf, String> {
    let path = install_folder()?.join("beautiful-wallpaper-sparse.msix");
    if !path.exists() {
        return Err(format!(
            "{} is not there. The sparse package is built and signed separately — see \
             docs/msix.md — because signing needs a certificate this build cannot make for you.",
            path.display()
        ));
    }
    Ok(path)
}
