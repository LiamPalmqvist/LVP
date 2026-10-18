# NotVLC

A media player with all of VLC's features and a redesigned interface, written in Rust on top of
VLC's own engine. This design was made by the YouTuber [Juxtopposed](https://www.youtube.com/@juxtopposed) in their video [I Redesigned VLC After 25 Years](https://www.youtube.com/watch?v=ERFeOgqk324)

All credit to the design goes to Juxtopposed.

# ALL CODE IS AI GENERATED
If you want to make improvements to this project, that is entirely welcome. I have to add this disclaimer otherwise I will be crucified by reddit. 

## Install

NotVLC plays everything through VLC's own engine, so it needs 64-bit VLC 3 installed alongside it.
The packages below take care of that, or say how.

### macOS (Apple silicon)

```bash
brew install --cask liampalmqvist/notvlc/notvlc
```

The cask expects VLC in `/Applications` or `~/Applications`. If you don't have it yet, run
`brew install --cask vlc` or get it from [videolan.org](https://www.videolan.org/vlc/).
`brew upgrade --cask notvlc` updates NotVLC. `brew uninstall --cask --zap notvlc` removes it along
with its settings and library.

### Arch Linux

```bash
yay -S notvlc        # built from source
yay -S notvlc-bin    # the release binary (x86_64)
```

Any AUR helper works. Both packages pull in `libvlc` and VLC's plugins.

### Windows

```powershell
winget install LiamPalmqvist.NotVLC
```

winget installs 64-bit VLC with it. NotVLC is installed as a portable app, so start it by running
`notvlc` in a terminal; it doesn't add a Start menu entry yet.

### Any other system, or by hand

Every [release](https://github.com/LiamPalmqvist/NotVLC/releases) has a build for each system:

- **macOS**: `notvlc-<version>-macos-arm64.zip`. Unzip it and move `NotVLC.app` to Applications.
  The app isn't notarized, so clear the download's quarantine flag before the first launch:
  `xattr -dr com.apple.quarantine /Applications/NotVLC.app`.
- **Linux** (x86_64): `notvlc-<version>-linux-x86_64.tar.gz`. Unpack it and run `./install.sh`,
  which installs to `~/.local`; `sudo PREFIX=/usr/local ./install.sh` installs it for every user.
  Install VLC 3 from your distribution too (for example `sudo apt install vlc`).
- **Windows** (x64): `notvlc-<version>-windows-x64.zip`. Unzip it and run `NotVLC.exe`, with 64-bit
  VLC 3 from [videolan.org](https://www.videolan.org/vlc/) installed.

NotVLC looks for VLC in its usual install location on each system. If yours is elsewhere, set
`NOTVLC_LIBVLC` to the folder that holds libvlc.

[`packaging/README.md`](packaging/README.md) describes how the packages are built and kept up to
date with each release.

## Build from source

You need Rust 1.90 or newer and VLC 3 (`/Applications/VLC.app` on macOS; see above for
`NOTVLC_LIBVLC`). The smoke test also uses `ffmpeg` on `PATH` if it is there.

```bash
cargo run --release              # open the player
cargo run --release -- movie.mkv # open files directly
```

`tools/bundle-macos.sh`, `tools/build-linux.sh` and `tools/build-windows.ps1` build the same
packages a release ships.

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
