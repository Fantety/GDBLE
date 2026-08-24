$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path -LiteralPath (Split-Path -Parent $PSScriptRoot)).Path
$source = Join-Path $repoRoot "addons\gdble"
$destination = Join-Path $repoRoot "demo\addons\gdble"
$demoAddons = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "demo\addons"))
$resolvedDestination = [System.IO.Path]::GetFullPath($destination)

if (-not $resolvedDestination.StartsWith(
    $demoAddons + [System.IO.Path]::DirectorySeparatorChar,
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Refusing to replace a directory outside demo/addons: $resolvedDestination"
}

if (Test-Path -LiteralPath $resolvedDestination) {
    Remove-Item -LiteralPath $resolvedDestination -Recurse -Force
}
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $resolvedDestination) | Out-Null
Copy-Item -LiteralPath $source -Destination $resolvedDestination -Recurse -Force
