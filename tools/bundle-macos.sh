#!/bin/sh
# Builds dist/NotVLC.app: release binary, icon from the design's cone, and an
# Info.plist that registers NotVLC for video, audio, playlist and subtitle
# files so Finder offers "Open With NotVLC".
#
# usage: tools/bundle-macos.sh [--debug]
set -eu
cd "$(dirname "$0")/.."

profile=release
if [ "${1:-}" = "--debug" ]; then profile=debug; fi
if [ "$profile" = release ]; then cargo build --release; else cargo build; fi

app=dist/NotVLC.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "target/$profile/notvlc" "$app/Contents/MacOS/notvlc"

# Icon: the cone on a dark rounded square, at every iconset size.
work=$(mktemp -d)
iconset="$work/NotVLC.iconset"
mkdir -p "$iconset"
sed 's/rgb(211,121,24)/rgb(255,136,0)/g' assets/icons/vlc.svg > "$work/cone.svg"
for size in 16 32 64 128 256 512 1024; do
    rsvg-convert -w $((size * 70 / 100)) -h $((size * 70 / 100)) "$work/cone.svg" -o "$work/cone-$size.png"
    magick -size ${size}x${size} xc:none -fill '#161616' \
        -draw "roundrectangle 0,0 $((size - 1)),$((size - 1)) $((size * 22 / 100)),$((size * 22 / 100))" \
        "$work/cone-$size.png" -gravity center -composite "$work/icon-$size.png"
done
for size in 16 32 128 256 512; do
    cp "$work/icon-$size.png" "$iconset/icon_${size}x${size}.png"
    cp "$work/icon-$((size * 2)).png" "$iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/NotVLC.icns"
rm -rf "$work"

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
doc_type() { # name, role, rank, extensions...
    name=$1; role=$2; shift 2
    printf '\t\t<dict>\n\t\t\t<key>CFBundleTypeName</key><string>%s</string>\n' "$name"
    printf '\t\t\t<key>CFBundleTypeRole</key><string>%s</string>\n' "$role"
    printf '\t\t\t<key>LSHandlerRank</key><string>Alternate</string>\n'
    printf '\t\t\t<key>CFBundleTypeIconFile</key><string>NotVLC</string>\n'
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
	<key>CFBundleName</key><string>NotVLC</string>
	<key>CFBundleDisplayName</key><string>NotVLC</string>
	<key>CFBundleIdentifier</key><string>org.notvlc.NotVLC</string>
	<key>CFBundleExecutable</key><string>notvlc</string>
	<key>CFBundleIconFile</key><string>NotVLC</string>
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

codesign --force --deep --sign - "$app" 2>/dev/null || true
echo "$app"
