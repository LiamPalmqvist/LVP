# LVP

LVP (Liam's Video Player) is a media player with all of VLC's features and a redesigned interface, written in Rust on top of
VLC's own engine. This design was made by the YouTuber [Juxtopposed](https://www.youtube.com/@juxtopposed) in their video [I Redesigned VLC After 25 Years](https://www.youtube.com/watch?v=ERFeOgqk324)

All credit to the design goes to Juxtopposed.

# ALL CODE IS AI GENERATED
If you want to make improvements to this project, that is entirely welcome. I have to add this disclaimer otherwise I will be crucified by reddit. 

## Install

LVP plays everything through VLC's own engine. The macOS and Windows packages include it; on Linux
it comes from your distribution's VLC packages.

### macOS (Apple silicon)

```bash
brew install --cask liampalmqvist/notvlc/lvp
```

Homebrew only loads casks from taps you trust, and naming the cask in full as above counts. Trust
the tap once with `brew trust liampalmqvist/notvlc` and a plain `brew upgrade` keeps LVP up to
date. `brew uninstall --cask --zap liampalmqvist/notvlc/lvp` removes it along with its settings and
library.

### Arch Linux

```bash
yay -S lvp        # built from source
yay -S lvp-bin    # the release binary (x86_64)
```

Any AUR helper works. Both packages pull in `libvlc` and VLC's plugins.

### Windows

```powershell
winget install LiamPalmqvist.LVP
```

LVP is installed as a portable app, so start it by running `lvp` in a terminal; it doesn't add a
Start menu entry yet.

### Any other system, or by hand

Every [release](https://github.com/LiamPalmqvist/LVP/releases) has a build for each system:

- **macOS**: `lvp-<version>-macos-arm64.zip`. Unzip it and move `LVP.app` to Applications.
  The app isn't notarized, so clear the download's quarantine flag before the first launch:
  `xattr -dr com.apple.quarantine /Applications/LVP.app`.
- **Linux** (x86_64): `lvp-<version>-linux-x86_64.tar.gz`. Unpack it and run `./install.sh`,
  which installs to `~/.local`; `sudo PREFIX=/usr/local ./install.sh` installs it for every user.
  Install VLC 3 from your distribution too (for example `sudo apt install vlc`).
- **Windows** (x64): `lvp-<version>-windows-x64.zip`. Unzip it and run `LVP.exe`.

On Linux, LVP looks for libvlc in the usual system locations. If yours is elsewhere, set
`LVP_LIBVLC` to the folder that holds libvlc; on macOS and Windows it replaces the bundled engine.

[`packaging/README.md`](packaging/README.md) describes how the packages are built and kept up to
date with each release.

## Build from source

You need Rust 1.90 or newer and VLC 3 (`/Applications/VLC.app` on macOS; see above for
`LVP_LIBVLC`). The smoke test also uses `ffmpeg` on `PATH` if it is there.

```bash
cargo run --release              # open the player
cargo run --release -- movie.mkv # open files directly
```

`tools/bundle-macos.sh`, `tools/build-linux.sh` and `tools/build-windows.ps1` build the same
packages a release ships. The macOS and Windows scripts download VLC's engine from VideoLAN, at the
version pinned in `tools/vlc-release.txt`.

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

GPL-2.0-or-later, the same as VLC: version 2 of the GNU General Public License, in
[LICENSE](LICENSE), or any later version. Inter and DM Mono are under the SIL Open Font
License (see `assets/fonts`). The macOS and Windows packages include VLC's engine (libvlc under
the LGPL-2.1-or-later, its plugins under the GPL or LGPL); [`licenses/VLC.txt`](licenses/VLC.txt)
is the notice they carry, with the link to VLC's source. VLC and VideoLAN are trademarks of
VideoLAN; LVP is not affiliated with VideoLAN.
