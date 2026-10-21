#!/bin/sh
# Points every package manifest in packaging/ at one GitHub release:
#
#   packaging/bump.sh 0.2.0
#
# Takes the asset checksums GitHub records for the release, hashes the source
# tarball the AUR's lvp builds from, and rewrites the version and checksums
# in the Homebrew cask, both AUR packages (.SRCINFO included) and the winget
# manifests. A new version resets the AUR pkgrel to 1. Running it twice for one
# version changes nothing.
#
# Needs curl, jq, perl and sha256sum or shasum. Set GITHUB_TOKEN to avoid the
# API's anonymous rate limit.
set -eu
cd "$(dirname "$0")"

version=${1:-}
version=${version#v}
if [ -z "$version" ]; then
    echo "usage: $0 <version>" >&2
    exit 2
fi
repo=LiamPalmqvist/LVP
tag="v$version"

get() {
    if [ -n "${GITHUB_TOKEN:-}" ]; then
        curl -fsSL -H "Authorization: Bearer $GITHUB_TOKEN" "$@"
    else
        curl -fsSL "$@"
    fi
}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum | cut -d' ' -f1
    else
        shasum -a 256 | cut -d' ' -f1
    fi
}

release=$(get "https://api.github.com/repos/$repo/releases/tags/$tag") || {
    echo "no release $tag on $repo" >&2
    exit 1
}

# GitHub keeps a sha256 digest for every asset; an asset without one is hashed here.
asset_sha() {
    url=$(printf '%s' "$release" | jq -r --arg n "$1" '.assets[] | select(.name == $n) | .browser_download_url')
    if [ -z "$url" ]; then
        echo "$tag has no asset $1" >&2
        exit 1
    fi
    digest=$(printf '%s' "$release" | jq -r --arg n "$1" '.assets[] | select(.name == $n) | .digest // empty')
    case "$digest" in
        sha256:*) echo "${digest#sha256:}" ;;
        *) get "$url" | sha256 ;;
    esac
}

V=$version
MAC=$(asset_sha "lvp-$version-macos-arm64.zip")
LINUX=$(asset_sha "lvp-$version-linux-x86_64.tar.gz")
WIN=$(asset_sha "lvp-$version-windows-x64.zip")
SRC=$(get "https://github.com/$repo/archive/refs/tags/$tag.tar.gz" | sha256)
DATE=$(printf '%s' "$release" | jq -r '.published_at[0:10]')
export V MAC LINUX WIN SRC DATE

# Each file is moved from the version it names now, so the files need not agree.
bump() {
    file=$1
    OLD=$2
    export OLD
    shift 2
    perl -pi -e "$@" "$file"
}

# Version-bearing lines only; a checksum or another number is never touched.
replace_version='s/\Q$ENV{OLD}\E/$ENV{V}/g if /^\s*(-\s*)?(version|pkgver|source(?:_\w+)?|PackageVersion|InstallerUrl|RelativeFilePath|ReleaseNotesUrl)\b/;'

cask=homebrew/lvp.rb
bump "$cask" "$(sed -n 's/^  version "\(.*\)"/\1/p' "$cask")" \
    "$replace_version"' s/^  sha256 ".*"/  sha256 "$ENV{MAC}"/;'

for pkg in lvp lvp-bin; do
    case "$pkg" in
        lvp) sum=SRC ;;
        lvp-bin) sum=LINUX ;;
    esac
    old=$(sed -n 's/^pkgver=//p' "aur/$pkg/PKGBUILD")
    rel='s/^pkgrel=.*/pkgrel=1/; s/^\tpkgrel = .*/\tpkgrel = 1/;'
    [ "$old" = "$version" ] && rel=''
    bump "aur/$pkg/PKGBUILD" "$old" \
        "$replace_version $rel s/^(sha256sums(?:_\\w+)?)=\('.*'\)/\$1=('\$ENV{$sum}')/;"
    bump "aur/$pkg/.SRCINFO" "$old" \
        "$replace_version $rel s/^(\tsha256sums(?:_\\w+)?) = .*/\$1 = \$ENV{$sum}/;"
done

for manifest in winget/*.yaml; do
    bump "$manifest" "$(sed -n 's/^PackageVersion: //p' "$manifest")" \
        "$replace_version"' s/^ReleaseDate: .*/ReleaseDate: $ENV{DATE}/; s/^(\s*InstallerSha256: ).*/$1\U$ENV{WIN}/;'
done

echo "packaging/ points at $tag"
