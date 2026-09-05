# When something does not work

Most of what follows is not a bug. This shell reaches for things Windows does
not offer an ordinary program — the notification area, another window manager's
workspaces, a laptop panel's backlight — and each of those can be absent on a
perfectly ordinary machine. Where that happens the shell is meant to say so
rather than go quiet, and the entries below are mostly about finding where it
said it.

Two things worth knowing first:

- **The config is `%APPDATA%\beautiful-wallpaper\config.json`**, watched both
  ways. An edit reaches the shell without a restart.
- **Unknown keys are rejected.** A whole file is refused if one key in it is
  misspelt, which is deliberate — a typo that silently did nothing would look
  exactly like a setting that does not work. Every accepted key is in
  [the configuration reference](config.md).

## The taskbar is gone and the shell is not running

`windows.hideSystemTaskbar` hides the stock taskbar while the shell's bar is
running, and the shell puts it back when it exits. If the shell is killed —
Task Manager, a crash, a power cut — nothing puts it back, and there is no
taskbar left to start anything from.

The way back, which works with no shell running:

```
bw taskbar show
```

Task Manager reaches it: **Ctrl+Shift+Esc → Run new task**. `Win+R` also works.

This is the reason the setting is off by default.

## A key does nothing

Windows keeps a set of chords for itself and simply refuses to register them —
`Win+Shift+S` is the Snipping Tool's, and it will not hand it over. A refused
registration is not an error the shell can show at the moment it happens,
because nothing happens: the key just belongs to somebody else.

The first-run screen's **keys** step lists every binding, says which of the two
kinds of trouble each is in — Windows refused it, or two of your own bindings
want the same chord — and offers a free alternative. It is reachable again at
any time:

```
bw wizard open
```

Any binding can also be set directly, with the paths under
[`keybinds`](config.md#keybinds):

```
bw config set keybinds.overview "Ctrl+Alt+Space"
```

## The tray is empty

Windows has no API for enumerating notification-area icons. They belong to
Explorer's own toolbar, and the only way to read them — the one every
third-party Windows bar ends up at — is to read that toolbar across the process
boundary. It is undocumented, so it degrades to showing nothing rather than
failing loudly.

Things that make it empty:

- **Explorer is not running**, or was restarted after the shell started.
- **The shell and Explorer are at different integrity levels.** Running the
  shell as administrator while Explorer runs normally puts the toolbar out of
  reach.
- A Windows update changed the toolbar's internals.

Restarting the shell after Explorer is the first thing to try. This is the
least verifiable code in the project; a change to it cannot be checked without
a real Explorer to read from.

## Brightness does not move

There is no single brightness API on Windows. Three are tried in order:

1. **WMI** (`WmiMonitorBrightnessMethods`) — laptop panels.
2. **DDC/CI** over the monitor cable — external displays, when the display and
   the cable both support it. Many do not, and some claim to and then ignore it.
3. **A gamma ramp** — not real brightness, but it changes what you see.

When none of them work the shell does not show a slider it cannot move: the
readout and the sidebar's brightness control are simply absent. That is the
intended behaviour, not a missing feature.

A desktop with an HDMI adapter in the way is the common case for DDC/CI
failing. DisplayPort and a direct HDMI connection are more likely to work.

## There are no workspaces

Windows has no workspace concept the shell can read — virtual desktops are not
exposed in a usable way. Workspaces come from a third-party tiling window
manager, and with none running there is nothing to show.

`windows.windowManager` is `"auto"`, which probes for
[GlazeWM](https://github.com/glzr-io/glazewm) and then
[komorebi](https://github.com/LGUG2Z/komorebi). If one is running and the bar
still shows nothing:

- **GlazeWM** is read over its WebSocket IPC on port `6123`. A different port
  goes in `windows.glazewm.port`.
- **komorebi** is read over a named pipe, `windows.komorebi.pipeName`.

Set `windows.windowManager` to `"none"` to stop probing entirely.

## Notifications from other applications never appear

The shell's own notifications work out of the box. Reading what _other_
applications have posted is a separate feature, off by default under
`hacks.readOtherNotifications`, and it needs package identity — which means
building and signing an MSIX sparse package and trusting your own certificate
on the machine.

That is a decision about your computer rather than about this shell, so nothing
here does it for you. [docs/msix.md](msix.md) is the whole of it, including
what the shell says at each of the four ways it can be blocked.

## The wallpaper will not change

- **The file is not there.** A config carried over from another machine names a
  picture this one may not have. The shell logs it and leaves everything else
  applied; the rest of the config still took effect.
- **`workSafety.blankWallpaper` is on** and the filename matches one of
  `workSafety.keywords`, in which case a flat colour is the intended result.
- **Something else owns the desktop.** Wallpaper Engine, Lively, and anything
  else that reparents a window under `WorkerW` is doing what this shell does,
  and the last one to start wins.

`bw wallpapers apply <path>` sets it directly and prints the failure if there
is one.

## An edit to `config.json` did nothing

The whole file is rejected when any key in it is unknown, so a single typo
means none of that save took effect. Check the spelling against
[the reference](config.md), or set the value through the CLI, which takes the
same dotted paths and will tell you if the path is wrong:

```
bw config set bar.reserveSpace false
bw config get bar.reserveSpace
```

Casing matters: keys are `camelCase`, as in `reserveSpace`.

## The shell will not start at all

It needs [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/),
which Windows 11 has already and Windows 10 usually does. The installer does
not bundle it.

## Getting back to something that worked

Presets are whole configurations, saved under a name:

```
bw preset list
bw preset save before-i-broke-it
bw preset apply before-i-broke-it
```

They live in `%APPDATA%\beautiful-wallpaper\presets`, and all four commands work
with no shell running.

Failing that, deleting `config.json` starts again from the defaults — the file
is written back the next time the shell starts. Nothing else in
`%APPDATA%\beautiful-wallpaper` is needed to run, and
`%LOCALAPPDATA%\beautiful-wallpaper` holds only generated themes, window
positions and caches, all of which are rebuilt.

## Uninstalling leaves things behind

The uninstaller shows the stock taskbar, removes the autostart entry and the
App Paths key, and asks before deleting your config and cache.

It does **not** unregister the MSIX sparse package, if you built one — that
needs the package manager rather than a registry write. Switching
`hacks.readOtherNotifications` off before uninstalling removes it. One left
behind points at a folder that is gone, which is inert rather than harmful.
