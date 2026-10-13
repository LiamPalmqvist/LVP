# Implementation status: NotVLC

**Last updated:** 2026-10-13

Status legend: `[ ]` not started · `[~]` in progress · `[x]` done · `[!]` blocked

> Maintained by both humans and Claude. Update a row in the same change that alters its state.
> IDs (`M1`, `O2`, …) are permanent. Reference them in commits, branches, and issues. Never
> renumber; retire an ID instead.

---

## End goal

A user can install VLC 3, run NotVLC, and do everything they could do in VLC media player (play any
file, disc or stream; pick tracks and subtitles; sync audio and subtitles; apply VLC's audio and
video effects; manage a queue and playlists; browse local and network media; record, snapshot,
convert and cast; configure every preference) through an interface that matches the design file
screen for screen.

**Done means:** every M-feature below is `[x]`, `cargo test` and `cargo clippy --all-targets -- -D
warnings` pass, and each screen's snapshot (`examples/snapshot.rs`) matches its frame in
`design/frames/`.

---

## Progress

| | Done | Total |
|---|---|---|
| Mandatory | 0 | 8 |
| Optional | 0 | 8 |

---

## Mandatory features

The end goal is not met until every one of these is `[x]`.

### M1: Playback core  `[~]`

Open files, folders, URLs and discs; play, pause, stop, seek, previous/next, speed, frame step,
A-B loop, jumps, titles and chapters, fullscreen, hide UI; video drawn in the redesigned player.

- **Acceptance:**
  - [x] `tests/engine_smoke.rs` plays a generated clip: frames reach the UI buffer, seek and end work
  - [x] Control bar matches the Player frame (`187:328`) in a snapshot
  - [ ] Open file/folder/disc/network dialogs open media and switch to the player
  - [ ] Fullscreen, hide UI and picture-in-picture work in the running app
- **Touches:** `infra/vlc/engine.rs`, `app/mod.rs`, `ui/shell/`
- **Depends on:** none
- **Notes:** engine, queue logic, shortcuts and control bar are in; dialogs pending.

### M2: Tracks and sync  `[~]`

Audio, video and subtitle track selection; external subtitles; audio and subtitle delay; subtitle
speed and duration factor; subtitle styling.

- **Acceptance:**
  - [x] Engine reports track lists and switches tracks (`EngineCommand::Set*Track`)
  - [ ] Track popovers and the Subtitles quick-settings tab match their design frames
  - [ ] Subtitle styling options reach libvlc (freetype args) and apply on the next file
- **Touches:** `infra/vlc/engine.rs`, `ui/panels/`
- **Depends on:** M1

### M3: Audio and video effects  `[~]`

Equalizer with VLC's presets, compressor, spatializer, stereo widener, spatial audio, pitch;
picture adjust, sharpen, banding removal, grain, colour effects, crop, deinterlace, aspect, zoom.

- **Acceptance:**
  - [x] `infra/vlc/filters.rs` maps every effect to VLC module variables (unit-tested)
  - [ ] Effect panels match `Audio - *` and `Video - *` component frames
  - [ ] Changing a slider changes playback live (libvlccore available)
- **Touches:** `domain/effects.rs`, `infra/vlc/filters.rs`, `ui/panels/`
- **Depends on:** M1

### M4: Playlists and queue  `[~]`

Create, edit, import and export playlists (M3U, PLS, XSPF); repeat and shuffle; the queue,
bookmarks and chapters panel.

- **Acceptance:**
  - [x] Queue next/previous/repeat/shuffle rules unit-tested (`domain::playlist`)
  - [x] Playlist formats round-trip (`domain::playlist_format`)
  - [ ] Queue panel and playlist pages match their frames
- **Touches:** `domain/playlist*.rs`, `app/library.rs`, `ui/pages/`, `ui/panels/`
- **Depends on:** M1

### M5: Library  `[~]`

Home (continue watching, listen again, frequent folders), Videos, Music, Recent, folders; grid and
list views; sort and search; thumbnails and metadata.

- **Acceptance:**
  - [x] History, resume points and folder listings unit-tested (`app::library`)
  - [x] Thumbnails and metadata come from libvlc (smoke test)
  - [ ] Pages match the library frames (`970:2703` and siblings) in snapshots
- **Touches:** `app/library.rs`, `app/workers.rs`, `ui/pages/`
- **Depends on:** M1

### M6: Network  `[~]`

Open network stream dialog; UPnP, SMB, SAP, Bonjour and podcast discovery; browsing shares with
VLC's login prompts.

- **Acceptance:**
  - [x] Engine lists discovery services and browses containers (`Helper::services`, `browse`)
  - [x] libvlc login/question/progress dialogs are bridged to events
  - [ ] Network page and Open Network Stream dialog match `6216:13614`
- **Touches:** `infra/vlc/helper.rs`, `infra/vlc/dialogs.rs`, `ui/pages/`, `ui/dialogs/`
- **Depends on:** M1

### M7: Settings  `[~]`

All thirteen settings pages with their advanced sections, dark/light/system theme, accent colours,
keyboard shortcut editor, persistence.

- **Acceptance:**
  - [x] Every option on every page exists in `domain/settings.rs` and round-trips through JSON
  - [x] Options libvlc reads at start-up are passed as instance args (`infra/vlc/args.rs`)
  - [ ] Each page matches its `Settings - *` frame in a snapshot
  - [ ] Keyboard Shortcuts dialog (`2437:14832`) edits bindings with conflict warnings
- **Touches:** `domain/settings.rs`, `infra/vlc/args.rs`, `ui/settings/`
- **Depends on:** none

### M8: Player chrome  `[~]`

Customisable toolbar, player and file context menus, mini player, music view.

- **Acceptance:**
  - [x] Toolbar layout model with insert/remove rules (`domain::toolbar`)
  - [x] Control bar draws every toolbar item
  - [ ] Customize Player dialog (`2160:6137`) edits the layout by drag and drop
  - [ ] Right-click menu (`6065:13616`), mini player and music view match their frames
  - [x] Mini player matches `970:3091` in every variant (default, hover, queue open, hover with
    queue, ended) and sits in the library's corner as in `970:2703`
- **Touches:** `ui/shell/controls.rs`, `ui/panels/`, `ui/music/`
- **Depends on:** M1

---

## Optional features

Not required for the end goal. Do not start one while any `M*` is unfinished unless the user
explicitly asks.

### O1: Onboarding wizard  `[ ]`

The eleven-step first-run flow from the Onboarding frames.

- **Value:** sets theme, accent, folders and key preferences before first use.
- **Depends on:** M7
- **Notes:** none

### O2: Convert and stream  `[~]`

VLC's Convert/Save and Stream with profiles (`Profiles` frame).

- **Value:** one of VLC's most used tools beyond playback.
- **Depends on:** M1
- **Notes:** engine side done (`Helper::start_job` runs a sout chain with progress events).

### O3: Record and snapshot  `[~]`

Record the playing input; save snapshots with the Video page's snapshot options.

- **Value:** parity with VLC's advanced controls.
- **Depends on:** M1
- **Notes:** both wired in `App` (`toggle_record`, `snapshot`).

### O4: Chromecast and renderers  `[~]`

Discover renderers and cast the current item ("Render" in video quick settings).

- **Value:** casting without leaving the player.
- **Depends on:** M1
- **Notes:** renderer discovery and `SetRenderer` done in the engine.

### O5: Audio visualiser  `[ ]`

VLC visualisations (spectrometer, scope, spectrum, VU meter, Goom) and the music view's visualiser.

- **Value:** the music view in the design centres on it.
- **Depends on:** M8

### O6: Batch subtitle matching  `[ ]`

"Add Subtitles to Playlist/Folder": match subtitle files to videos and attach them in bulk.

- **Value:** saves attaching subtitles one file at a time.
- **Depends on:** M2

### O7: Extensions  `[ ]`

Manage VLC Lua add-ons (playlist parsers, discovery, metadata) in the user's VLC Lua folders.

- **Value:** extends what libvlc can open (site parsers).
- **Depends on:** M7

### O8: Conversion profiles  `[ ]`

Create and edit Convert profiles (codec, bitrate, container) as in the Profiles frame.

- **Value:** custom output formats.
- **Depends on:** O2

---

## Out of scope

| Item | Why not |
|---|---|
| VLC's skins (`.vlt`) | The skins2 interface is a separate GUI; the setting is kept but has no effect |
| Wallpaper mode | Windows-only in VLC |
| Program (service ID) selection | libvlc 3 has no API for it; revisit with libvlc 4 |
| VLC's web interface | It controls VLC's internal playlist, not a libvlc player; a NotVLC remote would be new work |

---

## Foundations

- [x] Repository initialised: Rust, layered
- [x] Build, test, and lint commands verified
- [x] Walking skeleton: libvlc loads, plays a clip into the UI buffer, the player view renders it
- [x] Design tooling: .fig decoder, renderer, icon exporter, headless snapshot example
- [ ] CI pipeline
- [ ] Release / packaging (macOS .app bundle)

---

## Open questions

| # | Question | Blocks | Raised |
|---|---|---|---|

---

## Decision log

| Date | Decision | Why |
|---|---|---|
| 2026-10-13 | Architecture: layered with ports | libvlc FFI and threading stay out of screen code; fake engine for UI snapshots |
| 2026-10-13 | Language: Rust | User's choice |
| 2026-10-13 | Engine: libvlc 3 from VLC.app, not mpv | The design's effect panels map 1:1 to VLC modules; libvlc brings VLC's discovery, casting, recording and transcoding |
| 2026-10-13 | UI: eframe/egui 0.33 with wgpu | Custom-drawn design kit; native wgpu texture for video. 0.33 is the newest that builds on Rust 1.90 |
| 2026-10-13 | Video via vmem callbacks, geometry in our renderer | Works with any GPU backend; aspect/crop/zoom stay consistent |
