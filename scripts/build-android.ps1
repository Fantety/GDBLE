$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$metadata = cargo metadata --format-version 1 --locked | ConvertFrom-Json
$btleplug = $metadata.packages | Where-Object {
    $_.name -eq "btleplug" -and $_.version -eq "0.12.0"
} | Select-Object -First 1

if (-not $btleplug) {
    throw "btleplug 0.12.0 was not found in locked Cargo metadata"
}

$btleplugRoot = Split-Path -Parent $btleplug.manifest_path
$bridgeProject = Join-Path $btleplugRoot "src\droidplug\java"
$gradlew = Join-Path $bridgeProject "gradlew.bat"
$generatedJni = Join-Path $repoRoot "android-plugin\plugin\build\generated\jniLibs"

Push-Location $repoRoot
try {
    cargo ndk -t arm64-v8a -t x86_64 -o $generatedJni build --release --locked
    & $gradlew -p $bridgeProject assembleRelease
    & $gradlew -p (Join-Path $repoRoot "android-plugin") :plugin:assembleRelease

    $addonAndroid = Join-Path $repoRoot "addons\gdble\android"
    New-Item -ItemType Directory -Force -Path $addonAndroid | Out-Null
    Copy-Item (
        Join-Path $repoRoot "android-plugin\plugin\build\outputs\aar\plugin-release.aar"
    ) (Join-Path $addonAndroid "gdble-release.aar") -Force
    $bridgeAar = Get-ChildItem (
        Join-Path $bridgeProject "build\outputs\aar\*-release.aar"
    ) | Select-Object -First 1
    if (-not $bridgeAar) {
        throw "btleplug release AAR was not produced"
    }
    Copy-Item $bridgeAar.FullName (Join-Path $addonAndroid "btleplug-release.aar") -Force
} finally {
    Pop-Location
}
