# Builds LVP for Windows and packs it as dist\lvp-<version>-windows-<arch>.zip:
# LVP.exe, VLC's engine (libvlc, its plugins and Lua scripts, from the VLC
# release pinned in tools\vlc-release.txt), its icon as a PNG, licences and a
# README. The package runs without VLC installed.
#
# usage (from a PowerShell prompt in the repository):
#   powershell -ExecutionPolicy Bypass -File tools\build-windows.ps1 [-DebugBuild] [-NoVlc]
#
#   -NoVlc  leave VLC's engine out; LVP then needs 64-bit VLC 3 installed
#           (found in Program Files, or set LVP_LIBVLC to its folder)
#
# Needs Rust 1.90+ with the MSVC toolchain (https://rustup.rs, plus the
# "Desktop development with C++" workload from the Visual Studio Build Tools).

param([switch]$DebugBuild, [switch]$NoVlc)
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "Rust is required: install it from https://rustup.rs"
    exit 1
}
$rust = (rustc --version)
if ($rust -match '^rustc 1\.(\d+)' -and [int]$Matches[1] -lt 90) {
    Write-Error "Rust 1.90 or newer is required (found $rust); run: rustup update"
    exit 1
}

$buildProfile = if ($DebugBuild) { "debug" } else { "release" }
if ($DebugBuild) { cargo build --locked } else { cargo build --release --locked }
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
$name = "lvp-$version-windows-$arch"
$stage = Join-Path "dist" $name
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force $stage | Out-Null

$exe = Join-Path $stage "LVP.exe"
Copy-Item "target\$buildProfile\lvp.exe" $exe
# The release build is a GUI program, so wait for it explicitly.
Start-Process -FilePath $exe -ArgumentList @("--export-icon", (Join-Path $stage "LVP.png")) -Wait -NoNewWindow

# VLC's engine beside LVP.exe, where infra\vlc\ffi.rs looks first. Left out:
# VLC's own interfaces (plugins\gui), its executables, browser plugins and
# translations, since LVP's interface is in English.
$licenses = Join-Path $stage "licenses"
New-Item -ItemType Directory -Force $licenses | Out-Null
if (-not $NoVlc) {
    $release = @{}
    Get-Content tools\vlc-release.txt | Where-Object { $_ -match '^\w' } | ForEach-Object {
        $key, $value = $_ -split '\s+', 2
        $release[$key] = $value
    }
    $vlcVersion = $release["version"]
    $vlcArch = if ($arch -eq "arm64") { "winarm64" } else { "win64" }
    $cache = "target\vlc"
    New-Item -ItemType Directory -Force $cache | Out-Null
    $vlcZip = Join-Path $cache "vlc-$vlcVersion-$vlcArch.zip"
    if (-not (Test-Path $vlcZip) -or (Get-FileHash -Algorithm SHA256 $vlcZip).Hash -ne $release[$vlcArch]) {
        $ProgressPreference = "SilentlyContinue"
        Invoke-WebRequest -Uri "https://download.videolan.org/pub/videolan/vlc/$vlcVersion/$vlcArch/vlc-$vlcVersion-$vlcArch.zip" -OutFile $vlcZip
        if ((Get-FileHash -Algorithm SHA256 $vlcZip).Hash -ne $release[$vlcArch]) {
            Write-Error "checksum mismatch: $vlcZip"
            exit 1
        }
    }
    $unpacked = Join-Path $cache "vlc-$vlcVersion-$vlcArch"
    if (Test-Path $unpacked) { Remove-Item -Recurse -Force $unpacked }
    Expand-Archive $vlcZip -DestinationPath $unpacked
    $vlcDir = Join-Path $unpacked "vlc-$vlcVersion"
    Copy-Item (Join-Path $vlcDir "libvlc.dll"), (Join-Path $vlcDir "libvlccore.dll") $stage
    foreach ($dir in "plugins", "lua", "hrtfs") { Copy-Item -Recurse (Join-Path $vlcDir $dir) $stage }
    Remove-Item -Recurse -Force (Join-Path $stage "plugins\gui")
    Copy-Item licenses\LGPL-2.1.txt $licenses
    (Get-Content licenses\VLC.txt -Raw).Replace("{version}", $vlcVersion) |
        Set-Content -Encoding UTF8 -NoNewline (Join-Path $licenses "VLC.txt")
}
# Licence texts travel with the binaries, as the GPL and the fonts' OFL require.
Copy-Item LICENSE (Join-Path $licenses "LICENSE.txt")
Copy-Item assets\fonts\Inter-OFL.txt, assets\fonts\DMMono-OFL.txt $licenses

$engine = if ($NoVlc) {
    @"
It needs 64-bit VLC 3 installed (https://www.videolan.org/vlc/), because it plays
media through VLC's engine. If VLC is installed somewhere other than Program Files,
set the LVP_LIBVLC environment variable to the folder that contains libvlc.dll.
"@
} else {
    @"
VLC's engine is included, so VLC itself does not need to be installed. See
licenses\VLC.txt for its licences and source code.
"@
}
@"
LVP $version

Run LVP.exe.

$engine

Settings and the library are stored in %APPDATA%\LVP\LVP.
"@ | Set-Content -Encoding UTF8 (Join-Path $stage "README.txt")

$zip = "dist\$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path $stage -DestinationPath $zip
Write-Output $zip
