# beautiful-wallpaper

A Material 3 desktop shell for Windows 10 and 11 — the experience of
[end4-pC](https://github.com/pctrade/end4-pC) and
[illogical-impulse](https://github.com/end-4/dots-hyprland), rebuilt on Windows'
own APIs.

日本語版は [README.ja.md](README.ja.md) にあります。

Your wallpaper picks the colours. Everything else — the desktop widgets, the
picker, the panels still to come — follows from there.

<!-- Rendered from the surfaces themselves; regenerate with `pnpm screenshots`. -->

![The desktop and the wallpaper picker](docs/images/03-both.jpg)

## Why this is a rewrite, not a port

end4-pC is a [Quickshell](https://quickshell.org) configuration: 65,000 lines of
QML that talk to Hyprland over its IPC socket and place every panel through
`wlr-layer-shell`. None of that exists on Windows — not the runtime, not the
protocol, not the compositor. So the design carries over and the code does not.

What the original does with a Wayland protocol, this does with Win32:

| end4-pC                                   | here                                                 |
| ----------------------------------------- | ---------------------------------------------------- |
| `WlrLayer.Bottom` wallpaper layer         | a window reparented under `WorkerW`                  |
| `exclusiveZone` on a bar                  | `SHAppBarMessage`                                    |
| `WlrLayer.Overlay` panels                 | topmost `WS_EX_TOOLWINDOW｜WS_EX_NOACTIVATE` windows |
| `mask: Region` input passthrough          | `WS_EX_TRANSPARENT`                                  |
| `switchwall.sh` → matugen → `colors.json` | the `material-colors` crate, in process              |
| MPRIS                                     | the Windows media session (SMTC)                     |
| UPower, PipeWire, `/proc`                 | `GetSystemPowerStatus`, WASAPI, `sysinfo`            |
| Hyprland workspaces                       | GlazeWM / komorebi, when one is running              |
| `IpcHandler` targets                      | a named pipe; six target names carry over            |

The seams the original chose turn out to travel well. Its config key names, its
`colors.json` shape and most of its state flags are kept, so muscle memory
mostly carries over — but not all of it does, and
[docs/differences.md](docs/differences.md) is the list of what changed and why:
the keys Windows refuses to hand over, the two state flags that were renamed,
and the six IPC targets that still mean the same thing.

## What works today

- **Wallpaper-driven Material 3 theming.** The dominant colour of the wallpaper
  is quantised and scored, a scheme variant is chosen to suit the image, and the
  full role set — plus a `success` quad and sixteen terminal colours — is
  derived. Optionally pushed into the Windows accent colour and Windows Terminal.
- **The background surface**: the wallpaper with GPU transitions, and desktop
  widgets — clock, media, weather, CPU/RAM/disk, calendar, user card — that can
  be dragged and snapped to a grid.
- **The wallpaper picker**: the local folder with history and thumbnails, plus
  Wallhaven, Unsplash and Pexels.
- **A volume readout**, driven by a WASAPI callback so it appears on the
  keypress rather than on the next poll.
- **Notification toasts**, grouped by application and swipe-dismissable, over a
  persisted history.
- **The bar**, reserving its edge through `SHAppBarMessage` so maximised windows
  keep clear of it. Four styles, horizontal or vertical, with workspaces,
  active window, clock, weather, tray, battery, network, resources, media and
  utility buttons — laid out by `bar.left/center/right`.
- **The two sidebars**: the right one's quick toggles, sliders, night light and
  notification centre; the left one's AI chat, translator, media and image-board
  tabs.
- **The dock**, with pinned and running applications.
- **The overview and launcher**, searching applications, files, the web and a
  calculator, with `/` actions.
- **Screenshots**: a region picker, OCR and an on-screen translator.
- **The session screen, the desktop menu, the drop shelf and a floating
  overlay** with a crosshair.
- **A settings screen** whose form is generated from the config schema, so a new
  setting has a control the moment it exists.
- **Presets** — whole configurations saved by name, and a first-run screen that
  reports which of your keys Windows refused to register.
- **An installer** that puts the machine back on uninstall: the taskbar, the
  autostart entry, the App Paths key.
- **Config as one JSON file**, watched both ways: edit it in any editor and the
  shell follows. Every key is in [docs/config.md](docs/config.md).
- **A CLI** — `bw wallpapers apply <path>`, `bw config set bar.bottom true` — for
  hotkeys and scripts.
- **Fourteen locales' worth of plumbing**, with English and Japanese filled in.

What is missing is mostly at the edges: per-monitor surfaces, eight of the
fourteen wallpaper transitions, drag-to-reorder in the dock, and the audio
visualiser. [docs/roadmap.md](docs/roadmap.md) has the full list, including what
is deliberately not built.

## Documentation

|                                                         |                                                                 |
| ------------------------------------------------------- | --------------------------------------------------------------- |
| [Configuration reference](docs/config.md)               | every key, its type and its default — generated from the schema |
| [What is different from end4-pC](docs/differences.md)   | keys, workspaces, notifications, and what still means the same  |
| [When something does not work](docs/troubleshooting.md) | an empty tray, a key Windows refuses, a hidden taskbar          |
| [The MSIX sparse package](docs/msix.md)                 | reading other applications' notifications, and what it costs    |
| [Roadmap](docs/roadmap.md)                              | what is done, what is not, and what will not be                 |

## Install

Take the `.exe` from [the latest release](https://github.com/C0uki/beautiful-wallpaper/releases),
or the artifact from a CI run for something newer than the last tag. It installs
for the current user, so it does not ask for an administrator.

**The installers are not signed**, so SmartScreen will say _"Windows protected
your PC"_ and put the Run button behind **More info**. That is what Windows says
about any installer whose publisher it cannot verify, and it is the right
default — signing needs a code-signing certificate this project does not have.
Building it yourself avoids the question entirely:

```powershell
pnpm install
pnpm --filter @bw/shell app:build
```

The result lands in `target/release/bundle/`. Windows 10 and 11 are both
supported; the shell needs [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/),
which Windows 11 already has.

Releases are cut by tagging: `git tag v0.1.0 && git push origin v0.1.0` builds
the installers and attaches them to a draft release, which stays a draft until
somebody publishes it. The tag has to match the version in `tauri.conf.json` —
the workflow refuses a mismatch rather than shipping a `v0.2.0` release full of
`0.1.0` installers.

## Develop

The UI is a webview, so most of it can be built and reviewed without Windows.
`pnpm dev` serves every surface against a mock backend:

```bash
pnpm install
pnpm dev            # all surfaces, side by side, with fake system data
pnpm screenshots    # render each surface to screenshots/
```

For the real thing, on Windows:

```powershell
pnpm --filter @bw/shell app:dev
```

Checks:

```bash
pnpm lint && pnpm typecheck && pnpm test   # the frontend
cargo test -p bw-core                      # the portable core
cargo clippy --target x86_64-pc-windows-msvc --all-targets
```

That last one is the important one: it type-checks every Win32 and WinRT call
without needing a Windows machine or a cross compiler. `cargo check` for the
Windows target works from Linux and macOS too — only linking needs Windows.

The bundled icon font is a subset of Material Symbols covering only the icons
the shell draws, listed in `apps/shell/scripts/icons.json`. A name outside that
list renders as the literal word rather than a glyph, so adding an icon to the
UI means adding it there and running `pnpm gen:icons` (which needs `fonttools`
and `brotli`).

The Rust toolchain is pinned in `rust-toolchain.toml`, so rustup installs the
same compiler CI uses, along with clippy, rustfmt and the Windows target. That
matters for `-D warnings`: on a different version, a lint the runner enforces
may not exist locally, and the difference only shows up as a red CI job.

## Layout

```
crates/bw-core/          config schema, Material 3 pipeline, wallpaper indexing
                         — no Tauri, no Win32, tests run anywhere
apps/shell/src-tauri/    the Windows half: layering, providers, IPC, commands
apps/shell/src/          the surfaces (React), the widget kit, the mock backend
packages/core/           the Rust↔TS contract, generated from the Rust types
packages/tokens/         the derived token layer — Appearance.qml, in TypeScript
```

Run `pnpm gen:types` after changing anything in `crates/bw-core`: the TypeScript
types and the default config are generated from the Rust schema, and CI fails if
they are stale.

Adding or documenting a config key also means `pnpm gen:docs`, which rewrites
[docs/config.md](docs/config.md) from the schema — types and defaults from the
same values the shell reads, and the prose from the doc comments in
`config/schema.rs`. There is no second copy to keep in step, and CI fails if the
committed file does not match. A doc comment on a key is therefore the way to
document it.

## Licence

GPL-3.0-or-later, matching the projects this one derives from. See
[NOTICE](NOTICE) for the third-party material and for exactly what was and was
not taken from upstream.
