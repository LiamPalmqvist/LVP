# LVP

A desktop media player in Rust with every function of VLC, built on VLC's own engine (libvlc 3) and
drawn to match the "VLC Redesign (Community)" Figma file.

## End goal

A user can install VLC 3, run LVP, and do everything they could do in VLC media player (play any
file, disc or stream; pick tracks and subtitles; sync audio and subtitles; apply VLC's audio and
video effects; manage a queue and playlists; browse local and network media; record, snapshot,
convert and cast; configure every preference) through an interface that matches the design file
screen for screen. Done when every M-feature in `IMPLEMENTATION.md` is `[x]` and each screen's
snapshot matches its design frame.

**Scope tracker:** [`IMPLEMENTATION.md`](./IMPLEMENTATION.md) is the authoritative list of what is
built and what isn't. Read it before starting work.

## Stack

| | |
|---|---|
| Language | Rust 1.90 (edition 2024) |
| Package manager | cargo |
| Test framework | built-in `#[test]`, plus `egui_kittest` for headless UI renders |
| Lint / format | clippy / rustfmt (`rustfmt.toml`: width 120) |
| Key dependencies | eframe/egui 0.33 (wgpu renderer), libloading (libvlc at runtime), resvg (icons), serde, rfd, quick-xml |

egui is pinned to 0.33 because 0.34+ needs Rust 1.92+; do not bump it without upgrading the
toolchain.

## Commands

```bash
cargo build                                   # build
cargo test                                    # unit, UI and libvlc tests (libvlc ones skip if VLC/ffmpeg missing; silent)
cargo test --lib domain::playlist             # run one module's tests
cargo clippy --all-targets -- -D warnings     # lint
cargo fmt                                     # format
cargo run                                     # run the app (needs /Applications/VLC.app or LVP_LIBVLC)
cargo run --example snapshot -- --view player /tmp/p.png   # headless render of the UI with demo content
tools/bundle-macos.sh                         # release build packaged as dist/LVP.app with VLC's engine (--debug, --no-vlc)
tools/build-linux.sh                          # Linux tarball in dist/ (--install-deps, --debug)
gh workflow run build.yml                     # build macOS, Linux and Windows packages on GitHub Actions
```

`cargo run --example snapshot -- --help`-style flags are documented at the top of
`examples/snapshot.rs` (`--page`, `--panel`, `--settings`, `--dialog`, `--light`, `--accent`,
`--audio`, `--side-panel`...). Compare its output with the matching render in `design/frames/`.

## Architecture: layered

UI, application and domain layers, with libvlc and the filesystem as adapters behind traits. Chosen
because the hard parts (libvlc threading, FFI) must stay away from screen code, and the app layer
needs to run against a fake engine for snapshots and tests.

### Layout

```
src/
  main.rs          # wiring: JsonStore + VlcEngine + VlcProbe -> App -> LvpApp window
  domain/          # pure types and rules: media, settings (one struct per settings page),
                   # effects (VLC module params), queue/repeat/shuffle, shortcuts, toolbar,
                   # playlist file formats, library sorting
  app/             # App facade (everything the UI can do), library/history/playlists,
                   # background probe+thumbnail workers, ports.rs (MediaEngine, MediaProbe,
                   # Store traits), video.rs (frame hand-off), fake.rs (fake engine)
  infra/           # adapters: vlc/ (libvlc FFI, engine thread, helper instance, dialogs,
                   # filters, instance args), store.rs (JSON files)
  ui/              # egui: theme (Figma colour tokens), icons (generated from assets/icons),
                   # widgets (design control kit), shell/ (title bar, nav, control bar,
                   # player view), pages/, panels/, settings/, dialogs/, music/
assets/            # fonts (Inter, DM Mono) and the 208 design icons as SVG
design/            # the .fig file, FRAMES.md index; canvas.json and frames/ are generated
tools/fig/         # .fig decoder, renderer, text/geometry dumpers, icon exporter
examples/          # snapshot.rs: headless UI renders
tests/             # libvlc smoke test and demo fixtures
```

### The dependency rule

`domain/` imports nothing from this crate. `app/` imports `domain/` only. `ui/` imports `app/` and
`domain/`, never `infra/`. `infra/` imports `app/` (to implement `app::ports`) and `domain/`. Only
`main.rs` and test/example harnesses import `infra/`. libvlc symbols never appear outside
`infra/vlc/`.

### Where things go

| Adding... | Goes in | Notes |
|---|---|---|
| A player capability (new VLC feature) | `EngineCommand` in `app/ports.rs`, handled in `infra/vlc/engine.rs`, exposed as an `App` method | Live filter changes go through `filters.rs` and libvlccore vars |
| A preference | the page's struct in `domain/settings.rs`, flag in `infra/vlc/args.rs` if libvlc reads it at start-up | `App::update_settings` rebuilds the engine when args change |
| A keyboard shortcut | `actions!` table in `domain/shortcuts.rs`, behaviour in `app/actions.rs` | Window-level actions return a `UiCommand` |
| A toolbar button | `ToolbarItem` in `domain/toolbar.rs`, drawing in `ui/shell/controls.rs` | |
| A screen or panel | its module under `ui/` with its own state struct held by `UiState` | Use `ui/widgets.rs`; add a snapshot flag if it needs one |
| An icon | export from the .fig with `tools/fig/export_icons.py` | `build.rs` generates the `Icon` enum |

## Conventions

- **Naming:** colour tokens are named after the Figma styles (`fg_a2`, `bg_b2`, `bg_accent`...);
  use them through `widgets::pal(ctx)`, never hard-coded colours.
- **Errors:** `anyhow` in infra; the app turns failures into toasts (`App::toast_error`); the UI
  never panics on bad media.
- **Config:** settings persist as JSON in the platform data dir (`LVP_HOME` overrides it);
  `App::update_settings` is the only way to change settings from the UI.
- **Logging:** `log` macros, `RUST_LOG=lvp=debug` to see them; libvlc's own log goes to the
  file chosen in General > Diagnostics.
- **Tests:** pure logic gets inline `#[cfg(test)]` tests; libvlc behaviour is covered by
  `tests/engine_smoke.rs`; screens are checked with the snapshot example against `design/frames`.
- **Threads:** libvlc calls run on the engine thread (`infra/vlc/engine.rs`); the UI reads a cached
  `EngineStatus` and never blocks on libvlc. Background parsing and thumbnails use `app/workers.rs`.
- **Text:** no em-dashes anywhere (code, comments, docs, commits).

## Working agreement

**Keep `IMPLEMENTATION.md` current.** When a feature's state changes, update its row in the same
change that causes it, not later:

- starting work on it -> `[~]`
- merged and tested -> `[x]`, and tick its acceptance criteria
- blocked -> `[!]` with the blocker named in Notes
- newly discovered work -> add a row; mandatory vs optional is the user's call, so ask

**Scope discipline.** Mandatory features (`M*`) come before optional ones (`O*`). Anything in "Out
of scope" stays out unless the user moves it. If a task doesn't map to a tracked feature, say so
before building it.

**Follow the dependency rule above.** If a change seems to require breaking it, that's a signal the
architecture needs revisiting; raise it rather than routing around it.

## Project notes

- The app was called NotVLC until 2026-10-21 (Viddy was considered and dropped: sachaos/viddy, a
  `watch` replacement, owns that name on Homebrew and the AUR and installs /usr/bin/viddy). The
  Homebrew tap is still `LiamPalmqvist/homebrew-notvlc`. The logo is `assets/icons/logo.svg`, a
  placeholder until a real one exists: the title bar, About, onboarding, the empty player and the
  app icon all draw it. Do not bring back VLC's cone; it is VideoLAN's trademark.
- libvlc is loaded at runtime from `LVP_LIBVLC`, then the copy bundled next to the executable
  (packaged macOS and Windows builds; pinned in `tools/vlc-release.txt`), then a system install such
  as `/Applications/VLC.app`. libvlccore is preloaded first because libvlc links it through
  `@rpath`, and on macOS `VLC_DATA_PATH` is set because VLC 3 only finds its Lua scripts by itself
  when the host executable is named VLC. libvlccore also gives access to VLC object variables,
  which is how audio filters, video filters, recording and subtitle timing change live; without it
  only the equalizer and picture adjustments work live.
- On macOS the window keeps AppKit's frame (corners, shadow, traffic lights) with a transparent
  title bar; `infra/macos_window.rs` moves the traffic lights into the 40pt bar every frame, and
  `UiState::native_titlebar` makes the bar leave room for them. Elsewhere the app draws the
  design's window buttons.
- Video comes through libvlc's vmem callbacks as RGBA into a double buffer
  (`app/video.rs`) and is uploaded to an `Rgba8Unorm` wgpu texture (`ui/video.rs`). Aspect ratio,
  crop and zoom are applied by the renderer (`ui::video::fit`), not by VLC.
- The design's measurements are in the .fig: `tools/fig/geom.py design/canvas.json design/images
  GUID 5` prints boxes, fills, radii and fonts for any frame listed in `design/FRAMES.md`.
