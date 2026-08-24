[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidatePattern('^\d+\.\d+\.\d+([+-][0-9A-Za-z.-]+)?$')]
    [string]$Version,
    [string]$Repository = 'J0NAsM/AnyMio',
    [switch]$SetPackageVersion
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
$cargoToml = Join-Path $projectRoot 'Cargo.toml'
$releaseExe = Join-Path $projectRoot 'target\release\JRemote.exe'
$manifestPath = Join-Path $projectRoot 'update.json'

if ($SetPackageVersion) {
    $cargo = Get-Content -LiteralPath $cargoToml -Raw
    $updated = [regex]::Replace($cargo, '(?m)^version = ".+"$', "version = `"$Version`"", 1)
    if ($updated -eq $cargo) { throw 'Could not find the package version in Cargo.toml.' }
    [System.IO.File]::WriteAllText($cargoToml, $updated, [System.Text.UTF8Encoding]::new($false))
}

Push-Location $projectRoot
try {
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test --all-targets
    cargo build --release
} finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $releaseExe -PathType Leaf)) {
    throw 'JRemote.exe was not generated.'
}
if (-not (Test-Path -LiteralPath (Join-Path $projectRoot 'target\release\JRemoteUpdater.exe') -PathType Leaf)) {
    throw 'JRemoteUpdater.exe was not generated.'
}

$tag = "v$Version"
$sha256 = (Get-FileHash -LiteralPath $releaseExe -Algorithm SHA256).Hash.ToLowerInvariant()
$manifest = [ordered]@{
    version = $Version
    url = "https://github.com/$Repository/releases/download/$tag/JRemote.exe"
    sha256 = $sha256
    notes = "Publicar notas de la versión $Version."
} | ConvertTo-Json
[System.IO.File]::WriteAllText($manifestPath, "$manifest`n", [System.Text.UTF8Encoding]::new($false))

Write-Host "Prepared $tag"
Write-Host "SHA-256: $sha256"
Write-Host 'Next: review update.json, commit it, tag the commit and push the tag.'
