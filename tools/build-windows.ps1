# Builds NotVLC for Windows and packs it as dist\notvlc-<version>-windows-<arch>.zip:
# NotVLC.exe, its icon as a PNG, a README and the licence.
#
# usage (from a PowerShell prompt in the repository):
#   powershell -ExecutionPolicy Bypass -File tools\build-windows.ps1 [-DebugBuild]
#
# Needs Rust 1.90+ with the MSVC toolchain (https://rustup.rs, plus the
# "Desktop development with C++" workload from the Visual Studio Build Tools).
# NotVLC plays through VLC 3's libvlc at runtime: install 64-bit VLC 3 from
# https://www.videolan.org/vlc/ (it is found in Program Files), or set
# NOTVLC_LIBVLC to the folder that contains libvlc.dll.

param([switch]$DebugBuild)
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

$vlc = @("$env:ProgramFiles\VideoLAN\VLC\libvlc.dll", "${env:ProgramFiles(x86)}\VideoLAN\VLC\libvlc.dll")
if (-not $env:NOTVLC_LIBVLC -and -not ($vlc | Where-Object { Test-Path $_ })) {
    Write-Warning "VLC 3 was not found in Program Files. Install 64-bit VLC 3 before starting NotVLC."
}

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
$name = "notvlc-$version-windows-$arch"
$stage = Join-Path "dist" $name
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force $stage | Out-Null

$exe = Join-Path $stage "NotVLC.exe"
Copy-Item "target\$buildProfile\notvlc.exe" $exe
# The release build is a GUI program, so wait for it explicitly.
Start-Process -FilePath $exe -ArgumentList @("--export-icon", (Join-Path $stage "NotVLC.png")) -Wait -NoNewWindow

@"
NotVLC $version

Run NotVLC.exe. It needs 64-bit VLC 3 installed (https://www.videolan.org/vlc/),
because it plays media through VLC's engine. If VLC is installed somewhere other
than Program Files, set the NOTVLC_LIBVLC environment variable to the folder that
contains libvlc.dll.

Settings and the library are stored in %APPDATA%\NotVLC\NotVLC.
"@ | Set-Content -Encoding UTF8 (Join-Path $stage "README.txt")
Copy-Item LICENSE (Join-Path $stage "LICENSE.txt")

$zip = "dist\$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path $stage -DestinationPath $zip
Write-Output $zip
