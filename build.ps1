$ErrorActionPreference = "Stop"

$repoRoot = $PSScriptRoot
$addon = Join-Path $repoRoot "addons\gdble"

Push-Location $repoRoot
try {
    cargo build --release --locked
    Copy-Item (
        Join-Path $repoRoot "target\release\gdble.dll"
    ) (Join-Path $addon "libgdble.windows.x86_64.dll") -Force

    if ($env:ANDROID_NDK_HOME -and (Get-Command cargo-ndk -ErrorAction SilentlyContinue)) {
        & (Join-Path $repoRoot "scripts\build-android.ps1")
    } else {
        Write-Host "Android build skipped (ANDROID_NDK_HOME or cargo-ndk unavailable)."
    }

    & (Join-Path $repoRoot "scripts\sync-demo-addon.ps1")
} finally {
    Pop-Location
}
