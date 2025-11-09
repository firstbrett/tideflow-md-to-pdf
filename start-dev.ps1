$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSCommandPath
Set-Location $repoRoot

Write-Host "Tideflow dev bootstrap starting..." -ForegroundColor Cyan

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path $vswhere)) {
    throw "vswhere.exe not found at $vswhere. Install Visual Studio Build Tools."
}

$vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Workload.VCTools -property installationPath
if (-not $vsPath) {
    throw "Visual Studio Build Tools with the VC workload is required but was not found."
}

$devShell = Join-Path $vsPath 'Common7\Tools\Launch-VsDevShell.ps1'
if (-not (Test-Path $devShell)) {
    throw "Unable to locate Launch-VsDevShell.ps1 inside $vsPath."
}

Write-Host "Entering Visual Studio developer shell..." -ForegroundColor Yellow
& $devShell -Arch amd64 -HostArch amd64 | Out-Null

$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path $cargoBin) {
    if ($env:PATH -notlike "*$cargoBin*") {
        $env:PATH = "$cargoBin;$env:PATH"
    }
} else {
    Write-Warning "Rust cargo bin folder not found. Run rustup-init to install Rust."
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo is still unavailable even after adjusting PATH."
}

$tectonicLocal = Join-Path $repoRoot 'src-tauri\bin\tectonic\windows\tectonic.exe'
if (-not (Get-Command tectonic -ErrorAction SilentlyContinue) -and -not (Test-Path $tectonicLocal)) {
    Write-Warning "Tectonic CLI not found on PATH or at $tectonicLocal. Install it (e.g. 'winget install Tectonic.Tectonic') or drop the binary into src-tauri\bin\tectonic\windows."
}

Write-Host "Running npm install (skip if already up-to-date)..." -ForegroundColor Yellow
npm install

Write-Host "Starting Tideflow (npm run tauri:dev)..." -ForegroundColor Green
npm run tauri:dev
