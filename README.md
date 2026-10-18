# NotVLC

A media player with all of VLC's features and a redesigned interface, written in Rust on top of
VLC's own engine. This design was made by the YouTuber [Juxtopposed](https://www.youtube.com/@juxtopposed) in their video [I Redesigned VLC After 25 Years](https://www.youtube.com/watch?v=ERFeOgqk324)

All credit to the design goes to Juxtopposed.

# ALL CODE IS AI GENERATED
If you want to make improvements to this project, that is entirely welcome. I have to add this disclaimer otherwise I will be crucified by reddit. 

## Prerequisites

- Rust 1.90 or newer
- VLC 3 installed (`/Applications/VLC.app` on macOS). NotVLC loads its libvlc at runtime; point
  `NOTVLC_LIBVLC` at another install if needed.
- Optional, for the smoke test: `ffmpeg` on `PATH`.

## Install and run

```bash
cargo run --release              # open the player
cargo run --release -- movie.mkv # open files directly
```

## Test

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

## Screenshots without a window

```bash
cargo run --example snapshot -- --view player player.png
cargo run --example snapshot -- --settings audio settings-audio.png
```

The design file lives in `design/`; see `tools/fig/README.md` for rendering its frames.

## Licence

GPL-2.0-or-later, matching libvlc. Inter and DM Mono are under the SIL Open Font License (see
`assets/fonts`).
