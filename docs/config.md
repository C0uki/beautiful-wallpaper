<!-- Generated from crates/bw-core/src/config/schema.rs by `pnpm gen:docs`. Do not edit. -->

# Configuration reference

Everything `config.json` accepts, with the value it has when nothing sets it.

The file lives at `%APPDATA%\beautiful-wallpaper\config.json`. It is watched
both ways: an edit in any editor reaches the shell without a restart, and a
change made in the shell is written back. Anything the shell cannot parse is
ignored until the next save, so a half-written file mid-edit is not a problem —
but an unknown key **is** rejected, which is deliberate: a typo that silently
did nothing would look exactly like a setting that does not work.

Three ways to change a value:

- **Settings** — the screen is built from this same schema, so every key here
  has a control there.
- **The CLI** — `bw config set bar.reserveSpace false`, taking the dotted paths
  below verbatim.
- **The file** — any editor.

## Contents

- [`ai`](#ai)
- [`appearance`](#appearance)
- [`audio`](#audio)
- [`background`](#background)
- [`bar`](#bar)
- [`capture`](#capture)
- [`desktopMenu`](#desktopmenu)
- [`dock`](#dock)
- [`hacks`](#hacks)
- [`keybinds`](#keybinds)
- [`language`](#language)
- [`notifications`](#notifications)
- [`osd`](#osd)
- [`overlay`](#overlay)
- [`overview`](#overview)
- [`policies`](#policies)
- [`resources`](#resources)
- [`session`](#session)
- [`shelf`](#shelf)
- [`sidebar`](#sidebar)
- [`time`](#time)
- [`wallpaperSelector`](#wallpaperselector)
- [`weather`](#weather)
- [`windows`](#windows)
- [`workSafety`](#worksafety)

## `ai`

The assistant in the left sidebar: which model answers, and what it is allowed to do while answering.

| Setting           | Value        | Default           |
| ----------------- | ------------ | ----------------- |
| `ai.model`        | text         | `"claude-opus-5"` |
| `ai.maxTokens`    | whole number | `4096`            |
| `ai.webSearch`    | true / false | `true`            |
| `ai.maxSearches`  | whole number | `5`               |
| `ai.showThinking` | true / false | `true`            |

- **`ai.webSearch`** — Let the model search the web when it needs to. Costs tokens, so it is a setting rather than always on.
- **`ai.maxSearches`** — Searches per turn. Without a cap a single question can run several.
- **`ai.showThinking`** — Show the model's summarised reasoning in its own pane.

## `appearance`

Colours, fonts and corner rounding — everything about how the shell looks that the wallpaper does not decide for itself.

### `appearance.palette`

`"auto"` picks a scheme variant per wallpaper, otherwise a fixed Material variant name (`tonalSpot`, `neutral`, `vibrant`, ...).

| Setting                          | Value | Default  |
| -------------------------------- | ----- | -------- |
| `appearance.palette.type`        | text  | `"auto"` |
| `appearance.palette.accentColor` | text  | `null`   |
| `appearance.palette.mode`        | text  | `"dark"` |

- **`appearance.palette.type`** — `"auto"` | `"tonalSpot"` | `"neutral"` | `"vibrant"` | `"expressive"` | `"content"` | `"fidelity"` | `"monochrome"` | `"rainbow"` | `"fruitSalad"`
- **`appearance.palette.accentColor`** — When set, overrides the colour extracted from the wallpaper.
- **`appearance.palette.mode`** — `"auto"` follows the wallpaper's luminance, else `"light"` / `"dark"`.

### `appearance.fonts`

| Setting                       | Value  | Default                                             |
| ----------------------------- | ------ | --------------------------------------------------- |
| `appearance.fonts.main`       | text   | `"Segoe UI Variable Text, Segoe UI, sans-serif"`    |
| `appearance.fonts.title`      | text   | `"Segoe UI Variable Display, Segoe UI, sans-serif"` |
| `appearance.fonts.monospace`  | text   | `"Cascadia Code, Consolas, monospace"`              |
| `appearance.fonts.reading`    | text   | `"Georgia, serif"`                                  |
| `appearance.fonts.expressive` | text   | `"Segoe UI Variable Display, Segoe UI, sans-serif"` |
| `appearance.fonts.pixelSize`  | number | `15.0`                                              |

### `appearance.transparency`

Extra translucency applied on top of the wallpaper-derived value.

| Setting                          | Value        | Default |
| -------------------------------- | ------------ | ------- |
| `appearance.transparency.enable` | true / false | `true`  |
| `appearance.transparency.extra`  | number       | `0.0`   |

- **`appearance.transparency.extra`** — Added to the wallpaper-derived background transparency.

### `appearance.wallpaperTheming`

| Setting                                           | Value        | Default |
| ------------------------------------------------- | ------------ | ------- |
| `appearance.wallpaperTheming.syncSystemAccent`    | true / false | `true`  |
| `appearance.wallpaperTheming.syncWindowsTerminal` | true / false | `false` |

- **`appearance.wallpaperTheming.syncSystemAccent`** — Recolour the OS accent colour and light/dark mode from the wallpaper.
- **`appearance.wallpaperTheming.syncWindowsTerminal`** — Write a matching colour scheme into Windows Terminal's settings.

| Setting                         | Value        | Default |
| ------------------------------- | ------------ | ------- |
| `appearance.roundingScale`      | number       | `1.0`   |
| `appearance.fakeScreenRounding` | whole number | `2`     |
| `appearance.screenRounding`     | whole number | `24`    |

- **`appearance.roundingScale`** — Corner rounding multiplier applied to every surface.
- **`appearance.fakeScreenRounding`** — Draw rounded corners over the screen's own square ones. `0` never, `1` always, `2` only when nothing is full-screen — which is the default, because four rounded corners over a full-screen video are four notches cut out of the picture.
- **`appearance.screenRounding`** — The radius of those corners, in pixels.

## `audio`

Volume steps, and the guard against the volume jumping to something painful.

| Setting      | Value        | Default |
| ------------ | ------------ | ------- |
| `audio.step` | whole number | `5`     |

- **`audio.step`** — Percentage points per volume step.

### `audio.protection`

| Setting                      | Value        | Default |
| ---------------------------- | ------------ | ------- |
| `audio.protection.enable`    | true / false | `true`  |
| `audio.protection.maxVolume` | whole number | `100`   |

- **`audio.protection.maxVolume`** — Volume is not allowed above this by the shell's own controls.

## `background`

The wallpaper, how it arrives on screen, and the widgets drawn over it.

| Setting                             | Value        | Default    |
| ----------------------------------- | ------------ | ---------- |
| `background.wallpaperPath`          | text         | `""`       |
| `background.thumbnailPath`          | text         | `""`       |
| `background.wallpaperAnimation`     | text         | `"circle"` |
| `background.transitionDuration`     | whole number | `1200`     |
| `background.centeredWallpaper`      | true / false | `false`    |
| `background.centeredWallpaperShape` | text         | `"clover"` |
| `background.centeredWallpaperSize`  | number       | `0.55`     |

- **`background.thumbnailPath`** — Extracted still frame, for video wallpapers.
- **`background.wallpaperAnimation`** — Transition played when the wallpaper changes.
- **`background.centeredWallpaper`** — Render the wallpaper clipped into a Material shape, centred.

### `background.parallax`

| Setting                            | Value        | Default |
| ---------------------------------- | ------------ | ------- |
| `background.parallax.enable`       | true / false | `true`  |
| `background.parallax.zoom`         | number       | `1.07`  |
| `background.parallax.workspacePan` | number       | `0.6`   |

- **`background.parallax.zoom`** — Zoom applied so panning never exposes an edge.

### `background.widgets`

| Setting                     | Value        | Default |
| --------------------------- | ------------ | ------- |
| `background.widgets.enable` | true / false | `true`  |
| `background.widgets.grid`   | whole number | `8`     |

- **`background.widgets.grid`** — Snap-to-grid step, in pixels, for dragged widgets.

### `background.widgets.clock`

| Setting                                      | Value        | Default     |
| -------------------------------------------- | ------------ | ----------- |
| `background.widgets.clock.id`                | text         | `"clock"`   |
| `background.widgets.clock.enable`            | true / false | `true`      |
| `background.widgets.clock.x`                 | number       | `0.04`      |
| `background.widgets.clock.y`                 | number       | `0.06`      |
| `background.widgets.clock.placementStrategy` | text         | `"free"`    |
| `background.widgets.clock.style`             | text         | `"default"` |

- **`background.widgets.clock.x`** — Fraction of the monitor's width, so placement survives resolution changes.
- **`background.widgets.clock.placementStrategy`** — `"free"` keeps the stored position; `"leastBusy"` moves the widget to the calmest region of the current wallpaper.

### `background.widgets.media`

| Setting                                      | Value        | Default     |
| -------------------------------------------- | ------------ | ----------- |
| `background.widgets.media.id`                | text         | `"media"`   |
| `background.widgets.media.enable`            | true / false | `true`      |
| `background.widgets.media.x`                 | number       | `0.04`      |
| `background.widgets.media.y`                 | number       | `0.3`       |
| `background.widgets.media.placementStrategy` | text         | `"free"`    |
| `background.widgets.media.style`             | text         | `"default"` |

- **`background.widgets.media.x`** — Fraction of the monitor's width, so placement survives resolution changes.
- **`background.widgets.media.placementStrategy`** — `"free"` keeps the stored position; `"leastBusy"` moves the widget to the calmest region of the current wallpaper.

### `background.widgets.weather`

| Setting                                        | Value        | Default     |
| ---------------------------------------------- | ------------ | ----------- |
| `background.widgets.weather.id`                | text         | `"weather"` |
| `background.widgets.weather.enable`            | true / false | `true`      |
| `background.widgets.weather.x`                 | number       | `0.72`      |
| `background.widgets.weather.y`                 | number       | `0.05`      |
| `background.widgets.weather.placementStrategy` | text         | `"free"`    |
| `background.widgets.weather.style`             | text         | `"default"` |

- **`background.widgets.weather.x`** — Fraction of the monitor's width, so placement survives resolution changes.
- **`background.widgets.weather.placementStrategy`** — `"free"` keeps the stored position; `"leastBusy"` moves the widget to the calmest region of the current wallpaper.

### `background.widgets.resources`

| Setting                                          | Value        | Default       |
| ------------------------------------------------ | ------------ | ------------- |
| `background.widgets.resources.id`                | text         | `"resources"` |
| `background.widgets.resources.enable`            | true / false | `true`        |
| `background.widgets.resources.x`                 | number       | `0.72`        |
| `background.widgets.resources.y`                 | number       | `0.2`         |
| `background.widgets.resources.placementStrategy` | text         | `"free"`      |
| `background.widgets.resources.style`             | text         | `"default"`   |

- **`background.widgets.resources.x`** — Fraction of the monitor's width, so placement survives resolution changes.
- **`background.widgets.resources.placementStrategy`** — `"free"` keeps the stored position; `"leastBusy"` moves the widget to the calmest region of the current wallpaper.

### `background.widgets.calendar`

| Setting                                         | Value        | Default      |
| ----------------------------------------------- | ------------ | ------------ |
| `background.widgets.calendar.id`                | text         | `"calendar"` |
| `background.widgets.calendar.enable`            | true / false | `false`      |
| `background.widgets.calendar.x`                 | number       | `0.72`       |
| `background.widgets.calendar.y`                 | number       | `0.45`       |
| `background.widgets.calendar.placementStrategy` | text         | `"free"`     |
| `background.widgets.calendar.style`             | text         | `"default"`  |

- **`background.widgets.calendar.x`** — Fraction of the monitor's width, so placement survives resolution changes.
- **`background.widgets.calendar.placementStrategy`** — `"free"` keeps the stored position; `"leastBusy"` moves the widget to the calmest region of the current wallpaper.

### `background.widgets.userCard`

| Setting                                         | Value        | Default      |
| ----------------------------------------------- | ------------ | ------------ |
| `background.widgets.userCard.id`                | text         | `"userCard"` |
| `background.widgets.userCard.enable`            | true / false | `false`      |
| `background.widgets.userCard.x`                 | number       | `0.72`       |
| `background.widgets.userCard.y`                 | number       | `0.6`        |
| `background.widgets.userCard.placementStrategy` | text         | `"free"`     |
| `background.widgets.userCard.style`             | text         | `"default"`  |

- **`background.widgets.userCard.x`** — Fraction of the monitor's width, so placement survives resolution changes.
- **`background.widgets.userCard.placementStrategy`** — `"free"` keeps the stored position; `"leastBusy"` moves the widget to the calmest region of the current wallpaper.

### `background.widgets.notes`

| Setting                                      | Value        | Default     |
| -------------------------------------------- | ------------ | ----------- |
| `background.widgets.notes.id`                | text         | `"notes"`   |
| `background.widgets.notes.enable`            | true / false | `false`     |
| `background.widgets.notes.x`                 | number       | `0.04`      |
| `background.widgets.notes.y`                 | number       | `0.62`      |
| `background.widgets.notes.placementStrategy` | text         | `"free"`    |
| `background.widgets.notes.style`             | text         | `"default"` |

- **`background.widgets.notes.x`** — Fraction of the monitor's width, so placement survives resolution changes.
- **`background.widgets.notes.placementStrategy`** — `"free"` keeps the stored position; `"leastBusy"` moves the widget to the calmest region of the current wallpaper.

## `bar`

The strip along one edge of the screen: where it sits, how it looks, and what it carries.

| Setting              | Value        | Default                                                          |
| -------------------- | ------------ | ---------------------------------------------------------------- |
| `bar.enable`         | true / false | `true`                                                           |
| `bar.bottom`         | true / false | `false`                                                          |
| `bar.vertical`       | true / false | `false`                                                          |
| `bar.height`         | whole number | `40`                                                             |
| `bar.reserveSpace`   | true / false | `true`                                                           |
| `bar.autoHide`       | true / false | `false`                                                          |
| `bar.style`          | text         | `"m3"`                                                           |
| `bar.left`           | list of text | `["media"]`                                                      |
| `bar.center`         | list of text | `["workspaces","activeWindow"]`                                  |
| `bar.right`          | list of text | `["tray","resources","network","battery","utilButtons","clock"]` |
| `bar.showFrame`      | true / false | `false`                                                          |
| `bar.frameThickness` | whole number | `4`                                                              |
| `bar.frameColor`     | text         | `"black"`                                                        |

- **`bar.bottom`** — Anchor the bar to the bottom edge instead of the top.
- **`bar.reserveSpace`** — Reserve screen space through `SHAppBarMessage` so maximised windows keep clear of the bar.
- **`bar.style`** — `"hug"` | `"float"` | `"islands"` | `"m3"`
- **`bar.showFrame`** — Draw a thin border around the whole screen.
- **`bar.frameColor`** — A palette role name — `primary`, `surface`, `outline` and so on — or any CSS colour. The default is the original's.

## `capture`

Screenshots and what can be done with one — the region picker, OCR and the screen translator.

| Setting                   | Value        | Default |
| ------------------------- | ------------ | ------- |
| `capture.enable`          | true / false | `true`  |
| `capture.savePath`        | text         | `""`    |
| `capture.copyToClipboard` | true / false | `true`  |
| `capture.ocrLanguage`     | text         | `""`    |

- **`capture.savePath`** — Where screenshots go. Empty means `Pictures\Screenshots`, which is where Windows itself puts them.
- **`capture.ocrLanguage`** — A BCP-47 tag for the recogniser, or empty for the languages the user has already told Windows they read. Recognition only works for languages whose pack is installed, so naming one here that is not present leaves the feature unavailable rather than wrong.

## `desktopMenu`

The menu the desktop's right button opens, and which entries it offers. Opening it that way needs `hacks.desktopMenu`; its key and the launcher work either way.

| Setting                       | Value        | Default |
| ----------------------------- | ------------ | ------- |
| `desktopMenu.enable`          | true / false | `true`  |
| `desktopMenu.changeWallpaper` | true / false | `true`  |
| `desktopMenu.nextWallpaper`   | true / false | `true`  |
| `desktopMenu.editWidgets`     | true / false | `true`  |
| `desktopMenu.overview`        | true / false | `true`  |
| `desktopMenu.screenshot`      | true / false | `true`  |
| `desktopMenu.session`         | true / false | `true`  |
| `desktopMenu.displaySettings` | true / false | `true`  |
| `desktopMenu.personalise`     | true / false | `true`  |

## `dock`

The strip of pinned and running applications.

| Setting                  | Value        | Default |
| ------------------------ | ------------ | ------- |
| `dock.enable`            | true / false | `false` |
| `dock.height`            | whole number | `60`    |
| `dock.iconSize`          | whole number | `44`    |
| `dock.pinnedApps`        | list of text | `[]`    |
| `dock.ignored`           | list of text | `[]`    |
| `dock.showBackground`    | true / false | `true`  |
| `dock.showPinButton`     | true / false | `true`  |
| `dock.showMedia`         | true / false | `true`  |
| `dock.autoHide`          | true / false | `true`  |
| `dock.hoverRegionHeight` | whole number | `3`     |
| `dock.pinnedOnStartup`   | true / false | `false` |

- **`dock.pinnedApps`** — Full paths of the executables kept on the dock whether or not they are running. The original stores desktop-entry ids; Windows has no equivalent, and a path is the only thing that reliably identifies an application across launches.
- **`dock.ignored`** — Case-insensitive glob patterns matched against an executable's file name — `msedgewebview2.exe`, `*host.exe`. Anything matching never reaches the dock. The original calls this `ignoredAppRegexes` and takes regular expressions. A dock ignore list is a handful of file names, so this takes globs instead rather than pulling a regex engine into the portable crate — and is named for what it actually accepts.
- **`dock.autoHide`** — Slide out of the way until the pointer reaches the screen edge.
- **`dock.hoverRegionHeight`** — How much of the dock stays on screen while it is hidden. This is the strip the pointer has to reach, so zero would make the dock unreachable.
- **`dock.pinnedOnStartup`** — Start pinned open, reserving screen space rather than hiding.

## `hacks`

Settings that reach past what Windows offers an ordinary program. Each one costs something, and the note on each says what.

| Setting                        | Value        | Default |
| ------------------------------ | ------------ | ------- |
| `hacks.configReloadDelay`      | whole number | `50`    |
| `hacks.desktopMenu`            | true / false | `false` |
| `hacks.readOtherNotifications` | true / false | `false` |

- **`hacks.configReloadDelay`** — Debounce, in milliseconds, between a config file write and the reload that follows it.
- **`hacks.desktopMenu`** — Take over the desktop's right button, replacing Explorer's menu. **Off, and it is in `hacks` for a reason.** The background surface sits _below_ the desktop icons, so the click never reaches the shell; the only way to intercept it is a system-wide low-level mouse hook that swallows the button. Windows silently removes such a hook if its thread ever stalls past `LowLevelHooksTimeout`, and it is the kind of API security software watches. The menu is reachable by its key and from the launcher either way.
- **`hacks.readOtherNotifications`** — Show notifications posted by other applications, not just the shell's own. **Off, and it is in `hacks` for a reason.** Windows only lets `UserNotificationListener` read the Action Center for an application with _package identity_, which an ordinary installed program does not have. Getting it means registering a signed MSIX sparse package — and trusting the certificate it is signed with, which is a decision about the machine rather than about this shell. Switching this on without that in place changes nothing except the reason the settings screen gives for why it is not working.

## `keybinds`

The chords that open each surface. Windows keeps some combinations for itself and simply refuses to register them; the settings screen says which of these it refused and suggests a free one.

| Setting                      | Value        | Default           |
| ---------------------------- | ------------ | ----------------- |
| `keybinds.enable`            | true / false | `true`            |
| `keybinds.overview`          | text         | `"Alt+Space"`     |
| `keybinds.sidebarLeft`       | text         | `"Super+Shift+A"` |
| `keybinds.sidebarRight`      | text         | `"Super+Shift+N"` |
| `keybinds.wallpaperSelector` | text         | `"Super+Shift+W"` |
| `keybinds.widgetEditMode`    | text         | `"Super+Shift+D"` |
| `keybinds.captureRegion`     | text         | `"Print"`         |
| `keybinds.captureOcr`        | text         | `"Ctrl+Print"`    |
| `keybinds.captureTranslate`  | text         | `"Shift+Print"`   |
| `keybinds.session`           | text         | `"Super+Shift+E"` |
| `keybinds.desktopMenu`       | text         | `"Super+Shift+X"` |
| `keybinds.shelf`             | text         | `"Super+Shift+F"` |
| `keybinds.overlay`           | text         | `"Super+Shift+O"` |
| `keybinds.settings`          | text         | `"Super+Shift+I"` |

- **`keybinds.overview`** — `Alt+Space` follows PowerToys Run, which is what a Windows user is most likely to already have in their fingers.
- **`keybinds.captureRegion`** — `Win+Shift+S` is not available: Windows keeps it for the Snipping Tool and will not hand it over.
- **`keybinds.desktopMenu`** — `X` after `Win+X`, which is the closest thing Windows has to this. Not `Win+Shift+M`, which restores every minimised window.
- **`keybinds.shelf`** — `F` for files, since `Win+Shift+D` is the widget editor's.
- **`keybinds.settings`** — `I` after `Win+I`, which is where Windows keeps its own settings. Not `Win+Shift+S`, which opens the Snipping Tool.

## `language`

Which language the interface is drawn in.

| Setting       | Value | Default  |
| ------------- | ----- | -------- |
| `language.ui` | text  | `"auto"` |

- **`language.ui`** — `"auto"` follows the OS UI language.

## `notifications`

Toasts: where they appear, how long they stay, and how many at once.

| Setting                      | Value        | Default       |
| ---------------------------- | ------------ | ------------- |
| `notifications.enable`       | true / false | `true`        |
| `notifications.timeout`      | whole number | `7000`        |
| `notifications.position`     | text         | `"top_right"` |
| `notifications.maxVisible`   | whole number | `4`           |
| `notifications.doNotDisturb` | true / false | `false`       |
| `notifications.width`        | whole number | `380`         |

- **`notifications.timeout`** — Milliseconds a toast stays up. Urgent notifications ignore this.
- **`notifications.position`** — One of `top_left`, `top_center`, `top_right`, `bottom_left`, `bottom_center`, `bottom_right`.
- **`notifications.maxVisible`** — Toasts beyond this stay in the centre without ever popping up.
- **`notifications.doNotDisturb`** — Suppresses toasts without discarding the notifications themselves.

## `osd`

The readout that appears on a volume or brightness key.

| Setting          | Value        | Default |
| ---------------- | ------------ | ------- |
| `osd.enable`     | true / false | `true`  |
| `osd.timeout`    | whole number | `1000`  |
| `osd.position`   | text         | `"top"` |
| `osd.volume`     | true / false | `true`  |
| `osd.brightness` | true / false | `true`  |

- **`osd.timeout`** — Milliseconds the readout stays up after the last change.
- **`osd.position`** — `"top"` or `"bottom"`. The readout clears the bar on whichever edge the bar occupies, so this is only about which end of the screen.
- **`osd.brightness`** — Brightness is unavailable on some displays; the readout is simply not shown when the platform cannot report a level.

## `overlay`

The floating always-on-top panel, and the crosshair it can draw.

| Setting                       | Value        | Default |
| ----------------------------- | ------------ | ------- |
| `overlay.enable`              | true / false | `true`  |
| `overlay.darkenScreen`        | true / false | `true`  |
| `overlay.clickthroughOpacity` | number       | `0.8`   |

- **`overlay.darkenScreen`** — Darken what is behind while the overlay is open.
- **`overlay.clickthroughOpacity`** — How solid a widget looks once the pointer passes through it — visibly different, so "pinned and clickable" is not mistaken for "pinned and not".

### `overlay.crosshair`

| Setting                  | Value | Default                     |
| ------------------------ | ----- | --------------------------- |
| `overlay.crosshair.code` | text  | `"0;P;d;1;0l;10;0o;2;1b;0"` |

- **`overlay.crosshair.code`** — A Valorant crosshair share code. Paste one from the game or from a builder site — https://www.vcrdb.net/builder — rather than typing twenty numbers in by hand.

## `overview`

The overview and launcher — the search box, and what it may search.

| Setting                    | Value        | Default                                |
| -------------------------- | ------------ | -------------------------------------- |
| `overview.enable`          | true / false | `true`                                 |
| `overview.maxResults`      | whole number | `8`                                    |
| `overview.searchEngine`    | text         | `"https://www.google.com/search?q=%s"` |
| `overview.showWindows`     | true / false | `true`                                 |
| `overview.showApps`        | true / false | `true`                                 |
| `overview.allowRunCommand` | true / false | `true`                                 |

- **`overview.maxResults`** — How many applications and windows to offer. The arithmetic answer and the web-search row are never counted against this: they are one row each and both are the point of typing.
- **`overview.searchEngine`** — A URL with `%s` where the query goes. Without the placeholder the query is appended instead, so a hand-edited prefix still works.
- **`overview.allowRunCommand`** — Whether `>` runs the rest of the line.

## `policies`

Whether whole features are offered at all, which is how a feature is turned off for good rather than merely closed.

| Setting         | Value        | Default |
| --------------- | ------------ | ------- |
| `policies.ai`   | whole number | `1`     |
| `policies.weeb` | whole number | `0`     |

- **`policies.ai`** — 0 = off, 1 = on, 2 = local only.

## `resources`

How often CPU, RAM and disk are sampled for the widgets that show them.

| Setting                  | Value        | Default |
| ------------------------ | ------------ | ------- |
| `resources.pollInterval` | whole number | `2000`  |
| `resources.showSwap`     | true / false | `false` |

- **`resources.pollInterval`** — Sampling interval for CPU/RAM/disk, in milliseconds.

## `session`

The session screen, and which of its actions are offered.

| Setting             | Value        | Default |
| ------------------- | ------------ | ------- |
| `session.enable`    | true / false | `true`  |
| `session.lock`      | true / false | `true`  |
| `session.sleep`     | true / false | `true`  |
| `session.hibernate` | true / false | `true`  |
| `session.logOut`    | true / false | `true`  |
| `session.restart`   | true / false | `true`  |
| `session.shutDown`  | true / false | `true`  |
| `session.force`     | true / false | `false` |

- **`session.force`** — Close applications without giving them the chance to save. Off, and worth leaving off. Without it an unsaved document stops the shutdown and Windows says which program is holding it up, which is the whole point of being asked.

## `shelf`

The drop shelf: somewhere to park a dragged file on the way to somewhere else.

| Setting                | Value        | Default   |
| ---------------------- | ------------ | --------- |
| `shelf.enable`         | true / false | `true`    |
| `shelf.edge`           | text         | `"right"` |
| `shelf.width`          | number       | `0.2`     |
| `shelf.maxItems`       | whole number | `100`     |
| `shelf.clearAfterDrag` | true / false | `false`   |

- **`shelf.edge`** — Which edge it sits against: `left` or `right`.
- **`shelf.width`** — Width as a fraction of the screen, as the sidebars are measured.
- **`shelf.maxItems`** — Entries beyond this are refused rather than pushing others off the shelf. What is already there was put there deliberately; what is arriving may be a select-all nobody meant.
- **`shelf.clearAfterDrag`** — Take an entry off the shelf once it has been dragged somewhere. Off, because a shelf is often the source of two drops — into a chat and then into a folder — and the second one would have nothing to drag.

## `sidebar`

The two sidebars — the left one's tabs, the right one's toggles, sliders and notification centre.

| Setting                      | Value        | Default |
| ---------------------------- | ------------ | ------- |
| `sidebar.enable`             | true / false | `true`  |
| `sidebar.width`              | number       | `0.26`  |
| `sidebar.banner`             | true / false | `true`  |
| `sidebar.bannerImage`        | text         | `""`    |
| `sidebar.mediaPlayer`        | true / false | `true`  |
| `sidebar.notificationCentre` | true / false | `true`  |

- **`sidebar.width`** — Fraction of the screen width the panel occupies.
- **`sidebar.banner`** — Show the wallpaper banner with the avatar and uptime, rather than a plain row of system buttons.
- **`sidebar.bannerImage`** — Overrides the banner image; empty means the current wallpaper.

### `sidebar.profile`

| Setting                       | Value | Default |
| ----------------------------- | ----- | ------- |
| `sidebar.profile.displayName` | text  | `""`    |
| `sidebar.profile.avatarPath`  | text  | `""`    |

- **`sidebar.profile.displayName`** — Empty means the Windows account name.
- **`sidebar.profile.avatarPath`** — Empty means the account picture Windows already has.

### `sidebar.quickToggles`

| Setting                       | Value        | Default     |
| ----------------------------- | ------------ | ----------- |
| `sidebar.quickToggles.enable` | true / false | `true`      |
| `sidebar.quickToggles.style`  | text         | `"android"` |

- **`sidebar.quickToggles.style`** — `"classic"` for a single row of small buttons, `"android"` for the editable grid of tiles. Both are built; this only picks which.

### `sidebar.quickSliders`

| Setting                               | Value        | Default |
| ------------------------------------- | ------------ | ------- |
| `sidebar.quickSliders.enable`         | true / false | `true`  |
| `sidebar.quickSliders.showBrightness` | true / false | `true`  |
| `sidebar.quickSliders.showVolume`     | true / false | `true`  |
| `sidebar.quickSliders.showMic`        | true / false | `true`  |

### `sidebar.nightLight`

| Setting                          | Value        | Default   |
| -------------------------------- | ------------ | --------- |
| `sidebar.nightLight.enable`      | true / false | `false`   |
| `sidebar.nightLight.temperature` | whole number | `4000`    |
| `sidebar.nightLight.automatic`   | true / false | `false`   |
| `sidebar.nightLight.from`        | text         | `"20:00"` |
| `sidebar.nightLight.to`          | text         | `"07:00"` |

- **`sidebar.nightLight.temperature`** — Colour temperature in kelvin. 6500 is neutral; lower is warmer.
- **`sidebar.nightLight.automatic`** — Turn it on and off with the clock rather than by hand.
- **`sidebar.nightLight.from`** — 24-hour local times, used only when `automatic` is set.

### `sidebar.left`

| Setting               | Value        | Default |
| --------------------- | ------------ | ------- |
| `sidebar.left.enable` | true / false | `true`  |
| `sidebar.left.width`  | number       | `0.26`  |

- **`sidebar.left.width`** — Fraction of the screen width the panel occupies.

### `sidebar.left.translator`

| Setting                          | Value        | Default  |
| -------------------------------- | ------------ | -------- |
| `sidebar.left.translator.enable` | true / false | `true`   |
| `sidebar.left.translator.delay`  | whole number | `300`    |
| `sidebar.left.translator.from`   | text         | `"auto"` |
| `sidebar.left.translator.to`     | text         | `"en"`   |

- **`sidebar.left.translator.delay`** — Milliseconds of quiet before the text is sent. Translating on every keystroke would bill a request per character.
- **`sidebar.left.translator.from`** — Two-letter code, or `auto` to detect.

### `sidebar.left.media`

| Setting                     | Value        | Default |
| --------------------------- | ------------ | ------- |
| `sidebar.left.media.enable` | true / false | `true`  |

### `sidebar.left.booru`

| Setting                         | Value        | Default       |
| ------------------------------- | ------------ | ------------- |
| `sidebar.left.booru.provider`   | text         | `"safebooru"` |
| `sidebar.left.booru.allowAdult` | true / false | `false`       |
| `sidebar.left.booru.perPage`    | whole number | `30`          |

- **`sidebar.left.booru.provider`** — One of `safebooru`, `yandere`, `konachan`, `danbooru`, `gelbooru`. Safebooru by default: it is the one board that carries nothing but safe-rated work.
- **`sidebar.left.booru.allowAdult`** — Lift the safe-rating filter. Off unless set deliberately, and it does nothing on a board that has only safe work to return.

### `sidebar.cornerOpen`

| Setting                                 | Value        | Default              |
| --------------------------------------- | ------------ | -------------------- |
| `sidebar.cornerOpen.enable`             | true / false | `true`               |
| `sidebar.cornerOpen.bottom`             | true / false | `false`              |
| `sidebar.cornerOpen.valueScroll`        | true / false | `true`               |
| `sidebar.cornerOpen.clickless`          | true / false | `false`              |
| `sidebar.cornerOpen.cornerRegionWidth`  | whole number | `250`                |
| `sidebar.cornerOpen.cornerRegionHeight` | whole number | `5`                  |
| `sidebar.cornerOpen.visualize`          | true / false | `false`              |
| `sidebar.cornerOpen.topLeftAction`      | text         | `"sidebarLeftOpen"`  |
| `sidebar.cornerOpen.topRightAction`     | text         | `"sidebarRightOpen"` |
| `sidebar.cornerOpen.bottomLeftAction`   | text         | `"sidebarLeftOpen"`  |
| `sidebar.cornerOpen.bottomRightAction`  | text         | `"sidebarRightOpen"` |

- **`sidebar.cornerOpen.bottom`** — Whether the bottom two corners do anything. Off, because the bottom of the screen is where the taskbar and the dock already are.
- **`sidebar.cornerOpen.valueScroll`** — Scrolling on a left corner changes the brightness and on a right corner the volume.
- **`sidebar.cornerOpen.clickless`** — Open on hover rather than on a click. Faster, and much easier to trigger by accident, which is why it is off.
- **`sidebar.cornerOpen.cornerRegionWidth`** — A wide, thin strip rather than a square: what makes a corner reachable is being able to throw the pointer at the edge.
- **`sidebar.cornerOpen.visualize`** — Paint the regions, for working out where they actually are.
- **`sidebar.cornerOpen.topLeftAction`** — Which `GlobalStates` flag each corner flips. Empty means the corner does nothing and gets no region at all.

## `time`

Clock and date formats, and which day a week starts on.

| Setting                   | Value        | Default        |
| ------------------------- | ------------ | -------------- |
| `time.format`             | text         | `"HH:mm"`      |
| `time.dateFormat`         | text         | `"ddd, dd/MM"` |
| `time.weekStartsOnMonday` | true / false | `true`         |

## `wallpaperSelector`

The wallpaper picker: the folder it reads, how it is laid out, and the online sources it can search.

| Setting                                 | Value        | Default                                   |
| --------------------------------------- | ------------ | ----------------------------------------- |
| `wallpaperSelector.userPath`            | text         | `""`                                      |
| `wallpaperSelector.columns`             | whole number | `4`                                       |
| `wallpaperSelector.showSearchbar`       | true / false | `true`                                    |
| `wallpaperSelector.closeAfterSelection` | true / false | `true`                                    |
| `wallpaperSelector.changeInterval`      | whole number | `0`                                       |
| `wallpaperSelector.extensions`          | list of text | `["jpg","jpeg","png","webp","bmp","gif"]` |

- **`wallpaperSelector.changeInterval`** — Seconds between automatic wallpaper rotations; 0 disables it.

### `wallpaperSelector.online`

| Setting                                    | Value        | Default       |
| ------------------------------------------ | ------------ | ------------- |
| `wallpaperSelector.online.enable`          | true / false | `true`        |
| `wallpaperSelector.online.defaultProvider` | text         | `"wallhaven"` |
| `wallpaperSelector.online.resolution`      | text         | `"1080p"`     |
| `wallpaperSelector.online.purity`          | text         | `"sfw"`       |
| `wallpaperSelector.online.category`        | text         | `"general"`   |
| `wallpaperSelector.online.downloadPath`    | text         | `""`          |

- **`wallpaperSelector.online.defaultProvider`** — `"wallhaven"` | `"unsplash"` | `"pexels"`
- **`wallpaperSelector.online.resolution`** — `"1080p"` | `"2k"` | `"4k"`
- **`wallpaperSelector.online.purity`** — Wallhaven purity filter: `"sfw"` | `"sketchy"` | `"nsfw"`.
- **`wallpaperSelector.online.downloadPath`** — Where downloads land; empty means `%USERPROFILE%\Pictures\Wallpapers`.

## `weather`

Where the weather is fetched for, and in which units.

| Setting                   | Value        | Default |
| ------------------------- | ------------ | ------- |
| `weather.enable`          | true / false | `true`  |
| `weather.city`            | text         | `""`    |
| `weather.useUscUnits`     | true / false | `false` |
| `weather.refreshInterval` | whole number | `900`   |

- **`weather.city`** — Empty means the location is resolved from the public IP.

## `windows`

The things that only mean anything on Windows: the system taskbar, starting with the machine, and which window manager the workspaces come from.

| Setting                     | Value        | Default  |
| --------------------------- | ------------ | -------- |
| `windows.windowManager`     | text         | `"auto"` |
| `windows.hideSystemTaskbar` | true / false | `false`  |
| `windows.startWithWindows`  | true / false | `false`  |
| `windows.backdrop`          | text         | `"auto"` |

- **`windows.windowManager`** — `"auto"` probes for GlazeWM then komorebi; `"none"` disables workspace integration entirely.
- **`windows.hideSystemTaskbar`** — Hide the stock Windows taskbar while the shell's own bar is running.
- **`windows.backdrop`** — Blur behind panels: `"auto"` picks Mica on Windows 11 and Acrylic on Windows 10; `"acrylic"`, `"mica"` and `"none"` force one.

### `windows.glazewm`

| Setting                | Value        | Default |
| ---------------------- | ------------ | ------- |
| `windows.glazewm.port` | whole number | `6123`  |

### `windows.komorebi`

| Setting                     | Value | Default      |
| --------------------------- | ----- | ------------ |
| `windows.komorebi.pipeName` | text  | `"komorebi"` |

## `workSafety`

Blanking the wallpaper when its filename matches a keyword, for a screen other people can see.

| Setting                     | Value        | Default |
| --------------------------- | ------------ | ------- |
| `workSafety.blankWallpaper` | true / false | `false` |
| `workSafety.keywords`       | list of text | `[]`    |

- **`workSafety.blankWallpaper`** — Replace the wallpaper with a flat colour when its filename matches one of the keywords below.
