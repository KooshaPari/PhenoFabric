# install.ps1 — PowerShell installer for Phenotype Fabric
# Usage: irm https://raw.githubusercontent.com/KooshaPari/PhenoFabric/main/install.ps1 | iex
#   or:  .\install.ps1 -Version 0.1.8 -InstallDir C:\tools\bin
#
# Downloads the Windows release archive for this platform, verifies it against
# the release's own published SHA-256, and installs every required binary.
[CmdletBinding()]
param(
    [string]$Version,
    [string]$InstallDir,
    [string]$Arch
)

$ErrorActionPreference = 'Stop'

$Repo = 'KooshaPari/PhenoFabric'

# Binaries installed from the release archive. These are the real cargo [[bin]]
# target names and MUST match the archive contents produced by
# .github/workflows/release.yml:
#   - "fabric"           is the [[bin]] of the fabric-cli crate, NOT the crate
#                        name "fabric-cli"
#   - "fabric-graph-cli" is the crate/bin name; there is no "fabric-graph"
#                        binary (fabric-graph is a library crate with no [[bin]])
#   - "fabric-gui" is deliberately absent: the GUI is a Tauri app shipped as a
#     separate .msi/.exe asset by the build-gui job, and crates/fabric-gui has
#     no cargo binary target.
# Every name here is REQUIRED. If one is missing from the archive the install is
# partial — the user gets a broken toolchain while the script reports success —
# so a missing name is a hard error, not a warning. This mirrors install.sh.
$Bins = @('fabric', 'fabric-daemon', 'fabric-tui', 'fabric-tray', 'fabric-graph-cli')

if (-not $InstallDir) {
    $InstallDir = if ($env:LOCALAPPDATA) {
        Join-Path $env:LOCALAPPDATA 'PhenotypeFabric'
    } else {
        Join-Path $HOME '.local/bin'
    }
}

# ── Helpers ───────────────────────────────────────────────────────────
function Write-Info { param([string]$Msg) Write-Host "[info]  $Msg" -ForegroundColor Green }
function Write-Warn { param([string]$Msg) Write-Host "[warn]  $Msg" -ForegroundColor Yellow }
function Write-Die  { param([string]$Msg) Write-Host "[error] $Msg" -ForegroundColor Red; exit 1 }

# ── Detect architecture ──────────────────────────────────────────────
# Only the x64 archive is published today. Detect rather than assume, so an
# arm64 host gets a clear message instead of an x64 binary that will not run.
# -Arch overrides detection for staging or for testing on a non-Windows host.
if ($Arch) {
    $arch = $Arch
} elseif ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') {
    Write-Die "no Windows archive is published for ARM64 (only x64 today); pass -Arch x86_64 to override"
} else {
    $arch = 'x86_64'
}
if ($arch -ne 'x86_64') {
    Write-Die "unsupported -Arch '$arch'; only x86_64 is published for Windows"
}
$Platform = "windows-$arch"

# ── Fetch release tag ────────────────────────────────────────────────
function Get-ReleaseVersion {
    if ($Version) { return $Version }
    if ($env:FABRIC_VERSION) { return $env:FABRIC_VERSION }
    try {
        $headers = @{ 'User-Agent' = 'phenofabric-installer' }
        if ($env:GITHUB_TOKEN) { $headers['Authorization'] = "token $env:GITHUB_TOKEN" }
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

    $tag = Get-ReleaseVersion
    if (-not $tag) { Write-Die 'Could not determine the release version.' }
    Write-Info "Version: $tag"

    # The tag carries the leading `v` and so does the asset name.
    $Archive = "phenotype-fabric-$tag-$Platform.zip"
    $Url     = "https://github.com/$Repo/releases/download/$tag/$Archive"
    $TmpDir  = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null

    try {
        Write-Info "Downloading $Archive ..."
        $ProgressPreference = 'SilentlyContinue'
        $zip = Join-Path $TmpDir $Archive
        try {
            Invoke-WebRequest -Uri $Url -OutFile $zip -UseBasicParsing
            Invoke-WebRequest -Uri "$Url.sha256" -OutFile "$zip.sha256" -UseBasicParsing
        } catch {
            Write-Die "Download failed. Check that version '$tag' exists: $_"
        }

        # Verify before extracting: the archive is untrusted input and a
        # truncated download must never be installed.
        Write-Info 'Verifying sha256 ...'
        $expected = ((Get-Content "$zip.sha256" -Raw).Trim() -split '\s+')[0]
        if (-not $expected) { Write-Die "Checksum file was empty: $Url.sha256" }
        $actual = (Get-FileHash -Path $zip -Algorithm SHA256).Hash
        if ($actual -ne $expected) {
            Write-Die "Checksum mismatch for $Archive (expected $expected, got $actual)"
        }

        Write-Info 'Extracting ...'
        Expand-Archive -Path $zip -DestinationPath $TmpDir -Force

        $DistDir = Join-Path $TmpDir "phenotype-fabric-$tag-$Platform"
        if (-not (Test-Path $DistDir)) {
            Write-Die "Expected directory not found after extraction: $DistDir"
        }

        # Preflight: refuse to install anything if the archive is incomplete, so
        # a partial install never leaves a half-populated destination.
        $missing = @()
        foreach ($bin in $Bins) {
            if (-not (Test-Path (Join-Path $DistDir "$bin.exe"))) { $missing += $bin }
        }
        if ($missing.Count -gt 0) {
            $present = (Get-ChildItem -Path $DistDir -File | Select-Object -ExpandProperty Name) -join ' '
            Write-Die "Archive $Archive is missing REQUIRED binaries: $($missing -join ' ') -- nothing was installed. Contents: $present"
        }

        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

        Write-Info "Installing to $InstallDir ..."
        foreach ($bin in $Bins) {
            Copy-Item (Join-Path $DistDir "$bin.exe") (Join-Path $InstallDir "$bin.exe") -Force
            Write-Info "  Installed $bin"
        }

        # Add to the user PATH if absent. Only claim persistence on Windows:
        # .NET silently does nothing for the User scope on Unix, so printing
        # "added to PATH" off-Windows would be a false statement.
        $sep = [System.IO.Path]::PathSeparator
        $onWindows = ($env:OS -eq 'Windows_NT')
        $userPath = [Environment]::GetEnvironmentVariable('PATH', 'User')
        if ($userPath -notlike "*$InstallDir*") {
            if ($onWindows) {
                [Environment]::SetEnvironmentVariable('PATH', "$userPath;$InstallDir", 'User')
                Write-Info "Added $InstallDir to user PATH."
            } else {
                Write-Warn "not running on Windows: PATH was not persisted; add $InstallDir to PATH yourself if needed"
            }
            $env:PATH = "$env:PATH$sep$InstallDir"
        }

        echo ''
        Write-Info 'Installed binaries:'
        $count = 0
        foreach ($bin in $Bins) {
            $p = Join-Path $InstallDir "$bin.exe"
            if (Test-Path $p) { Write-Host ("  {0,-20} {1}" -f $bin, $p); $count++ }
            else { Write-Host ("  {0,-20} {1}" -f $bin, 'MISSING') }
        }
        if ($count -ne $Bins.Count) {
            Write-Die "Installed $count of $($Bins.Count) required binaries into $InstallDir."
        }

        # Report what the binary says its own version is, when this host can run it.
        $fabric = Join-Path $InstallDir 'fabric.exe'
        try { Write-Info "verify: $(& $fabric --version 2>&1 | Select-Object -First 1)" }
        catch { Write-Info 'note: could not execute fabric.exe on this host; skipping the run check' }

        echo ''
        Write-Info 'Get started: fabric --help'
    } finally {
        Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
        $ProgressPreference = 'Continue'
    }
}

Install-Fabric
