#!/bin/sh
# Builds LVP for Linux and packs it as dist/lvp-<version>-linux-<arch>.tar.gz:
# the binary, a desktop entry, an icon, licences and install.sh.
#
# usage: tools/build-linux.sh [--install-deps] [--debug]
#   --install-deps  install build tools and runtime libraries (VLC 3, graphics,
#                   portals) with apt, dnf, pacman or zypper, using sudo
#   --debug         package a debug build instead of a release build
#
# LVP needs VLC 3's libvlc at runtime (libvlc.so.5 and its plugins). It is
# found in the usual system locations, or set LVP_LIBVLC to its folder.
# Unlike the macOS and Windows packages, this one does not bundle libvlc: Linux
# packages (the AUR among them) are expected to use the distribution's.
set -eu
cd "$(dirname "$0")/.."

install_deps=0
profile=release
for arg in "$@"; do
    case "$arg" in
        --install-deps) install_deps=1 ;;
        --debug) profile=debug ;;
        -h|--help) sed -n '2,14p' "$0"; exit 0 ;;
        *) echo "unknown option: $arg" >&2; exit 2 ;;
    esac
done

if [ "$install_deps" = 1 ]; then
    if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update
        sudo apt-get install -y build-essential pkg-config curl \
            vlc libvlc5 libvlccore9 vlc-plugin-base \
            libxkbcommon0 libwayland-client0 libwayland-cursor0 libwayland-egl1 \
            libx11-6 libxcursor1 libxrandr2 libxi6 \
            libvulkan1 mesa-vulkan-drivers libgl1 libegl1 \
            xdg-desktop-portal libnotify-bin
    elif command -v dnf >/dev/null 2>&1; then
        # VLC comes from RPM Fusion on Fedora.
        sudo dnf install -y gcc pkgconf-pkg-config curl \
            vlc vlc-plugins-base \
            libxkbcommon wayland-devel libX11 libXcursor libXrandr libXi \
            vulkan-loader mesa-vulkan-drivers mesa-libGL mesa-libEGL \
            xdg-desktop-portal libnotify
    elif command -v pacman >/dev/null 2>&1; then
        sudo pacman -S --needed --noconfirm base-devel pkgconf curl \
            vlc libxkbcommon wayland libx11 libxcursor libxrandr libxi \
            vulkan-icd-loader mesa xdg-desktop-portal libnotify
    elif command -v zypper >/dev/null 2>&1; then
        sudo zypper install -y gcc pkg-config curl \
            vlc libvlc5 libxkbcommon0 libwayland-client0 libX11-6 libXcursor1 libXrandr2 libXi6 \
            libvulkan1 Mesa-libGL1 xdg-desktop-portal libnotify-tools
    else
        echo "No supported package manager found; install VLC 3, a C linker and pkg-config by hand." >&2
    fi
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust is required: https://rustup.rs  (curl https://sh.rustup.rs -sSf | sh)" >&2
    exit 1
fi
rust_minor=$(rustc --version | sed -n 's/^rustc 1\.\([0-9]*\).*/\1/p')
if [ -n "$rust_minor" ] && [ "$rust_minor" -lt 90 ]; then
    echo "Rust 1.90 or newer is required (found $(rustc --version)); run: rustup update" >&2
    exit 1
fi

if [ "$profile" = release ]; then
    cargo build --release --locked
else
    cargo build --locked
fi

found_vlc=0
for dir in /usr/lib /usr/lib64 /usr/lib/x86_64-linux-gnu /usr/lib/aarch64-linux-gnu /usr/local/lib; do
    [ -e "$dir/libvlc.so.5" ] && found_vlc=1
done
if [ "$found_vlc" = 0 ] && [ -z "${LVP_LIBVLC:-}" ]; then
    echo "warning: libvlc.so.5 was not found. Install VLC 3 (or run with --install-deps) before starting LVP." >&2
fi

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
name="lvp-$version-linux-$(uname -m)"
stage="dist/$name"
rm -rf "$stage"
mkdir -p "$stage/bin" "$stage/share/applications" "$stage/share/icons/hicolor/256x256/apps" \
    "$stage/share/licenses/lvp"

cp "target/$profile/lvp" "$stage/bin/lvp"
[ "$profile" = release ] && strip "$stage/bin/lvp" 2>/dev/null || true
"$stage/bin/lvp" --export-icon "$stage/share/icons/hicolor/256x256/apps/lvp.png"
# Licence texts travel with the binary, as the GPL and the fonts' OFL require.
cp LICENSE assets/fonts/Inter-OFL.txt assets/fonts/DMMono-OFL.txt "$stage/share/licenses/lvp/"

# Wayland matches windows to desktop entries by app id, so the file is named after it.
cat > "$stage/share/applications/org.lvp.LVP.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=LVP
GenericName=Media Player
Comment=Play videos, music, discs and network streams with VLC's engine
Exec=lvp %F
Icon=lvp
Terminal=false
Categories=AudioVideo;Player;Video;Audio;
Keywords=video;music;player;movie;vlc;dvd;stream;subtitles;
StartupWMClass=org.lvp.LVP
MimeType=video/mp4;video/x-matroska;video/webm;video/x-msvideo;video/quicktime;video/mpeg;video/x-flv;video/x-ms-wmv;video/3gpp;video/mp2t;video/ogg;video/x-m4v;audio/mpeg;audio/mp4;audio/flac;audio/x-flac;audio/ogg;audio/opus;audio/x-wav;audio/wav;audio/aac;audio/x-ms-wma;audio/x-matroska;audio/webm;audio/x-aiff;application/x-mpegurl;audio/x-mpegurl;audio/x-scpls;application/xspf+xml;application/x-cd-image;
DESKTOP

cat > "$stage/install.sh" <<'INSTALL'
#!/bin/sh
# Installs LVP into PREFIX (default ~/.local). For every user:
#   sudo PREFIX=/usr/local ./install.sh
set -eu
here=$(cd "$(dirname "$0")" && pwd)
prefix=${PREFIX:-$HOME/.local}
mkdir -p "$prefix/bin" "$prefix/share/applications" "$prefix/share/icons/hicolor/256x256/apps"
cp "$here/bin/lvp" "$prefix/bin/lvp"
chmod 755 "$prefix/bin/lvp"
cp "$here/share/icons/hicolor/256x256/apps/lvp.png" "$prefix/share/icons/hicolor/256x256/apps/lvp.png"
sed "s|^Exec=lvp|Exec=$prefix/bin/lvp|" "$here/share/applications/org.lvp.LVP.desktop" \
    > "$prefix/share/applications/org.lvp.LVP.desktop"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database -q "$prefix/share/applications" || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t "$prefix/share/icons/hicolor" || true
echo "Installed LVP to $prefix (run: $prefix/bin/lvp)"
INSTALL
chmod +x "$stage/install.sh"

tar -C dist -czf "dist/$name.tar.gz" "$name"
echo "dist/$name.tar.gz"
