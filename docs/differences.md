# What is different from end4-pC

This is a rebuild, not a port. [end4-pC](https://github.com/pctrade/end4-pC) is
a Quickshell configuration — QML that talks to Hyprland over its IPC socket and
places every panel through `wlr-layer-shell`. None of that exists on Windows, so
the design carries over and the code does not.

The [README](../README.md#why-this-is-a-rewrite-not-a-port) has the mechanism
table — which Win32 API stands in for which Wayland protocol. This page is about
what you will notice using it, and what still means the same thing.

The lists of things not built and things half-built are in
[the roadmap](roadmap.md#deliberately-not-built).

## What you will notice

### The keys are not the same

Windows keeps a set of chords for itself and refuses to register them. Most
bindings are `Super+Shift+<letter>` as upstream, but three had to move, and one
moved on purpose:

- **Capture is on the `Print` key** — `Print`, `Ctrl+Print`, `Shift+Print` — and
  not `Win+Shift+S`, which belongs to the Snipping Tool and cannot be taken.
- **Settings is `Super+Shift+I`**, after `Win+I`, for the same reason:
  `Win+Shift+S` was the obvious choice and is not available.
- **The desktop menu is `Super+Shift+X`**, after `Win+X`. Not `Win+Shift+M`,
  which restores every minimised window.
- **The overview is `Alt+Space`**, which is PowerToys Run's — what a Windows
  user is most likely to already have in their fingers.

The current defaults for all of them are in
[the configuration reference](config.md#keybinds), generated from the schema, so
that list cannot go stale the way this paragraph could.

Because a refused registration is silent — the key simply belongs to somebody
else — the first-run screen's **keys** step lists every binding, says which ones
Windows refused or which two of yours collide, and offers a free alternative.
`bw wizard open` reaches it again later.

### Workspaces need a third-party window manager

Hyprland's workspaces have no Windows equivalent the shell can read. Virtual
desktops are not exposed usefully, so workspaces come from
[GlazeWM](https://github.com/glzr-io/glazewm) or
[komorebi](https://github.com/LGUG2Z/komorebi) when one is running, and the bar
simply omits them when neither is.

### The shell is not the notification server

On Linux the shell _is_ the notification daemon, so every notification arrives
addressed to it. Windows keeps its own Action Center and does not hand it over.

The shell's own notifications work as they do upstream. Reading what _other_
applications posted is a separate, off-by-default feature that needs package
identity — an MSIX sparse package, signed with a certificate the machine
trusts. [docs/msix.md](msix.md) is the whole of that decision.

### The desktop's right button is off by default

Upstream opens the menu at the pointer from a right-click on the background
surface itself, which works because a `wlr-layer-shell` surface receives the
click.

Here the background is a window reparented under `WorkerW`, which puts it
_below_ the desktop icons: the right-click goes to Explorer's `SysListView32`
and never reaches the shell. Intercepting it needs a system-wide low-level mouse
hook that swallows the button — and Windows silently unhooks one whose thread
ever stalls past `LowLevelHooksTimeout`, quite apart from it being the kind of
API security software watches for.

So `hacks.desktopMenu` is off, and the menu is opened by its key or from the
launcher instead. Turning it on is a decision worth making deliberately, which
is why it lives in `hacks`.

### Theming stops at the shell

Upstream recolours GTK, Qt, Kvantum and more from the wallpaper. There is no
Windows counterpart to most of that. What is here:

- **The Windows accent colour** and light/dark mode
  (`appearance.wallpaperTheming.syncSystemAccent`, on by default).
- **Windows Terminal**, written into its settings
  (`syncWindowsTerminal`, off by default).

Everything else keeps its own colours.

### One monitor, for now

Only the primary monitor gets a background surface, and the region picker covers
the primary monitor only. Upstream's `Variants { model: Quickshell.screens }`
gives it every screen for free; the equivalent here is one window per monitor
plus `WM_DISPLAYCHANGE` handling, which is not written yet.

## What still means the same thing

**Config key names.** The schema mirrors `modules/common/Config.qml`, so the
vocabulary is the same one. `hyprland.*` is the exception, replaced by
[`windows.*`](config.md#windows) for GlazeWM/komorebi, taskbar hiding and accent
sync.

**`colors.json`.** The generated theme is written in matugen's shape, in the
same role vocabulary. Nothing in the shell reads it back — the theme is passed
in memory — but tooling written against the original still has a file to watch.

**Most state flag names.** Nine are identical: `desktopMenuOpen`,
`mediaControlsOpen`, `overlayOpen`, `overviewOpen`, `sessionOpen`,
`settingsOpen`, `sidebarLeftOpen`, `sidebarRightOpen`, `wallpaperSelectorOpen`.

Two are **not**, and a script written against upstream will miss them:

| end4-pC              | here               |
| -------------------- | ------------------ |
| `dropShelfOpen`      | `shelfOpen`        |
| `regionSelectorOpen` | `regionSelectOpen` |

Flags with no counterpart here — `barOpen`, `crosshairOpen`, `osdVolumeOpen`,
`osdBrightnessOpen`, `oskOpen`, `screenLocked`, `searchOpen`, `superDown` and
the rest — either belong to something not built or are not exposed.

**Six IPC target names**, reachable through the CLI rather than a socket:
`background`, `overlay`, `session`, `settings`, `wallpaperSelector`,
`wallpapers`. So `bw wallpapers apply <path>` is the same request as upstream's
`wallpapers` target.

The rest of the CLI has no upstream counterpart, either because the target does
not exist here (`lock`, `osk`, `mpris`, `cliphistService`) or because the
problem is Windows-specific:

| Command        | Why it exists                                              |
| -------------- | ---------------------------------------------------------- |
| `bw taskbar`   | showing the stock taskbar again after the shell is killed  |
| `bw autostart` | the Run key, which has no `exec-once` equivalent           |
| `bw config`    | one setting at a time, without an editor                   |
| `bw preset`    | whole configurations, saved by name                        |
| `bw wizard`    | the first-run screen, including the key report             |
| `bw capture`   | upstream's `region` and `screenTranslator`, under one verb |

## Things that only exist because this is Windows

- **The key report** — upstream never has a binding refused by the compositor.
- **`bw taskbar show`**, which works with no shell running, because hiding the
  stock taskbar and then being killed leaves nothing to start anything from.
- **An installer and an uninstaller** that put the machine back.
- **The MSIX sparse package**, for reading other applications' notifications.
- **Backdrop selection** — Mica on Windows 11, Acrylic on Windows 10.
