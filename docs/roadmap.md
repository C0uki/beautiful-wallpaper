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

### Done: the AI chat

Streamed rather than awaited — a long answer takes minutes, and an empty pane
for that long reads as a hang. The summarised reasoning gets its own
collapsible pane instead of being spliced into the answer, web search is on
with its queries and sources shown, and images and PDFs can be attached.
Replies render as Markdown with highlighted code and a copy button per block.

The original reaches three APIs through strategy objects; this reaches one,
so the strategy layer is gone and the streaming is what is left. Its SSE
parsing lives in bw-core under tests, including the three cases that are only
obvious once seen: a fallback has no event type of its own, a failed web
search arrives as HTTP 200 with an object where a list belongs, and thinking
deltas must not be concatenated onto the reply.

### Done: the image-board browser

The last of the original's four left-sidebar tabs, and the end of Phase 3.
Tag search over five boards in a masonry grid, each result settable as the
wallpaper through the download path the wallpaper picker already uses.

Two of the original's seven providers are deliberately not built: Zerochan has
no tag search in its API (upstream substitutes the colour parameter, which is
not what someone typing tags expects) and `t.alcy.cc` is a random-image CDN
with no metadata to show.

**The rating filter is part of the query, not a pass over the results.** A
client-side filter is one forgotten branch away from displaying what it was
meant to exclude, and it spends the request either way. It is on unless
switched off deliberately, the board that ships as the default carries nothing
but safe-rated work, and the tab itself is hidden entirely until
`policies.weeb` is changed from the 0 it ships with.

### Still to do

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
- **Reading other applications' notifications** is built, behind
  `hacks.readOtherNotifications` and the MSIX sparse package that gives the
  shell the package identity Windows requires for it — see the Phase 5 entry
  and `docs/msix.md`. With the setting off, the toasts and the centre show only
  what the shell itself posts.

## Phase 4 — Overlays

### Done: the overview and launcher

The search overlay, and the keyboard that reaches it.

- **Global hotkeys**, which the original does not need: Hyprland owns the
  keyboard there and a keybind runs `qs ipc call overview toggle`. Windows has
  no such layer, so the shell registers system-wide hotkeys itself — and
  Windows reserves a large part of the `Win`+letter space for its own shell
  and refuses the rest. Which combinations are refused is undocumented and
  moves with the version and the installed software, so a refusal is reported
  in a notification naming the binding rather than leaving a key that silently
  does nothing. The overview opens on `Alt+Space`, following PowerToys Run;
  the original's bare `Super` is not available on Windows at all.
- **The applications**, which have no single list to ask for. Desktop programs
  are Start-menu shortcuts across two folder trees, each `.lnk` a
  structured-storage document opened through COM to find its target; Store
  applications are not files and come from WinRT. Both are read, merged and
  de-duplicated, and shortcuts whose target is gone are dropped — those are
  the residue of an uninstall. It is slow enough that it runs once in the
  background and again when the Start menu changes, and the overview opens
  before it finishes, listing open windows until the rest arrives.
- **The matching**, in `bw-core` under tests. The best alignment is searched
  for rather than taken from the first greedy pass: `vsc` finding Visual
  Studio Code is the whole job, and a greedy scan matches the `s` inside
  `Visual`. The positions it settles on are what the UI highlights.
- **Arithmetic**, because the original hands it to `qalc` and there is no
  `qalc` on Windows. The part that matters is knowing when _not_ to answer —
  every keystroke goes through it, and `notepad` producing a calculator row
  would push the program being opened off the list.
- **The rest of the modes**: open windows, a `>` prefix that runs a command
  line, a `/` prefix for what the shell itself does, and a web search that is
  always last so a query matching nothing still goes somewhere.

Not built: the workspace grid with live window previews, which needs
`DwmRegisterThumbnail`, and ordering by how often something is used, which
needs a history the shell does not keep.

### Done: screenshots, OCR and the screen translator

The region picker, and the two things worth doing with a region besides
saving it.

- **The shutter fires before the overlay appears.** An overlay shown first is
  in its own screenshot, and a selection drawn against a live screen captures
  whatever the screen has moved on to rather than what was chosen. So the
  shell's transient surfaces are hidden — the bar and the dock stay, being on
  screen all the time — and `DwmFlush` waits for the compositor to draw a
  frame without them. That is a documented wait rather than a guess at a
  delay, and the picker then draws on the frozen copy.
- **Reading text needs no dependency**, because Windows has a recogniser.
  What it does not have is every language: one exists only for languages
  whose pack is installed, so "this machine cannot read text" is a
  first-class outcome, the way "this display has no brightness control"
  already is. Joining what it returns is not a formality either — it hands
  back lines, and a space between two Japanese lines is wrong.
- **The screen translator** is the recogniser feeding the translator that was
  already there for the left sidebar, so it is one path rather than two.
- The clipboard gets twenty-four bits per pixel, not thirty-two: a screenshot
  has no meaningful alpha and applications disagree about the fourth channel,
  some pasting the image and some a black rectangle.

`Win+Shift+S` is not among the defaults — Windows keeps it for the Snipping
Tool — so the keys are `Print`, `Ctrl+Print` and `Shift+Print`, and the same
three are `/screenshot`, `/ocr` and `/translate` in the launcher.

### Done: the session screen

Six ways out, of which a given machine can rarely do all six.

- **What the machine cannot do is not offered.** Hibernation needs a
  hibernation file, and sleep is not just S3 — most machines built in the last
  several years report every one of S1–S3 as unavailable and sleep through
  modern standby instead, so checking S3 alone would hide the sleep button on
  exactly the hardware that sleeps best.
- **The keyboard never starts on a button that ends the session.** The screen
  opens under a key the user pressed and Enter is one keystroke further, so
  the caret starts on something recoverable; if nothing recoverable is on
  offer it starts nowhere and the user has to say which they meant.
- **Nothing forces by default.** A shutdown that closes programs without
  letting them save is a data-loss button dressed as a convenience. Without
  forcing, an unsaved document stops the shutdown and Windows says which
  program is holding it up.
- Restarting and shutting down need `SeShutdownPrivilege`, which is in an
  ordinary user's token and switched off. Enabling it has a trap:
  **`AdjustTokenPrivileges` reports success when it granted nothing**, and the
  only way to know is to read the last error for `ERROR_NOT_ALL_ASSIGNED`.
  Skip that and the shell believes it holds a privilege it does not.
- A refusal leaves the screen up. Closing on one would leave the user with a
  machine that simply did not switch off.

### Done: the desktop menu

The menu the desktop's right button opens — and the reason it needs a hack to
open that way at all.

- **The right-click cannot reach the shell.** The background surface is
  reparented under `WorkerW` so it sits below the desktop icons, which is what
  makes it a wallpaper rather than a window over one. A click on the desktop
  therefore goes to Explorer's `SysListView32`. Floating the surface above the
  icons instead would take the click and cost the icons: no selection, no
  drag, no double-click to open.
- **So replacing Explorer's menu needs a system-wide low-level mouse hook**,
  and that is switched off by default under `hacks.desktopMenu`. Windows
  removes such a hook silently if the owning thread fails to answer within
  `LowLevelHooksTimeout` — nothing is reported, the menu just stops appearing
  one day — and it is the kind of API security software watches. So the hook's
  own callback does a hit test and a non-blocking send and nothing else; the
  work happens on a thread that is allowed to take time. The menu opens from
  its key, the launcher's `/desktop` and `bw desktopMenu` whether or not the
  hook is on.
- **At the edge the menu flips rather than sliding.** A menu nudged back onto
  the screen leaves the pointer sitting on an entry nobody aimed at, one
  twitch from selecting it, so it opens on the other side of the cursor
  instead. That rule and the entry list live in `bw-core` under tests, and the
  surface measures what it drew and asks rather than working it out again.
- **An entry appears only if the thing it opens exists.** Switching the
  overview off removes the overview line as well: a menu item pointing at a
  surface that has been turned off does nothing, and there is no way to tell
  that from a bug.

Not built: Explorer's own entries (New, Refresh, Paste), which would mean
enumerating shell extensions and driving `IContextMenu`.

### Done: the drop shelf

Somewhere to put a file down while the place it is going to is not on screen.

- **It holds paths, not copies.** Copying would duplicate gigabytes for a
  gesture that is meant to be free, and leave someone editing a copy while
  believing it was the original. The cost is that the thing behind a path can
  move, so an entry whose file has gone stays on the shelf, struck through and
  labelled, and is only cleared when the user says so — doing it automatically
  is how a shelf silently empties itself while a network drive is unplugged.
- **A full shelf refuses rather than evicting.** What is already there was put
  there deliberately; what is arriving may be a select-all nobody meant. And a
  drop reports three numbers — added, moved, refused — because "eight of the
  twenty I dropped" has to be explainable.
- **Receiving is free; giving back is not.** The webview already registers a
  shell drop target, so Windows hands the paths over as `tauri://drag-drop`
  without the backend emitting anything — which is also why the page's own
  `ondrop` never fires on Windows. Dragging back out cannot start from the page
  at all: an application expecting a file wants shell items, so a press and a
  few pixels of movement hand the selection to `SHCreateDataObject` and
  `SHDoDragDrop`, which also supply the drag image every other drag on the
  machine has. Copy and link only — a move would let the target delete the
  original, which is not what putting something on a shelf asked for.
- The drag command is deliberately synchronous. `SHDoDragDrop` is modal and
  must run on the thread that owns the window with OLE initialised, which is
  where Tauri runs a synchronous command and is not where the async runtime
  would put it.
- Names are worked out without `Path::file_name`: these are Windows paths and
  `bw-core`'s tests run on Linux, where `\` is not a separator and every name
  would come out as the whole path.

### Done: the screen's chrome and the hot corners

Fake rounded corners, a frame around the display, and the corners you throw
the pointer at.

- **One window, not eight.** The original gives each corner and each frame
  edge its own `PanelWindow`, which is cheap under Quickshell. Here every
  surface is a webview, and eight of them to paint eight coloured shapes is
  not a translation worth making — so the corners and the frame are one
  full-screen, click-through window drawn in CSS.
- **The hot corners are the exception, and they need the opposite.** They have
  to receive the pointer, and a full-screen window that is not click-through
  swallows every click on the desktop. Quickshell's `mask: Region` has no
  direct equivalent, but `SetWindowRgn` serves: everything outside the region
  is neither drawn nor hit-tested, so one window covers the display, its
  region is the union of the corner strips, and the rest of the desktop
  carries on as if it were not there. The ownership rule is the trap — on
  success the system takes the region handle and freeing it would hand the
  window manager a dangling pointer; on failure the caller still owns it.
- **A strip a few pixels out is not a cosmetic bug.** These are input regions
  nobody can see, so a right-hand strip anchored at zero, or two strips
  meeting in the middle of a narrow screen, is a sidebar that opens when
  somebody reaches for a window's close button. Both rules live in `bw-core`
  under tests, and `sidebar.cornerOpen.visualize` paints the strips for when
  that is not enough.
- **Full-screen detection compares rectangles**, because Windows has no
  concept of a full-screen window — only of one that happens to be the size of
  the screen. Exactly, not approximately: a maximised window stops at the work
  area and a full-screen one does not, and "close enough" would hide the
  corners for every maximised window on the machine.

Not built: the frame does **not** reserve screen space. The original gives each
of its four edges an exclusive zone so a maximised window stops short of them.
Windows binds one edge per app bar, so the same thing needs four more
`SHAppBarMessage` registrations on top of the bar's — five app bars from one
process, fighting Explorer's bookkeeping and any the user runs themselves. The
frame is off by default and four pixels thick; until that is worth the fight,
it sits over the edge of a maximised window rather than beside it.

Also not carried over: `clicklessCornerEnd` and its vertical offset, which
tune how close to the very corner the pointer has to get before a hover opens
something. They exist to make the Hyprland version usable; the region here is
already the exact strip.

### Done: the floating overlay

A canvas of small widgets over everything else — and pinning, which is the
whole point of it.

- **A pinned widget stays on screen after the overlay closes.** A crosshair is
  only useful while you are playing, which is exactly when the overlay is
  shut. That is also what makes this need two windows rather than one: Windows
  decides what a window draws and what it hit-tests with the same region, so a
  pinned crosshair — visible, and emphatically not clickable — cannot share a
  window with a pinned note, which is both. They are split by that one
  property, and getting it wrong is either a crosshair that eats every click
  in the middle of the screen or a note nobody can type into.
- **The crosshair is configured by pasting a Valorant share code**, because
  people already have one they like and typing twenty numbers into a config
  file is not the same offer. Reading that format has three traps and all
  three are silent: a code is a patch on the game's defaults rather than a
  document, unknown keys have to be stepped over (every real code opens with a
  profile marker), and the unbind flags have to be applied last — a vertical
  arm length means nothing until the axes are unbound, whichever order the two
  fields arrive in. All of it is in `bw-core` under tests.
- **The crosshair ships unpinned.** Click-through is the only mode it is
  useful in, so that is its default; pinning is a decision, because a pinned
  crosshair is over everything until somebody takes it away and nobody asked
  for one by installing a shell.
- The open overlay takes the keyboard — Escape closes it, and a note is there
  to be typed into. The window left behind by a pinned widget must not: the
  user is playing a game, and a shell that stole the keyboard when a crosshair
  appeared would be worse than no crosshair.

Not built, and not because of time:

- **The FPS limiter.** The original writes `fps_limit=` into MangoHud's config
  and sends it a signal. There is no MangoHud on Windows, and the nearest
  equivalent — RivaTuner — has no interface a shell is invited to drive. There
  is nothing here to port to.
- **The recorder.** The original shells out to `wf-recorder` through a script.
  Doing it here means `Windows.Graphics.Capture` into a `MediaTranscoder`,
  which is a body of D3D11 interop the size of its own change. The screenshot
  half of that panel already exists as the region picker, reachable from
  `Print`, the launcher and the desktop menu.
- **Resizing a widget by dragging its corner**, which the original allows.
  Widgets take the size their content asks for and can be moved but not
  stretched.
- `floatingImage` and `volumeMixer`, the two overlay widgets the original has
  that this does not: the mixer is already in the right sidebar, and a
  floating GIF is a joke that does not need porting.

### Still to do

Nothing: Phase 4 is complete.

## Phase 5 — Finishing

### Done: the settings screen

Every config key, reachable without opening a text editor.

- **The form is generated from the schema, not hand-written.** The original
  writes a control per key — six thousand lines of QML across ten pages — and
  that is reasonable to do once and bad to maintain: every key added later is
  one somebody has to remember a control for, and forgetting is silent. Here
  the rows come from walking `Config::default`, so a key added in any future
  change has a control the moment it exists, with the right type and the right
  default.
- **What the schema cannot say is curated on top**: which page a section
  belongs on, that a string is really a choice between four bar styles, that a
  decimal is a fraction of the screen and wants a slider. Anything not curated
  still gets a control, so the table is an improvement on the default rather
  than a list that has to be kept complete — and a test holds every section in
  the schema against the page table, so a new section cannot end up with
  nowhere to live.
- **Wording is not derived.** Doc comments are developer prose in English and
  UI copy is neither, so labels are mechanical and the exceptions live where
  the translations do. Acronyms are the one exception that has to be in code:
  "Ocr language" reads like a typo every time somebody sees it.
- **A value the form cannot edit is listed anyway**, saying so. A setting
  nobody can see is worse than one that admits it has to be edited in the
  file, because only the second tells you it exists.
- Every row shows its dotted path, so what the screen changes and what
  `bw config set` takes are visibly the same thing.

### Done: presets

The whole config under a name, and back again.

- **Applying shows what it would change first.** The original merges the file
  and tells you afterwards, which is fine for something you saved an hour ago
  and not fine for one somebody sent you: it rewrites a couple of hundred
  settings and there is no list of which. So Apply opens that list — every row
  with its label, its dotted path and both values — every row can be unticked,
  and the button says how many it will write. A row whose two ends read the
  same is worse than no row, so when both sides begin alike the shared
  beginning is dropped from each: two wallpaper paths become
  `…dunes-at-dusk.jpg → …pine-fog.png` rather than `C:/Users/you/…` twice.
- **And it can be taken back.** The config a preset replaced is kept for one
  press. Nothing else in this shell changes that much in one click, and nobody
  is putting two hundred settings back by hand. In memory and only the most
  recent one: persisting it would be offering to revert a decision made days
  ago, which is not an undo but a second preset nobody saved.
- **Applying is not a merge.** A preset written by a newer build carries keys
  this one has never heard of, and merging those in produces a `config.json`
  the shell then refuses to load — a shell that will not start, from pressing
  Apply. So applying is a list of paths, every one of which the comparison
  found on _both_ sides. The rest are reported, because "it applied and that
  part did nothing" is indistinguishable from a bug. A key the preset leaves
  out keeps its current value; nothing is ever deleted.
- **A name is a file name, and on Windows that is a rule with teeth.** The
  original replaces whitespace with `_` and stops, which is enough for a Bash
  argument: `..` walks out of the folder, `NUL` is a device whatever extension
  follows it, and a trailing dot is not refused but _dropped_, so saving `Dark.`
  would silently overwrite `Dark`. Names are checked in `bw-core` under tests,
  and matched case-insensitively — `dark` and `Dark` cannot both exist, so the
  button says Replace rather than finding out afterwards.
- A preset that will not parse is listed saying so rather than hidden. It is
  still on disk, and only the second kind of failure tells you that.
- **Applying one goes through the same path a hand-edited `config.json` does.**
  Two lists of what a wholesale config change invalidates would drift the first
  time either was added to, and the symptom would be a setting that works when
  typed into the file and not when a preset sets it. Folding them together
  found one that was already missing: `background.wallpaperPath` moved every
  surface to the new colours and left the desktop showing the old picture,
  because nothing told Windows. It does now, from either direction.
- `bw preset list|save|apply|remove` works without the shell running, the way
  `bw config` does — a preset is a file and applying one is a config edit.
- Nothing secret is in one: API keys live in the credential manager, not in
  `config.json`, which is what makes a preset safe to send to somebody.

### Done: the first run

The original's welcome app is one scrolling page of preferences, and on Linux
that is the right shape — the shell works the moment it starts, and the page is
an invitation. Here three of the things it would offer may not work at all
until somebody looks, and none of them announce themselves: no wallpaper has
been chosen, so the palette has nothing to generate from; the workspaces widget
needs a tiling window manager Windows does not ship, and without one it is
permanently empty, which reads as a bug; and Windows may have refused any of
the keyboard shortcuts. So this is a wizard, and each step establishes one
thing and reports what it found.

- **The keys step is the one only Windows makes necessary**, and it turned out
  to be worth more than the screen it was built for. Three different things can
  be wrong with a shortcut and only one of them is discoverable by pressing it:
  Windows documents a set of combinations as its own, which is knowable in
  advance; two bindings can name the same chord, which _nothing_ refuses — the
  first registers, the second is turned away, and whichever lost is a feature
  that cannot be reached and cannot be explained; and Windows can refuse a
  registration at runtime for reasons that are neither documented nor stable.
  All three are reported together, each with a free combination to use instead.
- **Writing that table down found two of the shipped defaults already broken.**
  `settings` was on `Win+Shift+S`, which opens the Snipping Tool on every
  Windows 11 machine — three lines above it, a doc comment said so — and `shelf`
  and `widgetEditMode` were both on `Win+Shift+D`. A test now holds every
  default against the table and against itself, so neither can come back.
- **What the table cannot say is whether `RegisterHotKey` will actually
  refuse.** That is undocumented and moves with the Windows version and the
  installed software, so the refusals the shell reports at runtime stay the
  authority; the table is what keeps the defaults out of a fight nobody needs.
- **Every step can be skipped**, from every step. A first-run screen that
  cannot be dismissed is a hostage, and none of this is a form. The step is
  remembered as it is left, so closing the window half-way and coming back
  resumes — and a state file from a build with more steps is clamped rather
  than resuming on a step that does not exist and showing nothing.
- The workspaces step says plainly that it found no GlazeWM and offers to take
  the widget off the bar, because a permanently empty widget looks like a bar
  that failed to draw. Only GlazeWM can be detected: its IPC is what this build
  reads, and komorebi would need the named-pipe client that does not exist yet.
- Hiding the taskbar switches the dock on with it. Hiding one without the other
  leaves no way to reach a minimised window at all.

### Done: the installer, and putting Windows back

The bundle already built; what was missing was everything either side of it.

- **Two settings had a control each and nothing behind them.**
  `windows.hideSystemTaskbar` and `windows.startWithWindows` were in the
  schema, so the generated settings screen drew a switch for each and the
  first-run screen drew one of them again — and _nothing in the shell read
  either key_. Both are the reason somebody opens that screen at all. They are
  applied at startup now, and re-applied whenever the config changes, through
  the same path a preset and a hand edit already take.
- **A shell that hides the taskbar has to be able to give it back.** It is held
  by a guard, the way the bar's reserved edge is — and that turned out not to
  be enough for either of them: Tauri's exit path calls `process::exit`, which
  unwinds nothing, so neither guard was ever dropped. The work area stayed
  shrunk and the taskbar stayed hidden after the shell was gone. Both are
  released on `RunEvent::Exit` now.
- That still leaves the process being killed outright, which no guard survives,
  so **`bw taskbar show` works with nothing running** — Task Manager's "Run new
  task" reaches it, which is the one thing still on screen when the taskbar is
  not. It deliberately does not write the config: somebody who wants it back
  for good is changing the setting, not running a rescue.
- **The Run key value is quoted.** `C:\Program Files\...\bw.exe` unquoted is
  read as `C:\Program.exe` with an argument, which is the classic way an
  auto-start entry fails — at login, where nothing of this shell is up to
  explain itself. And it carries no arguments at all: `bw` with none starts the
  shell, with any at all it is a CLI client, so a well-meant `--startup` would
  make every login print "not running". An entry left by an installation that
  has since moved is detected and rewritten rather than left as an error dialog
  every morning. All of it is in `bw-core` under tests.
- **The uninstaller undoes what a shell did to the machine**, in the order
  where failure hurts least: show the taskbar first, then remove the Run entry
  and the App Paths key. The user's own config, presets and thumbnails are
  _asked_ about, defaulting to keeping them.
- **The binary is called `bw` now.** Every usage string, doc comment and rescue
  instruction in this repository said `bw`, and none of them named a file that
  existed — it was `beautiful-wallpaper.exe`. An App Paths entry makes the bare
  name resolve from the Run dialog, which is what makes the rescue above
  something a person can type; the installer deliberately does **not** touch
  `PATH`, which is shared with every other program on the machine and cannot be
  repaired from here if a write goes wrong.

Not built: forgetting the API keys on uninstall. They are in the Windows
credential manager rather than in either data folder, so removing them needs a
command the shell does not have; until then an uninstall leaves them.

### Done: the MSIX sparse package

One feature needs it and nothing else does: showing notifications posted by
_other_ applications. `UserNotificationListener` is the only supported way to
read the Action Center, and Windows will not hand anything to a process it
cannot name — which is the normal state of a program installed from an `.exe`.
A sparse package is the documented way out: an MSIX carrying a manifest and
three logos and no payload at all, declaring `AllowExternalContent` and
registered with the folder the shell is already installed in.

- **The certificate is a decision about the machine, so nothing here makes
  one.** Windows refuses an unsigned package and one signed by a certificate it
  does not trust, and trusting a root certificate is a much larger permission
  than "run this package" — it can vouch for anything. So the repository ships
  no certificate, `build.ps1` takes one you already have, and `docs/msix.md`
  says plainly what installing one means. The setting stays off by default.
- **Every way this can fail gets its own sentence.** Four states rather than a
  boolean: no package identity yet, a Windows too old to have the listener,
  nobody has answered Windows' access prompt, and the user refused. "It is off"
  tells somebody nothing about which of those they are standing at, and each
  has a different next step.
- **Registering the package does not give identity to the process that
  registered it.** Windows decides identity when a process starts, so the shell
  says so and asks to be restarted rather than appearing to work and reporting
  nothing.
- **Every read returns the whole Action Center**, so the listener inverts the
  usual problem: not "what arrived" but "what is new since I last looked". Ids
  answer it — and **the first look deliberately reports nothing**, because the
  Action Center holds everything the user has not dismissed, often days of it,
  and replaying all of that the moment somebody switched the feature on would
  bury whatever was actually happening. Both rules are in `bw-core` under
  tests, along with how a toast's text elements become a summary and a body and
  what to call an application that has no display name.
- It polls rather than using `NotificationChanged`, which is only delivered to
  a packaged application with a background task — which a sparse package over a
  Win32 executable is not.

Not built: unregistering the package on uninstall, which needs the package
manager rather than a registry write; a signed package in the releases, which
needs a certificate held as a CI secret.

### Still to do

Documentation.

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
- **Capturing across monitors.** The region picker covers the primary monitor
  only. A window has a single scale factor, so an overlay spanning two
  monitors at different scales cannot map what was drawn on it back to pixels
  on both; doing it properly means one picker window per monitor, which is the
  same work as the point above.
