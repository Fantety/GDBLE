$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false
$repoRoot = Split-Path -Parent $PSScriptRoot
$fixtureRoot = Join-Path $repoRoot ("target\build-script-tests-" + [guid]::NewGuid().ToString("N"))

# The mock has the same exit-code semantics as a native Cargo process.
function cargo {
    if ((Get-Location).Path -ne $env:GDBLE_TEST_ROOT) {
        throw "Cargo was invoked outside the fixture repository"
    }
    if ($args[0] -eq "metadata") {
        $global:LASTEXITCODE = if ($env:GDBLE_TEST_FAIL -eq "metadata") { 13 } else { 0 }
        @{ packages = @(@{
            name = "btleplug"; version = "0.12.0"
            manifest_path = (Join-Path $env:GDBLE_TEST_ROOT "btleplug\Cargo.toml")
        }) } | ConvertTo-Json -Depth 5 -Compress
    } else {
        $global:LASTEXITCODE = if ($env:GDBLE_TEST_FAIL -in @("desktop", "ndk")) { 13 } else { 0 }
    }
}

foreach ($mode in @("desktop", "metadata", "ndk", "bridge", "plugin")) {
    $fixture = Join-Path $fixtureRoot $mode
    $env:GDBLE_TEST_ROOT = [System.IO.Path]::GetFullPath($fixture)
    $env:GDBLE_TEST_FAIL = $mode
    foreach ($directory in @(
        "scripts", "target\release", "addons\gdble\android",
        "btleplug\src\droidplug\java\build\outputs\aar",
        "android-plugin\plugin\build\outputs\aar"
    )) {
        New-Item -ItemType Directory -Force -Path (Join-Path $fixture $directory) | Out-Null
    }
    Copy-Item -LiteralPath (Join-Path $repoRoot "build.ps1") -Destination $fixture
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot "build-android.ps1") -Destination (Join-Path $fixture "scripts")
    Set-Content -LiteralPath (Join-Path $fixture "scripts\sync-demo-addon.ps1") -Encoding utf8 -Value "# test fixture"
    $desktopArtifact = Join-Path $fixture "addons\gdble\libgdble.windows.x86_64.dll"
    $pluginArtifact = Join-Path $fixture "addons\gdble\android\gdble-release.aar"
    $bridgeArtifact = Join-Path $fixture "addons\gdble\android\btleplug-release.aar"
    foreach ($artifact in @($desktopArtifact, $pluginArtifact, $bridgeArtifact)) {
        Set-Content -LiteralPath $artifact -Encoding utf8 -Value "canonical"
    }
    foreach ($artifact in @(
        "target\release\gdble.dll",
        "android-plugin\plugin\build\outputs\aar\plugin-release.aar",
        "btleplug\src\droidplug\java\build\outputs\aar\btleplug-release.aar"
    )) {
        Set-Content -LiteralPath (Join-Path $fixture $artifact) -Encoding utf8 -Value "stale-build"
    }
    $bridge = Join-Path $fixture "btleplug\src\droidplug\java"
    $env:GDBLE_TEST_FAIL = $mode
    $env:GDBLE_TEST_BRIDGE = $bridge
    $env:GDBLE_TEST_PLUGIN = Join-Path $fixture "android-plugin"
    $batch = @'
@echo off
if "%GDBLE_TEST_FAIL%"=="bridge" if "%~2"=="%GDBLE_TEST_BRIDGE%" exit /b 13
if "%GDBLE_TEST_FAIL%"=="plugin" if "%~2"=="%GDBLE_TEST_PLUGIN%" exit /b 13
exit /b 0
'@
    Set-Content -LiteralPath (Join-Path $bridge "gradlew.bat") -Encoding ascii -Value $batch
    $originalLocation = (Get-Location).Path
    $caught = $false
    try {
        if ($mode -eq "desktop") {
            & (Join-Path $fixture "build.ps1")
        } else {
            & (Join-Path $fixture "scripts\build-android.ps1")
        }
    } catch {
        if ($_.Exception.Message -notmatch "failed with exit code 13") { throw }
        $caught = $true
    }
    if (-not $caught) { throw "$mode did not propagate the native failure" }
    if ((Get-Location).Path -ne $originalLocation) { throw "$mode did not restore the working directory" }
    foreach ($artifact in @($desktopArtifact, $pluginArtifact, $bridgeArtifact)) {
        if ((Get-Content -LiteralPath $artifact -Encoding UTF8 -Raw).Trim() -ne "canonical") {
            throw "$mode overwrote a canonical artifact with a stale build"
        }
    }
    Write-Output "PASS: $mode failure preserves all canonical artifacts"
}

# GitHub's PowerShell runner forwards LASTEXITCODE, including injected failures.
$global:LASTEXITCODE = 0
