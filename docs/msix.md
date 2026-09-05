# The MSIX sparse package

This exists for exactly one feature: **showing notifications posted by other
applications**. Everything else in the shell works without it, and nothing here
is needed unless you want the notification centre to show more than what the
shell itself posts.

It is off by default, under `hacks.readOtherNotifications`, and it is in
`hacks` for the same reason the desktop menu is: it reaches past what Windows
offers an ordinary program, and the reaching has a cost you should decide about
rather than have decided for you.

## Why a package is involved at all

`UserNotificationListener` is the only supported way to read the Action Center.
Windows will not hand anything to a process it cannot name, and "cannot name"
means **no package identity** — which is the normal state of a program
installed from an `.exe`.

A **sparse package** is the documented way out. It is an MSIX containing a
manifest and three logos and nothing else; it declares
`<uap10:AllowExternalContent>`, and when it is registered you tell Windows
where the real files are. The shell stays installed where it was. It simply
gains a name Windows recognises.

## The part that is a decision about your machine

**The package has to be signed, and the certificate has to be trusted by the
machine.** There is no way around it: Windows refuses an unsigned package and
refuses one signed by a certificate it does not trust.

For a package you build yourself, that means installing your own certificate
into the machine's **Trusted Root Certification Authorities** store. Be clear
about what that is: a root certificate can vouch for _anything_, not just this
package. Installing one you made and keep to yourself is a normal thing to do
on your own machine; installing one somebody sent you is not, and neither is
leaving one behind when you are done with it.

**This repository ships no certificate and installs nothing.** The build script
takes one you already have. If you would rather not do any of this, leave the
setting off — the shell's own notifications work either way.

## Building it

From `apps/shell/src-tauri/msix`, with the Windows SDK available:

```powershell
.\build.ps1 -Certificate .\mine.pfx -Password (Read-Host -AsSecureString)
```

The script writes the certificate's subject into the manifest's `Publisher`
field, because Windows requires the two to match exactly and the error it gives
when they do not says nothing about which one is wrong.

A certificate for testing, if you have none:

```powershell
$cert = New-SelfSignedCertificate -Type Custom -Subject "CN=Your Name Here" `
  -KeyUsage DigitalSignature -FriendlyName "beautiful-wallpaper (test)" `
  -CertStoreLocation "Cert:\CurrentUser\My" `
  -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
Export-PfxCertificate -Cert $cert -FilePath mine.pfx -Password (Read-Host -AsSecureString)
```

Trusting it needs an administrator, and this is the step to be deliberate
about:

```powershell
Import-Certificate -FilePath mine.cer -CertStoreLocation Cert:\LocalMachine\Root
```

## Using it

1. Put `beautiful-wallpaper-sparse.msix` beside `bw.exe` in the install folder.
2. Switch on `hacks.readOtherNotifications` — in Settings, or with
   `bw config set hacks.readOtherNotifications true`.
3. **Restart the shell.** Windows decides a process's identity when it starts,
   so the process that registered the package does not have identity itself.
   Until the restart the shell will tell you exactly that.
4. Windows will ask, once, whether this application may read notifications.
   Answer it. If it was answered before, Settings › Privacy & security ›
   Notifications is where it is changed.

Switching the setting off removes the package again.

## When it does not work

The shell posts a notification saying which of the steps is missing, rather
than going quiet. The four it distinguishes:

| What it says                                                   | What is missing                                  |
| -------------------------------------------------------------- | ------------------------------------------------ |
| needs the sparse package registered, and a restart             | no package identity yet                          |
| this Windows does not offer the notification listener          | older than 10.0.19041, or the listener is absent |
| Windows has not been given an answer about notification access | nobody has allowed or refused it yet             |
| Windows is set to refuse this shell access                     | it was refused; only you can undo that           |

## What is not built

- **Removing the package on uninstall.** The uninstaller undoes the taskbar,
  the Run entry and the App Paths key; it does not unregister this, because
  doing so needs the package manager rather than a registry write. Switching
  the setting off before uninstalling is the tidy path, and a leftover sparse
  package pointing at a folder that is gone is inert rather than harmful.
- **A signed package in the releases.** Signing in CI needs a certificate held
  as a secret, and this project does not have one.
- **Sending notifications _as_ an application with identity.** The identity
  would allow it; nothing here uses it for that yet.
