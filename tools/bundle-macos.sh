#!/bin/sh
# Builds dist/LVP.app: release binary, icon from the logo, and an
# Info.plist that registers LVP for video, audio, playlist and subtitle
# files so Finder offers "Open With LVP". VLC's engine (libvlc, its plugins
# and Lua scripts) is copied in from the VLC release pinned in
# tools/vlc-release.txt, so the app runs without VLC installed.
#
# usage: tools/bundle-macos.sh [--debug] [--vlc-app PATH | --no-vlc]
#   --debug          bundle a debug build
#   --vlc-app PATH   take VLC's engine from this VLC.app instead of downloading
#                    the pinned release (it must be VLC 3)
#   --no-vlc         leave VLC's engine out; the app then needs VLC 3 installed
set -eu
cd "$(dirname "$0")/.."

profile=release
bundle_vlc=1
vlc_app=
while [ $# -gt 0 ]; do
    case "$1" in
        --debug) profile=debug ;;
        --vlc-app) vlc_app=${2:?--vlc-app needs the path of a VLC.app}; shift ;;
        --no-vlc) bundle_vlc=0 ;;
        -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
    shift
done
if [ "$profile" = release ]; then cargo build --release; else cargo build; fi

app=dist/LVP.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "target/$profile/lvp" "$app/Contents/MacOS/lvp"

# VLC's engine, laid out as in VLC.app's Contents/MacOS (lib/, plugins/,
# share/), which is where infra/vlc/ffi.rs looks first. Left out: VLC's own
# interface and Growl plugins, which need VLC.app's frameworks, and VLC's
# translations, since LVP's interface is in English.
if [ "$bundle_vlc" = 1 ]; then
    if [ -z "$vlc_app" ]; then
        release=$(awk '$1 == "version" { print $2 }' tools/vlc-release.txt)
        case "$(uname -m)" in arm64) arch=arm64 ;; *) arch=intel64 ;; esac
        sum=$(awk -v k="$arch" '$1 == k { print $2 }' tools/vlc-release.txt)
        dmg="target/vlc/vlc-$release-$arch.dmg"
        if ! echo "$sum  $dmg" | shasum -a 256 -c - >/dev/null 2>&1; then
            mkdir -p target/vlc
            curl -fL --retry 3 -o "$dmg" \
                "https://download.videolan.org/pub/videolan/vlc/$release/macosx/vlc-$release-$arch.dmg"
            echo "$sum  $dmg" | shasum -a 256 -c - >/dev/null || { echo "checksum mismatch: $dmg" >&2; exit 1; }
        fi
        mnt=$(mktemp -d)
        hdiutil attach -quiet -nobrowse -readonly -mountpoint "$mnt" "$dmg"
        trap 'hdiutil detach -quiet "$mnt"; rmdir "$mnt" 2>/dev/null || true' EXIT
        vlc_app="$mnt/VLC.app"
    fi
    vlc_version=$(/usr/libexec/PlistBuddy -c "Print CFBundleShortVersionString" "$vlc_app/Contents/Info.plist")
    case "$vlc_version" in
        3.*) ;;
        *) echo "$vlc_app is VLC $vlc_version; LVP needs VLC 3" >&2; exit 1 ;;
    esac
    src="$vlc_app/Contents/MacOS"
    dst="$app/Contents/MacOS"
    ditto "$src/lib" "$dst/lib"
    ditto "$src/plugins" "$dst/plugins"
    rm -f "$dst/plugins/libmacosx_plugin.dylib" "$dst/plugins/libosx_notifications_plugin.dylib"
    ditto "$src/share/lua" "$dst/share/lua"
    ditto "$src/share/hrtfs" "$dst/share/hrtfs"
fi

# Licence texts travel with the binaries, as the GPL and the fonts' OFL require.
licenses="$app/Contents/Resources/licenses"
mkdir -p "$licenses"
cp LICENSE "$licenses/LICENSE.txt"
cp assets/fonts/Inter-OFL.txt assets/fonts/DMMono-OFL.txt "$licenses/"
if [ "$bundle_vlc" = 1 ]; then
    cp licenses/LGPL-2.1.txt "$licenses/"
    sed "s/{version}/$vlc_version/g" licenses/VLC.txt > "$licenses/VLC.txt"
fi

# Icon: the logo at every iconset size.
work=$(mktemp -d)
iconset="$work/LVP.iconset"
mkdir -p "$iconset"
sed 's/rgb(211,121,24)/rgb(255,136,0)/g' assets/icons/logo.svg > "$work/logo.svg"
for size in 16 32 64 128 256 512 1024; do
    rsvg-convert -w $size -h $size "$work/logo.svg" -o "$work/icon-$size.png"
done
for size in 16 32 128 256 512; do
    cp "$work/icon-$size.png" "$iconset/icon_${size}x${size}.png"
    cp "$work/icon-$((size * 2)).png" "$iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/LVP.icns"
rm -rf "$work"

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
doc_type() { # name, role, rank, extensions...
    name=$1; role=$2; shift 2
    printf '\t\t<dict>\n\t\t\t<key>CFBundleTypeName</key><string>%s</string>\n' "$name"
    printf '\t\t\t<key>CFBundleTypeRole</key><string>%s</string>\n' "$role"
    printf '\t\t\t<key>LSHandlerRank</key><string>Alternate</string>\n'
    printf '\t\t\t<key>CFBundleTypeIconFile</key><string>LVP</string>\n'
    printf '\t\t\t<key>CFBundleTypeExtensions</key>\n\t\t\t<array>\n'
    for e in "$@"; do printf '\t\t\t\t<string>%s</string>\n' "$e"; done
    printf '\t\t\t</array>\n\t\t</dict>\n'
}
video_ext=$(sed -n '/pub const VIDEO_EXTENSIONS/,/];/p' src/domain/media.rs | rg -o '"[a-z0-9-]+"' | tr -d '"')
audio_ext=$(sed -n '/pub const AUDIO_EXTENSIONS/,/];/p' src/domain/media.rs | rg -o '"[a-z0-9-]+"' | tr -d '"')
list_ext="m3u m3u8 pls xspf asx b4s cue"
sub_ext="srt ass ssa sub vtt smi idx"

{
cat <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key><string>LVP</string>
	<key>CFBundleDisplayName</key><string>LVP</string>
	<key>CFBundleIdentifier</key><string>org.lvp.LVP</string>
	<key>CFBundleExecutable</key><string>lvp</string>
	<key>CFBundleIconFile</key><string>LVP</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleShortVersionString</key><string>$version</string>
	<key>CFBundleVersion</key><string>$version</string>
	<key>LSMinimumSystemVersion</key><string>11.0</string>
	<key>LSApplicationCategoryType</key><string>public.app-category.entertainment</string>
	<key>NSHighResolutionCapable</key><true/>
	<key>NSSupportsAutomaticGraphicsSwitching</key><true/>
	<key>CFBundleDocumentTypes</key>
	<array>
PLIST
doc_type "Video" Viewer $video_ext
doc_type "Audio" Viewer $audio_ext
doc_type "Playlist" Viewer $list_ext
doc_type "Subtitles" Viewer $sub_ext
cat <<PLIST
	</array>
</dict>
</plist>
PLIST
} > "$app/Contents/Info.plist"
plutil -lint "$app/Contents/Info.plist" >/dev/null

# Finder info and resource forks make a strict signature check fail, so clear
# them first. (In a folder synced by iCloud Drive the sync adds Finder info back
# afterwards; build outside one to get a bundle that passes --strict.)
xattr -cr "$app"
codesign --force --deep --sign - "$app"
echo "$app"
