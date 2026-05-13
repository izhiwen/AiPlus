# install.ps1 — AiPlus Windows installer (PowerShell)
#
# Run interactively from PowerShell:
#   iwr -useb https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.ps1 | iex
#
# Environment overrides:
#   $env:AIPLUS_VERSION      Release tag (e.g. "v0.5.9"); default = latest GitHub release
#   $env:AIPLUS_INSTALL_DIR  Install directory; default = $HOME\.local\bin
#
# Safety boundaries (parity with install.sh):
#   - Downloads only the official GitHub Release asset and its checksum.
#   - Verifies SHA-256 against the published checksums.txt before installing.
#   - Installs only the `aiplus.exe` binary. Does not touch your PATH,
#     PowerShell profile, sudo, Defender exclusions, or any global Codex /
#     Claude Code / OpenCode config. Does not collect telemetry or upload
#     data.

[CmdletBinding()]
param(
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$Repo   = "izhiwen/aiplus"
$Target = "x86_64-pc-windows-msvc"
$Asset  = "aiplus-$Target.zip"

# --- 1. Resolve version --------------------------------------------------------
$Version = $env:AIPLUS_VERSION
if (-not $Version) {
    try {
        $latest = Invoke-RestMethod -UseBasicParsing `
            -Uri "https://api.github.com/repos/$Repo/releases/latest"
        $Version = $latest.tag_name
    } catch {
        Write-Warning "Could not query latest release; falling back to v0.5.9"
        $Version = "v0.5.9"
    }
}
$Version = $Version.Trim()
if (-not $Version) { $Version = "v0.5.9" }

# --- 2. Resolve install dir ---------------------------------------------------
$InstallDir = $env:AIPLUS_INSTALL_DIR
if (-not $InstallDir) {
    $InstallDir = Join-Path $HOME ".local\bin"
}

# --- 3. Print plan ------------------------------------------------------------
Write-Host "AiPlus installer (Windows)"
Write-Host "version=$Version"
Write-Host "asset=$Asset"
Write-Host "install_dir=$InstallDir"
Write-Host "writes=$InstallDir\aiplus.exe"
Write-Host "shell_profile_edits=none"
Write-Host "telemetry=none"

$BaseUrl = "https://github.com/$Repo/releases/download/$Version"

if ($DryRun) {
    Write-Host "DRY_RUN=YES"
    Write-Host "download=$BaseUrl/$Asset"
    Write-Host "checksums=$BaseUrl/checksums.txt"
    return
}

# --- 4. Download to temp ------------------------------------------------------
$Tmp = New-Item -ItemType Directory -Path (Join-Path $env:TEMP "aiplus-install-$([Guid]::NewGuid())")
try {
    $AssetPath     = Join-Path $Tmp $Asset
    $ChecksumsPath = Join-Path $Tmp "checksums.txt"

    Write-Host "Downloading $Asset ..."
    Invoke-WebRequest -UseBasicParsing -Uri "$BaseUrl/$Asset"           -OutFile $AssetPath
    Invoke-WebRequest -UseBasicParsing -Uri "$BaseUrl/checksums.txt"   -OutFile $ChecksumsPath

    # --- 5. Verify SHA-256 ----------------------------------------------------
    $expected = (Get-Content $ChecksumsPath |
                 Where-Object { $_ -match "^\s*([0-9a-fA-F]{64})\s+$([Regex]::Escape($Asset))\s*$" } |
                 ForEach-Object { $Matches[1].ToLower() } |
                 Select-Object -First 1)
    if (-not $expected) {
        throw "ERROR checksum not found for $Asset in checksums.txt"
    }
    $actual = (Get-FileHash -Algorithm SHA256 $AssetPath).Hash.ToLower()
    if ($actual -ne $expected) {
        throw "ERROR checksum mismatch for $Asset (expected $expected, got $actual)"
    }
    Write-Host "checksum=OK"

    # --- 6. Extract -----------------------------------------------------------
    $ExtractDir = Join-Path $Tmp "extract"
    Expand-Archive -Path $AssetPath -DestinationPath $ExtractDir -Force
    $Bin = Get-ChildItem -Path $ExtractDir -Filter "aiplus.exe" -Recurse | Select-Object -First 1
    if (-not $Bin) {
        throw "ERROR release archive did not contain aiplus.exe"
    }

    # --- 7. Install -----------------------------------------------------------
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir | Out-Null
    }
    Copy-Item -Path $Bin.FullName -Destination (Join-Path $InstallDir "aiplus.exe") -Force

    Write-Host "INSTALL_STATUS=PASS"
    Write-Host "installed=$InstallDir\aiplus.exe"

    # --- 8. PATH advisory (do NOT auto-edit the user's profile) ---------------
    $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if (-not ($userPath -split ';' | Where-Object { $_ -eq $InstallDir })) {
        Write-Host "PATH_NOTICE=$InstallDir is not on your user PATH"
        Write-Host "To add it for future PowerShell sessions, run:"
        Write-Host "  [Environment]::SetEnvironmentVariable('PATH', `"`$env:PATH;$InstallDir`", 'User')"
    }

    Write-Host "Next:"
    Write-Host "  cd MyProject"
    Write-Host "  aiplus install codex"

} finally {
    if (Test-Path $Tmp) { Remove-Item -Recurse -Force $Tmp }
}
