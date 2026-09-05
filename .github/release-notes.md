## Installing

Download **`beautiful-wallpaper_<version>_x64-setup.exe`** and run it. The `.msi`
is the same shell for anyone who prefers it or deploys by policy.

It installs for the current user only, so it does not ask for an administrator.

## Windows will warn you, and it is right to

**These installers are not code-signed**, so SmartScreen shows _"Windows
protected your PC"_ and hides the Run button behind **More info**. That warning
is not about this shell specifically — it is what Windows says about any
installer whose publisher it cannot verify, and it is the correct default.

Signing needs a code-signing certificate, which is a purchase and an identity
check, and this project does not have one. If you would rather not run an
unsigned installer, [build it yourself](https://github.com/C0uki/beautiful-wallpaper/blob/main/README.md#install) — `pnpm install`
then `pnpm --filter @bw/shell app:build` produces the same bundle from source
you can read.

## What it needs

- **Windows 10 or 11.**
- **[WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)**, which
  Windows 11 already has and Windows 10 usually does. The installer does not
  bundle it.

## First run

The first-run screen walks through the wallpaper, the bar, the Windows
integration and the keys — including which of your key combinations Windows
refused to hand over, and what is free instead. `bw wizard open` reaches it
again later.

## If something does not work

Most of what people hit is not a bug but Windows declining to offer something:
an empty tray, brightness that will not move, no workspaces without a tiling
window manager. [docs/troubleshooting.md](https://github.com/C0uki/beautiful-wallpaper/blob/main/docs/troubleshooting.md) covers
those, including the one that strands you — the taskbar hidden with no shell
left to restore it, which `bw taskbar show` undoes from Task Manager's **Run new
task**.

Every setting is in [docs/config.md](https://github.com/C0uki/beautiful-wallpaper/blob/main/docs/config.md).
