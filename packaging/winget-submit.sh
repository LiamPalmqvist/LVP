#!/bin/sh
# Opens the pull request that adds packaging/winget/ to microsoft/winget-pkgs:
#
#   GH_TOKEN=<classic token, public_repo scope> packaging/winget-submit.sh 0.2.0
#
# The manifests go up exactly as they are in packaging/winget/, so run bump.sh
# for the same version first. The token's account forks winget-pkgs (once) and
# the pull request comes from a branch on that fork, built through the GitHub
# API so the very large repository is never cloned. A version that is already
# in winget-pkgs, or already has an open pull request, is left alone.
#
# Needs gh and jq. Set VALIDATED=1 when `winget validate` and
# `winget install --manifest` passed on these files, to tick those boxes.
set -eu
cd "$(dirname "$0")"

version=${1:-}
version=${version#v}
if [ -z "$version" ]; then
    echo "usage: $0 <version>" >&2
    exit 2
fi
id=LiamPalmqvist.NotVLC
upstream=microsoft/winget-pkgs
pkgdir=manifests/l/LiamPalmqvist/NotVLC
dir="$pkgdir/$version"

for manifest in winget/*.yaml; do
    if [ "$(sed -n 's/^PackageVersion: //p' "$manifest")" != "$version" ]; then
        echo "$manifest is not at $version; run packaging/bump.sh $version first" >&2
        exit 1
    fi
done

if gh api "repos/$upstream/contents/$dir" >/dev/null 2>&1; then
    echo "$id $version is already in $upstream"
    exit 0
fi
owner=$(gh api user --jq .login)
open=$(gh pr list -R "$upstream" --author "$owner" --state open --search "$id $version in:title" \
    --json url --jq '.[0].url // empty')
if [ -n "$open" ]; then
    echo "$id $version already has a pull request: $open"
    exit 0
fi
if gh api "repos/$upstream/contents/$pkgdir" >/dev/null 2>&1; then
    title="Update: $id to $version"
else
    title="New package: $id version $version"
fi

# A fork that is new takes a moment before its API answers.
fork="$owner/winget-pkgs"
gh repo fork "$upstream" --clone=false >/dev/null 2>&1 || true
tries=0
until gh api "repos/$fork" >/dev/null 2>&1; do
    tries=$((tries + 1))
    if [ "$tries" -gt 30 ]; then
        echo "the fork $fork did not appear" >&2
        exit 1
    fi
    sleep 5
done
gh api -X POST "repos/$fork/merge-upstream" -f branch=master >/dev/null

# One commit with the three manifests, on a branch named after the version.
base=$(gh api "repos/$fork/git/ref/heads/master" --jq .object.sha)
base_tree=$(gh api "repos/$fork/git/commits/$base" --jq .tree.sha)
tree=$(
    for manifest in winget/*.yaml; do
        jq -n --arg path "$dir/$(basename "$manifest")" --rawfile content "$manifest" \
            '{path: $path, mode: "100644", type: "blob", content: $content}'
    done | jq -s --arg base "$base_tree" '{base_tree: $base, tree: .}' |
        gh api -X POST "repos/$fork/git/trees" --input - --jq .sha
)
commit=$(jq -n --arg msg "$title" --arg tree "$tree" --arg parent "$base" \
    '{message: $msg, tree: $tree, parents: [$parent]}' |
    gh api -X POST "repos/$fork/git/commits" --input - --jq .sha)
branch="$id-$version"
if gh api "repos/$fork/git/ref/heads/$branch" >/dev/null 2>&1; then
    gh api -X PATCH "repos/$fork/git/refs/heads/$branch" -f sha="$commit" -F force=true >/dev/null
else
    gh api -X POST "repos/$fork/git/refs" -f ref="refs/heads/$branch" -f sha="$commit" >/dev/null
fi

tested=' '
[ "${VALIDATED:-}" = 1 ] && tested=x
body=$(cat <<EOF
## 📖 Description

$title: NotVLC, a media player built on VLC's engine (https://github.com/LiamPalmqvist/NotVLC). The installer is the release's Windows zip, which holds a portable NotVLC.exe; it depends on VideoLAN.VLC, whose libvlc it plays through.

## ✅ Checklist

- [ ] Signed the [Contributor License Agreement](https://cla.opensource.microsoft.com)
- [ ] Linked to an issue (if applicable)

## 📦 Manifest Checklist

- [x] Checked that there aren't other open [pull requests](https://github.com/microsoft/winget-pkgs/pulls) for the same manifest update/change
- [x] This PR only modifies one (1) manifest
- [$tested] Validated manifest locally with \`winget validate --manifest <path>\` ([validation guide](https://github.com/microsoft/winget-pkgs/blob/master/doc/ValidationFailureGuide.md))
- [$tested] Tested manifest locally with \`winget install --manifest <path>\`
- [x] Manifest conforms to the [1.12 schema](https://github.com/microsoft/winget-pkgs/tree/master/doc/manifest/schema/1.12.0)
EOF
)
gh pr create -R "$upstream" --base master --head "$owner:$branch" --title "$title" --body "$body"
