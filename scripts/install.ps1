# terminal-fabric installer for Windows (PowerShell)
# Usage: irm https://raw.githubusercontent.com/KooshaPari/terminal-fabric/main/scripts/install.ps1 | iex
$ErrorActionPreference = "Stop"

$Repo = "KooshaPari/terminal-fabric"
$Binary = "tf-web"

# Get latest version
$Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
$Version = $Release.tag_name -replace '^v', ''
$Asset = $Release.assets | Where-Object { $_.name -match "tf-web\.exe" } | Select-Object -First 1

if (-not $Asset) {
    Write-Error "No tf-web.exe found in release v$Version"
    exit 1
}

Write-Host "Downloading $Binary v$Version..."

$InstallDir = "$env:USERPROFILE\.tf"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

# Backup existing
$ExePath = Join-Path $InstallDir "$Binary.exe"
if (Test-Path $ExePath) {
    Rename-Item $ExePath "$Binary.exe.bak" -Force
}

# Download
Invoke-WebRequest -Uri $Asset.browser_download_url -OutFile $ExePath

Write-Host "Installed $Binary v$Version to $ExePath"
Write-Host "Add $InstallDir to PATH if not already present."