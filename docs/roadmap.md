# Roadmap

end4-pC is 65,000 lines of QML plus 41 shell and Python scripts. Reaching parity
is not one change, so the work is split into phases that each land on the same
foundation. Phases 0 and 1 are done; the rest are planned, not promised.

## Phase 0 — Foundation ✅

Repository skeleton, the pnpm and cargo workspaces, CI, the config system, the
theme engine, the token layer and widget kit, the three surface modes (WorkerW,
app bar, overlay), single-instance IPC and the CLI, and the mock backend that
makes the UI developable off Windows.

## Phase 1 — Wallpaper and desktop widgets ✅

The background surface, wallpaper transitions, the drag-and-snap widget canvas
with six widgets, and the wallpaper picker with local browsing and three online
providers.

## Phase 2 — The bar (mostly done)

Done: screen-space reservation through `SHAppBarMessage` — held in a value whose
`Drop` gives the edge back, so exiting never leaves the work area shrunk — the
four bar styles (`m3`, `hug`, `float`, `islands`), horizontal and vertical
variants, hover popups, and ten widgets: workspaces (GlazeWM), active window,
clock, weather, tray, battery, network throughput, resources, media and the
utility buttons. The layout comes from `bar.left/center/right`, as upstream.

The tray was the awkward one, as expected. Windows has no StatusNotifierItem
equivalent, so the icons are read out of Explorer's own toolbar across the
process boundary — `VirtualAllocEx` a `TBBUTTON` inside Explorer, `TB_GETBUTTON`
into it, `ReadProcessMemory` it back. It is undocumented and unverifiable
without a real Explorer, so it degrades to "no icons" on any failure.

Still to do:

- **Tray icon bitmaps.** Only the presence and owner of each icon is read; the
  widget shows dots rather than the icons themselves. The `HICON` is in the
  struct that is already being read, and icon handles are session-wide, so
  `DrawIconEx` into a bitmap should work — untested.
- **Tray interaction.** Clicking an icon should forward the owner's registered
  callback message to its window.
- **The audio visualiser**, which needs a WASAPI loopback capture the shell does
  not have yet.
- **The bar layout editor.** The three slots are configurable, but only by
  editing `config.json`; the drag-and-drop editor is part of Phase 5's settings
  UI.
- **Auto-hide**, and **one bar per monitor** — the config keys exist, the
  behaviour does not.

## Phase 3 — Sidebars, notifications, OSD, dock

The right sidebar (quick toggles, notification centre, calendar, to-do,
pomodoro, volume mixer, Wi-Fi, Bluetooth), the left sidebar (AI chat, translator,
booru browser), notification toasts, the volume and brightness OSD, and the dock.

Reading other applications' notifications needs `UserNotificationListener`, which
requires package identity. That means shipping an MSIX sparse package alongside
the installer (Phase 5); without it the notification centre shows only the
shell's own toasts.

## Phase 4 — Overlays

The overview and launcher (Start menu and UWP enumeration with fuzzy search),
region selection for screenshots and OCR, the session screen, the desktop
context menu, the drop shelf, screen corners, the screen frame, the floating
overlays (crosshair, notes, resources, FPS limiter, recorder) and the screen
translator.

## Phase 5 — Finishing

The full settings UI, presets, the first-run wizard, the MSIX sparse package,
the installer, and documentation.

## Deliberately not built

- **The lock screen.** Windows owns the lock screen, and a third-party
  replacement is a cosmetic imitation, not a security boundary. The shell will
  call the real one instead.
- **A polkit agent.** UAC cannot be hosted by a third-party process.
- **An on-screen keyboard.** Windows ships `osk.exe` and TabTip.
- **The Hyprland and niri settings pages**, replaced by a `windows.*` page for
  GlazeWM/komorebi, taskbar hiding and accent sync.
- **GTK, Kvantum and xsettingsd theming**, EasyEffects, the screen-share privacy
  indicator, and the Arch package integrations — none have a Windows counterpart.

## Known gaps in what exists

- **Transitions.** The original ships fourteen wallpaper transitions as
  precompiled Qt `.qsb` bundles with no GLSL source in the repository. Six are
  re-authored here as WebGL shaders (`fade`, `circle`, `dissolve`, `pixelate`,
  `ripple`, `stripes`); the rest — `glitch`, `crt`, `shatter`, `Doom`, `magic`,
  `Peel`, `circlePit`, `circleSelect` — are still to write, and will be
  approximations rather than reproductions.
- **`leastBusy` widget placement.** The original uses a 399-line OpenCV script to
  find the calmest region of a wallpaper. The config key exists; the
  implementation does not yet.
- **Video wallpapers.** The plan is a `<video>` element in the WorkerW surface,
  which needs no `mpvpaper` equivalent. Not started.
- **Per-monitor surfaces.** Only the primary monitor gets a background surface so
  far; `Variants { model: Quickshell.screens }` maps to one window per monitor
  plus `WM_DISPLAYCHANGE` handling.
