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

Split across several changes, because the original's version of this phase is
over ten thousand lines: the right sidebar alone is 4,611.

### Done: the OSD and notification toasts

- **The readout.** Volume comes from WASAPI, and it is pushed rather than
  polled — `IAudioEndpointVolumeCallback`, so the pill appears on the same
  keypress that changed the level instead of up to a poll interval later. The
  registration follows the default device, so plugging in headphones does not
  leave it reporting a device nobody is listening to.
- **Toasts.** Grouped by application, swipe to dismiss, and the history is
  bounded and persisted. The store lives in `bw-core` and has one `post` entry
  point, so a notification listener can feed it later without reshaping it.
- **Flag-to-window plumbing.** `surfaces::set_visible` had no callers: overlay
  windows were created hidden and nothing ever showed them, so the wallpaper
  picker could not be opened on Windows at all. Flags now move windows.
- **An icon subset that fails loudly.** The bundled font is subset to the icons
  the shell draws; a name outside that set used to render as the literal word.
  `pnpm gen:icons` rebuilds it from `apps/shell/scripts/icons.json`, and errors
  if a listed name is not in the font.

### Done: brightness and the right sidebar

- **Brightness**, and the night light. Laptop panels go through WMI, external
  displays through DDC/CI, and a display supporting neither is approximated
  with a gamma ramp. All three fail on some perfectly ordinary machine, so "no
  brightness control" is a first-class outcome and the control is hidden
  rather than shown dead. DDC/CI is a round trip over I²C per call, so writes
  are coalesced on a worker thread; the raw-range arithmetic lives in
  `bw-core` under tests, because a monitor reporting 0–64 rather than 0–100 is
  the normal case, not the exception.
- **The right sidebar** — banner, quick toggles in both of the original's
  styles, sliders, media, notification centre, and a tabbed calendar / to-do /
  timer group, with Wi-Fi, Bluetooth, mixer and night-light dialogs drawn over
  the panel rather than in windows of their own.
- **Per-application volume**, through `IAudioSessionManager2`. Sessions are
  addressed by session instance identifier rather than process id, which is
  reused the moment a process exits, and application icons are rasterised out
  of the executable and cached as PNGs the way wallpaper thumbnails are.
- **Wi-Fi and Bluetooth.** The earlier note here assumed `wlanapi` wrapped by
  hand and a WinRT Bluetooth stack that did not exist; in fact both are in the
  pinned `windows` crate and needed feature flags rather than bindings.
- **An icon subset that is verified, not assumed.** `pnpm gen:icons` now
  reopens the font it just wrote and fails if any listed name did not survive
  subsetting. It found four that had not: every Material Symbols name
  containing `_digit_` is an alias whose output glyph the subsetter prunes, so
  the shell was drawing "\_THREE\_BAR" where a signal-strength icon belonged.

### Done: the dock and the left sidebar's first two tabs

- **The dock**, which is what replaces the taskbar once the shell hides it.
  Windows are enumerated with Explorer's own filters — visible, un-owned, not
  a tool window, titled — plus `DWMWA_CLOAKED`, without which every UWP
  application on every other virtual desktop appears, looking entirely
  legitimate. Clicking raises, clicking again minimises, and a refused
  activation flashes the window the way Explorer does rather than leaving the
  icon inert. The watcher is event-driven: an icon that lingers a second after
  its application closes is what makes a dock feel broken.
- **The left sidebar**, with the translator and media tabs. The original
  shells out to `trans` (translate-shell) for translation, which is a Bash
  script and does not exist on Windows, so the translator goes through the
  Anthropic API — which puts the client and the key handling in place for the
  chat tab.

### Still to do

- **The left sidebar's AI chat and booru browser.** The chat is the largest
  single piece of the original's sidebar (~2,100 lines) and needs streaming,
  Markdown and code-block rendering, and conversation history on top of the
  client that now exists.
- **The dock's drag-to-reorder and drop targets.** Pinning works; rearranging
  pinned icons by dragging does not.
- **The media tab's visualiser and lyrics.** The visualiser needs a WASAPI
  loopback capture the shell does not have; the lyrics came from an external
  script.
- **Power plans.** The documented API reaches only the classic schemes, and
  Windows 11's power mode sits behind an undocumented overlay call, so the
  quick toggle for it is not built.
- **Bluetooth pairing and connecting.** Only paired devices are listed;
  pairing needs a PIN exchange with a UI of its own, and connecting is largely
  the stack's decision, so the dialog opens Windows' own settings for both.
- **Reading other applications' notifications**, which needs
  `UserNotificationListener` and therefore package identity — the MSIX sparse
  package in Phase 5. Until then the toasts and the centre show only what the
  shell itself posts.

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
