# Packaging

LVP ships through three package managers, each built from the assets of a GitHub release:

| Channel | Install | Built from | Lives in |
|---|---|---|---|
| Homebrew (macOS, Apple silicon) | `brew install --cask liampalmqvist/notvlc/lvp` | `lvp-<v>-macos-arm64.zip` | [LiamPalmqvist/homebrew-notvlc](https://github.com/LiamPalmqvist/homebrew-notvlc) |
| AUR, from source | `yay -S lvp` | the tag's source tarball | [aur.archlinux.org/packages/lvp](https://aur.archlinux.org/packages/lvp) |
| AUR, release binary | `yay -S lvp-bin` | `lvp-<v>-linux-x86_64.tar.gz` | [aur.archlinux.org/packages/lvp-bin](https://aur.archlinux.org/packages/lvp-bin) |
| winget (Windows) | `winget install LiamPalmqvist.LVP` | `lvp-<v>-windows-x64.zip` | [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs/tree/master/manifests/l/LiamPalmqvist/LVP) |

This folder holds the source of each: `homebrew/lvp.rb`, `aur/<package>/PKGBUILD` with its
`.SRCINFO`, and the three `winget/` manifests. They always name the latest release.

The Mac and Windows packages carry VLC's engine themselves (see `tools/vlc-release.txt`), so the
cask and the winget manifest need nothing else. The AUR packages depend on `libvlc` and
`vlc-plugins-all`, as Arch packages use the system's libraries.

The tap kept its first name, `homebrew-notvlc`, when the app became LVP, so the cask installs as
`liampalmqvist/notvlc/lvp`. Its `cask_renames.json` moves anyone who installed `notvlc` over to `lvp`.

The Mac app is signed ad hoc, not notarized, so the cask clears the quarantine flag after
installing it. Without that, Gatekeeper refuses the first launch.

## A release

Pushing a `v*` tag runs `build.yml`, which publishes the release and then calls `packaging.yml`.
That workflow:

1. runs `bump.sh <tag>`, which writes the release's version and checksums into every file here;
2. tests each channel:
   - installs the cask from a local tap on macOS;
   - builds and installs both AUR packages in an Arch container;
   - validates and installs the winget manifest on Windows. winget's own archive scan fails on
     GitHub's runners, where Defender's real-time protection is off, so the job scans the zip
     with Defender itself and fails only if Defender names a threat;
3. publishes:
   - pushes the cask to the tap;
   - pushes both PKGBUILDs to the AUR, while `AUR_PUBLISH` is on;
   - opens a pull request on microsoft/winget-pkgs, while `WINGET_PUBLISH` is on;
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
  Public Key* on aur.archlinux.org. The first push creates `lvp` and `lvp-bin` under that
  account. While the key is on no account (AUR registration is sometimes closed), the AUR refuses
  it: both packages are still built and tested, and the push is skipped with a warning.
  AUR publishing is also switched off until the repository variable `AUR_PUBLISH` is `true`:
  `gh variable set AUR_PUBLISH --body true -R LiamPalmqvist/LVP`.
- **`WINGET_TOKEN`**: a classic personal access token with the `public_repo` scope. It forks
  microsoft/winget-pkgs and opens the pull requests. A fine-grained token cannot open a pull
  request on another owner's repository. The pull request is only opened while the repository
  variable `WINGET_PUBLISH` is `true`: `gh variable set WINGET_PUBLISH --body true -R LiamPalmqvist/LVP`.

Microsoft reviews every winget-pkgs pull request. On the first one, their policy bot asks you to
accept the Contributor License Agreement in a comment. `winget-submit.sh` submits by hand, with
`GH_TOKEN` set to that token.
