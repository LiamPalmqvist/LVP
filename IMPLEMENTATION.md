# Implementation status: NotVLC

**Last updated:** 2026-10-16

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
| Mandatory | 8 | 8 |
| Optional | 7 | 8 |

---

## Mandatory features

The end goal is not met until every one of these is `[x]`.

### M1: Playback core  `[x]`

Open files, folders, URLs and discs; play, pause, stop, seek, previous/next, speed, frame step,
A-B loop, jumps, titles and chapters, fullscreen, hide UI; video drawn in the redesigned player.

- **Acceptance:**
  - [x] `tests/engine_smoke.rs` plays a generated clip: frames reach the UI buffer, seek and end work
  - [x] Control bar matches the Player frame (`187:328`) in a snapshot
  - [x] Open file/folder/disc/network dialogs open media and switch to the player
  - [x] Fullscreen, hide UI and picture-in-picture toggle from their shortcuts and Esc leaves them (`tests/player_window.rs`)
- **Touches:** `infra/vlc/engine.rs`, `app/mod.rs`, `ui/shell/`
- **Depends on:** none
- **Notes:** engine, queue logic, shortcuts and control bar are in; Open Disc, Open Capture Device,
  Open Network Stream and Jump to Time dialogs are in `ui/dialogs/` (MRLs in `domain::disc`).

### M2: Tracks and sync  `[x]`

Audio, video and subtitle track selection; external subtitles; audio and subtitle delay; subtitle
speed and duration factor; subtitle styling.

- **Acceptance:**
  - [x] Engine reports track lists and switches tracks (`EngineCommand::Set*Track`)
  - [x] Track popovers and the Subtitles quick-settings tab match their design frames
  - [x] Subtitle styling options reach libvlc (freetype args, `args::tests::subtitle_style_reaches_freetype`); a settings change rebuilds the instance and resumes playback (`tests/engine_features.rs`)
- **Touches:** `infra/vlc/engine.rs`, `ui/panels/`
- **Depends on:** M1
- **Notes:** panels done (`ui/panels/tracks.rs`, `quick.rs`); "Show Subtitle Timeline on Seekbar" is stored in `PanelsState::subtitle_timeline` for the seek bar to read.

### M3: Audio and video effects  `[x]`

Equalizer with VLC's presets, compressor, spatializer, stereo widener, spatial audio, pitch;
picture adjust, sharpen, banding removal, grain, colour effects, crop, deinterlace, aspect, zoom.

- **Acceptance:**
  - [x] `infra/vlc/filters.rs` maps every effect to VLC module variables (unit-tested)
  - [x] Effect panels match `Audio - *` and `Video - *` component frames
  - [x] Changing an effect changes playback live: enabling Negate mid-playback inverts the decoded picture (`tests/engine_features.rs`)
- **Touches:** `domain/effects.rs`, `infra/vlc/filters.rs`, `ui/panels/`
- **Depends on:** M1
- **Notes:** quick settings Video/Audio tabs and effect cards in `ui/panels/video_fx.rs`, `audio_fx.rs`; named effect presets in `app/presets.rs`. Live effects verified against libvlc in `tests/engine_features.rs`.

### M4: Playlists and queue  `[x]`

Create, edit, import and export playlists (M3U, PLS, XSPF); repeat and shuffle; the queue,
bookmarks and chapters panel.

- **Acceptance:**
  - [x] Queue next/previous/repeat/shuffle rules unit-tested (`domain::playlist`)
  - [x] Playlist formats round-trip (`domain::playlist_format`)
  - [x] Queue panel and playlist pages match their frames
  - [x] Playlist pages: list, detail with play/shuffle/add/rename/export/delete, drag to reorder,
        import, New Playlist dialog (`6259:14954`) and the playlist context menu (`2459:9389`)
- **Touches:** `domain/playlist*.rs`, `app/library.rs`, `ui/pages/`, `ui/panels/`
- **Depends on:** M1
- **Notes:** the Queue / Bookmarks / Chapters popover matches `491:1780` (`ui/panels/queue.rs`); playlist
  pages done (`ui/pages/`, `app/pages.rs`).

### M5: Library  `[x]`

Home (continue watching, listen again, frequent folders), Videos, Music, Recent, folders; grid and
list views; sort and search; thumbnails and metadata.

- **Acceptance:**
  - [x] History, resume points and folder listings unit-tested (`app::library`)
  - [x] Thumbnails and metadata come from libvlc (smoke test)
  - [x] Pages match the library frames (`970:2703` and siblings) in snapshots
  - [x] Sorting follows the design's options (`domain::browse`, unit-tested); page models unit-tested
- **Touches:** `app/library.rs`, `app/workers.rs`, `ui/pages/`
- **Depends on:** M1
- **Notes:** grid (`970:2703`), list (`970:3308`), Home (`6261:15078`) and the player side column
  (`187:328`, `848:1996`) render from `--view library --page season|home [--list]` and
  `--view player --side-panel`. Music adds Songs/Albums/Artists/Genres from tags.

### M6: Network  `[x]`

Open network stream dialog; UPnP, SMB, SAP, Bonjour and podcast discovery; browsing shares with
VLC's login prompts.

- **Acceptance:**
  - [x] Engine lists discovery services and browses containers (`Helper::services`, `browse`)
  - [x] libvlc login/question/progress dialogs are bridged to events
  - [x] Network page and Open Network Stream dialog match `6216:13614` (`--page network`, `--dialog open_network`)
  - [x] Network page: services by category, service items, containers browsed on a worker thread,
        recent streams (`ui/pages/network.rs`); the dialog itself lives in `ui/dialogs/`
  - [x] VLC's login, question, progress and error prompts are shown and answered (`--vlc-dialog login`)
- **Touches:** `infra/vlc/helper.rs`, `infra/vlc/dialogs.rs`, `ui/pages/`, `ui/dialogs/`
- **Depends on:** M1

### M7: Settings  `[x]`

All thirteen settings pages with their advanced sections, dark/light/system theme, accent colours,
keyboard shortcut editor, persistence.

- **Acceptance:**
  - [x] Every option on every page exists in `domain/settings.rs` and round-trips through JSON
  - [x] Options libvlc reads at start-up are passed as instance args (`infra/vlc/args.rs`)
  - [x] Each page matches its `Settings - *` frame in a snapshot
  - [x] Keyboard Shortcuts dialog (`2437:14832`) edits bindings with conflict warnings
- **Touches:** `domain/settings.rs`, `infra/vlc/args.rs`, `ui/settings/`
- **Depends on:** none

### M8: Player chrome  `[x]`

Customisable toolbar, player and file context menus, mini player, music view.

- **Acceptance:**
  - [x] Toolbar layout model with insert/remove rules (`domain::toolbar`)
  - [x] Control bar draws every toolbar item
  - [x] Customize Player dialog (`2160:6137`) edits the layout by drag and drop
  - [x] Right-click menu (`6065:13616`), mini player and music view match their frames
  - [x] Mini player matches `970:3091` in every variant (default, hover, queue open, hover with
    queue, ended) and sits in the library's corner as in `970:2703`
  - [x] Music view matches `4767:23726` and `4778:25123` (with the library column), in the light
    theme, in hide-UI and at picture-in-picture size
  - [x] Music seek bar draws real loudness peaks (`App::waveform`, VLC transcodes to a 2 kHz mono
    WAV in the background; peaks cached under the cache dir) as in control bar `491:1780`
- **Touches:** `ui/shell/controls.rs`, `ui/panels/`, `ui/music/`
- **Depends on:** M1
- **Notes:** right-click menu in `ui/panels/menu.rs`, mini player and music view in `ui/music/`.

---

## Optional features

Not required for the end goal. Do not start one while any `M*` is unfinished unless the user
explicitly asks.

### O1: Onboarding wizard  `[x]`

The eleven-step first-run flow from the Onboarding frames.

- **Value:** sets theme, accent, folders and key preferences before first use.
- **Depends on:** M7
- **Notes:** `ui/dialogs/onboarding.rs` (`--onboarding`, `--dialog onboarding_theme`,
  `onboarding_shortcuts`). The eleven frames are three steps: welcome with a media folder
  (`1899:3055`), theme and accent over a player preview (`1902:3670` and eight accent variants), and
  keyboard shortcuts (`1902:6770`, shown from the live bindings; pressing a key lights its row).
  Skip, Esc or Finish set `general.onboarding_done`.

### O2: Convert and stream  `[x]`

VLC's Convert/Save and Stream with profiles (`Profiles` frame).

- **Value:** one of VLC's most used tools beyond playback.
- **Depends on:** M1
- **Notes:** engine side done (`Helper::start_job` runs a sout chain with progress events). Convert /
  Save and Stream dialog (`--dialog convert`): current item, picked files or the queue; file, HTTP,
  RTSP, RTP or UDP destinations; job list with progress and cancel. Chains built and tested in
  `domain::convert`.

### O3: Record and snapshot  `[x]`

Record the playing input; save snapshots with the Video page's snapshot options.

- **Value:** parity with VLC's advanced controls.
- **Depends on:** M1
- **Notes:** `App::toggle_record` uses the input's record switch through libvlccore; a recording is
  written to the record folder in `tests/engine_features.rs`. `App::snapshot` saves the current frame
  with the Video page's snapshot options.

### O4: Chromecast and renderers  `[~]`

Discover renderers and cast the current item ("Render" in video quick settings).

- **Value:** casting without leaving the player.
- **Depends on:** M1
- **Notes:** renderer discovery and `SetRenderer` done in the engine; the Render dropdown in quick
  settings lists renderers. Not yet verified against a real Chromecast (none available here).

### O5: Audio visualiser  `[x]`

VLC visualisations (spectrometer, scope, spectrum, VU meter, Goom) and the music view's visualiser.

- **Value:** the music view in the design centres on it.
- **Depends on:** M8
- **Notes:** the music view's own bar visualiser is done (`ui/music/visualiser.rs`): libvlc 3 gives
  no samples without replacing its audio output, so it follows a smooth pseudo-signal scaled by the
  track's waveform loudness at the playhead, and settles when paused. VLC's own visualisations show
  as a picture through the shell when `audio.visualizer` is set (checked with the Spectrum module in
  `tests/engine_features.rs`).

### O6: Batch subtitle matching  `[x]`

"Add Subtitles to Playlist/Folder": match subtitle files to videos and attach them in bulk.

- **Value:** saves attaching subtitles one file at a time.
- **Depends on:** M2
- **Notes:** Add Subtitles dialog (`--dialog add_subtitles`, `add_subtitles_file`,
  `add_subtitles_online`): to the playing file, a folder of videos (subtitles copied next to each video
  under its name after confirmation) or the queue (`:sub-file=` options). Matching levels follow VLC's
  sub-autodetect-fuzzy (`domain::subtitles`). "From OpenSubtitles" explains it is unavailable (VLC uses
  the VLSub Lua extension, which libvlc cannot run).

### O7: Extensions  `[x]`

Manage VLC Lua add-ons (playlist parsers, discovery, metadata) in the user's VLC Lua folders.

- **Value:** extends what libvlc can open (site parsers).
- **Depends on:** M7
- **Notes:** Settings > Extensions lists, installs (by kind), enables/disables (renames to `*.disabled`)
  and removes scripts in VLC's user Lua folder. "Load add-ons on startup" is stored only: libvlc 3
  has no switch to skip user Lua scripts.

### O8: Conversion profiles  `[x]`

Create and edit Convert profiles (codec, bitrate, container) as in the Profiles frame.

- **Value:** custom output formats.
- **Depends on:** O2
- **Notes:** VLC's standard profiles plus an editor (`--dialog convert_profile`), stored under the
  store key "profiles" (`App::conversion_profiles`). The `Profiles` frame (`6648:20666`) is in fact
  the app menu's Profile submenu (Movies, Shows, Music, Work), i.e. preference profiles, not
  conversion profiles; that menu is not built.

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
- [x] System integration: media keys and Now Playing, single instance, display sleep prevention,
      track notifications
- [ ] CI pipeline
- [x] Release / packaging: `tools/bundle-macos.sh` builds `dist/NotVLC.app` with document types; files
      opened from Finder reach the queue (Apple Events, `infra/macos_open.rs`)

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
| 2026-10-16 | Software decoding while subtitles are shown (hardware decoding on Automatic) | VLC 3 cannot draw subtitles onto VideoToolbox pictures through vmem; files with subtitles open in software mode and turning subtitles on reopens in place |
