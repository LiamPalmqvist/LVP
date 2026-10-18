# Packaging

NotVLC ships through three package managers, each built from the assets of a GitHub release:

| Channel | Install | Built from | Lives in |
|---|---|---|---|
| Homebrew (macOS, Apple silicon) | `brew install --cask liampalmqvist/notvlc/notvlc` | `notvlc-<v>-macos-arm64.zip` | [LiamPalmqvist/homebrew-notvlc](https://github.com/LiamPalmqvist/homebrew-notvlc) |
| AUR, from source | `yay -S notvlc` | the tag's source tarball | [aur.archlinux.org/packages/notvlc](https://aur.archlinux.org/packages/notvlc) |
| AUR, release binary | `yay -S notvlc-bin` | `notvlc-<v>-linux-x86_64.tar.gz` | [aur.archlinux.org/packages/notvlc-bin](https://aur.archlinux.org/packages/notvlc-bin) |
| winget (Windows) | `winget install LiamPalmqvist.NotVLC` | `notvlc-<v>-windows-x64.zip` | [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs/tree/master/manifests/l/LiamPalmqvist/NotVLC) |

This folder holds the source of each: `homebrew/notvlc.rb`, `aur/<package>/PKGBUILD` with its
`.SRCINFO`, and the three `winget/` manifests. They always name the latest release.

Each needs VLC 3, which NotVLC plays through. The AUR packages depend on `libvlc` and
`vlc-plugins-all`, and the winget manifest on `VideoLAN.VLC`. The cask only says so in its caveats.
A Mac with VLC installed from videolan.org would otherwise fail the install, because Homebrew
would try to put a second copy in `/Applications`.

The Mac app is signed ad hoc, not notarized, so the cask clears the quarantine flag after
installing it. Without that, Gatekeeper refuses the first launch.

## A release

Pushing a `v*` tag runs `build.yml`, which publishes the release and then calls `packaging.yml`.
That workflow:

1. runs `bump.sh <tag>`, which writes the release's version and checksums into every file here;
2. tests each channel:
   - installs the cask from a local tap on macOS;
   - builds and installs both AUR packages in an Arch container;
   - validates and installs the winget manifest on Windows;
3. publishes:
   - pushes the cask to the tap;
   - pushes both PKGBUILDs to the AUR;
   - opens a pull request on microsoft/winget-pkgs;
4. commits the bumped files to `main`.

A pull request that changes this folder runs steps 1 and 2 against the files as they stand.

To publish a release again, for example after fixing a PKGBUILD (raise `pkgrel` first):

```bash
gh workflow run packaging.yml -f tag=v0.1.0
```

`packaging/bump.sh 0.2.0` makes the same changes by hand.

## One-time setup

Publishing to a channel needs a secret on this repository. A channel whose secret is missing is
skipped with a notice, and the rest still publish.

- **`HOMEBREW_TAP_KEY`**: the private half of a deploy key with write access on
  `LiamPalmqvist/homebrew-notvlc`.
- **`AUR_SSH_KEY`**: the private half of an SSH key whose public half is under *My Account > SSH
  Public Key* on aur.archlinux.org. The first push creates `notvlc` and `notvlc-bin` under that
  account.
- **`WINGET_TOKEN`**: a classic personal access token with the `public_repo` scope. It forks
  microsoft/winget-pkgs and opens the pull requests. A fine-grained token cannot open a pull
  request on another owner's repository.

Microsoft reviews every winget-pkgs pull request. On the first one, their policy bot asks you to
accept the Contributor License Agreement in a comment. `winget-submit.sh` submits by hand, with
`GH_TOKEN` set to that token.
