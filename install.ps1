# install.ps1 — PowerShell installer for Phenotype Fabric
# Usage: irm https://raw.githubusercontent.com/<REDACTED>/PhenoFabric/main/install.ps1 | iex
#   or:  .\install.ps1
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$Repo       = '<REDACTED>/PhenoFabric'
$Bins       = @('fabric-daemon', 'fabric-cli', 'fabric-tui', 'fabric-gui', 'fabric-tray', 'fabric-graph')
$InstallDir = Join-Path $env:LOCALAPPDATA 'PhenotypeFabric'

# ── Helpers ───────────────────────────────────────────────────────────
function Write-Info  { param([string]$Msg) Write-Host "[info]  $Msg" -ForegroundColor Green }
function Write-Warn  { param([string]$Msg) Write-Host "[warn]  $Msg" -ForegroundColor Yellow }
function Write-Die   { param([string]$Msg) Write-Host "[error] $Msg" -ForegroundColor Red; exit 1 }

# ── Detect architecture ──────────────────────────────────────────────
$Arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'aarch64' } else { 'x86_64' }
$Platform = "windows-$Arch"

# ── Fetch latest release tag ─────────────────────────────────────────
function Get-LatestVersion {
    if ($env:FABRIC_VERSION) {
        return $env:FABRIC_VERSION
    }
    try {
        $headers = @{}
        if ($env:GITHUB_TOKEN) {
            $headers['Authorization'] = "token $env:GITHUB_TOKEN"
        }
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers $headers
        return $release.tag_name
    } catch {
        Write-Die "Failed to fetch latest release: $_"
    }
}

# ── Main ──────────────────────────────────────────────────────────────
function Install-Fabric {
    Write-Info 'Phenotype Fabric installer'
    Write-Info "Detected platform: $Platform"

    $Version = Get-LatestVersion
    Write-Info "Latest version: $Version"

    $Archive = "phenotype-fabric-$Version-$Platform.zip"
    $Url     = "https://github.com/$Repo/releases/download/$Version/$Archive"
    $TmpDir  = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null

    try {
        Write-Info "Downloading $Archive ..."
        $ProgressPreference = 'SilentlyContinue'
        Invoke-WebRequest -Uri $Url -OutFile (Join-Path $TmpDir $Archive) -UseBasicParsing
        $ProgressPreference = 'Continue'

        Write-Info 'Extracting ...'
        Expand-Archive -Path (Join-Path $TmpDir $Archive) -DestinationPath $TmpDir -Force

        $DistDir = Join-Path $TmpDir "phenotype-fabric-$Version-$Platform"
        if (-not (Test-Path $DistDir)) {
            Write-Die "Expected directory not found after extraction: $DistDir"
        }

        # Ensure install directory exists
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

        Write-Info "Installing to $InstallDir ..."
        foreach ($Bin in $Bins) {
            $Src = Join-Path $DistDir "$Bin.exe"
            $Dst = Join-Path $InstallDir "$Bin.exe"
            if (Test-Path $Src) {
                Copy-Item $Src $Dst -Force
                Write-Info "  Installed $Bin"
            } else {
                Write-Warn "  Skipped $Bin (not in archive)"
            }
        }

        # Add to PATH if not already there
        $CurrentPath = [Environment]::GetEnvironmentVariable('PATH', 'User')
        if ($CurrentPath -notlike "*$InstallDir*") {
            [Environment]::SetEnvironmentVariable('PATH', "$CurrentPath;$InstallDir", 'User')
            $env:PATH = "$env:PATH;$InstallDir"
            Write-Info "Added $InstallDir to user PATH."
        }

        Write-Info 'Done.'
        Write-Host ''
        Write-Info 'Installed binaries:'
        foreach ($Bin in $Bins) {
            $BinPath = Join-Path $InstallDir "$Bin.exe"
            if (Test-Path $BinPath) {
                Write-Host ("  {0,-25} {1}" -f $Bin, $BinPath)
            }
        }
        Write-Host ''
        Write-Info 'Get started: fabric-cli --help'
    } finally {
        Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
    }
}

Install-Fabric
