# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

<#
.SYNOPSIS
    Ensures the correct version of Verus is installed on Windows.

.DESCRIPTION
    Reads the expected version from build/verus-version, checks whether Verus is
    already present at that version in the target directory, and downloads/installs
    it when missing or outdated. Downloads the prebuilt Windows binary from GitHub
    releases.

.PARAMETER InstallDir
    Directory where Verus will be installed. Defaults to $env:USERPROFILE\verus.

.PARAMETER DownloadOnly
    Download the Verus archive to the local cache and exit without installing.
    Useful for CI pre-warming.

.EXAMPLE
    .\scripts\setup\verus.ps1 C:\verus
    .\scripts\setup\verus.ps1 -DownloadOnly C:\verus

.NOTES
    Environment Variables:
      GITHUB_TOKEN        GitHub token for authenticated downloads (optional).
      GH_TOKEN            Alternative to GITHUB_TOKEN (GitHub CLI convention).
      VERUS_ZIP_CACHE_DIR Override the default cache directory (.verus-cache).
#>

param(
    [Parameter(Position = 0)]
    [string]$InstallDir = "$env:USERPROFILE\verus",

    [switch]$DownloadOnly
)

# Fail fast on errors.
$ErrorActionPreference = "Stop"

# ==================================================================================================
# Constants
# ==================================================================================================

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (git rev-parse --show-toplevel 2>$null)
if (-not $RepoRoot) {
    $RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..\..")).Path
}

$VerusVersionFile = Join-Path $RepoRoot "build\verus-version"
$VerusReleaseUrl = "https://github.com/verus-lang/verus/releases/download/release"
$VerusZipSubdir = "verus-x86-win"

if ($env:VERUS_ZIP_CACHE_DIR) {
    $CacheDir = $env:VERUS_ZIP_CACHE_DIR
} else {
    $CacheDir = Join-Path $RepoRoot ".verus-cache"
}

# ==================================================================================================
# Helper Functions
# ==================================================================================================

function Write-Info { param([string]$Msg) Write-Host "[INFO] $Msg" -ForegroundColor Cyan }
function Write-Ok   { param([string]$Msg) Write-Host "[OK]   $Msg" -ForegroundColor Green }
function Write-Warn { param([string]$Msg) Write-Host "[WARN] $Msg" -ForegroundColor Yellow }
function Write-Err  { param([string]$Msg) Write-Host "[ERROR] $Msg" -ForegroundColor Red }

function Get-ExpectedVersion {
    if (-not (Test-Path $VerusVersionFile)) {
        Write-Err "Verus version file not found: $VerusVersionFile"
        exit 1
    }
    $version = (Get-Content $VerusVersionFile -Raw).Trim()
    if (-not $version) {
        Write-Err "Verus version file is empty: $VerusVersionFile"
        exit 1
    }
    return $version
}

function Get-InstalledVersion {
    param([string]$Dir)
    $versionFile = Join-Path $Dir "version.txt"
    if (Test-Path $versionFile) {
        return (Get-Content $versionFile -Raw).Trim()
    }
    return ""
}

function Get-AuthHeaders {
    $token = if ($env:GITHUB_TOKEN) { $env:GITHUB_TOKEN } elseif ($env:GH_TOKEN) { $env:GH_TOKEN } else { "" }
    if ($token) {
        return @{ Authorization = "Bearer $token" }
    }
    return @{}
}

function Test-ZipArchive {
    param([string]$ZipPath)
    try {
        $zip = [System.IO.Compression.ZipFile]::OpenRead($ZipPath)
        $zip.Dispose()
        return $true
    } catch {
        return $false
    }
}

function Get-VerusArchive {
    param(
        [string]$DestPath,
        [string]$Version
    )

    $zipName = "verus-${Version}-x86-win.zip"
    $downloadUrl = "${VerusReleaseUrl}/${Version}/${zipName}"

    # Try the local zip cache first.
    if ($CacheDir) {
        $cachedZip = Join-Path $CacheDir $zipName
        if ((Test-Path $cachedZip) -and (Get-Item $cachedZip).Length -gt 0) {
            Add-Type -AssemblyName System.IO.Compression.FileSystem
            if (Test-ZipArchive $cachedZip) {
                Write-Info "Using cached Verus archive from $cachedZip"
                Copy-Item $cachedZip $DestPath -Force
                return
            } else {
                Write-Warn "Cached Verus archive at $cachedZip is corrupted; deleting and re-downloading."
                Remove-Item $cachedZip -Force
            }
        }
    }

    Write-Info "Downloading Verus $Version from $downloadUrl ..."
    $headers = Get-AuthHeaders

    $maxRetries = 5
    $retryDelay = 10
    for ($attempt = 1; $attempt -le $maxRetries; $attempt++) {
        try {
            $webArgs = @{
                Uri     = $downloadUrl
                OutFile = $DestPath
            }
            if ($headers.Count -gt 0) { $webArgs.Headers = $headers }
            Invoke-WebRequest @webArgs
            break
        } catch {
            if ($attempt -eq $maxRetries) {
                Write-Err "Failed to download Verus from $downloadUrl after $maxRetries attempts."
                Write-Err $_.Exception.Message
                exit 1
            }
            Write-Warn "Download attempt $attempt failed; retrying in ${retryDelay}s..."
            Start-Sleep -Seconds $retryDelay
        }
    }

    # Persist the downloaded archive for future runs.
    if ($CacheDir) {
        if (-not (Test-Path $CacheDir)) {
            New-Item -ItemType Directory -Path $CacheDir -Force | Out-Null
        }
        Copy-Item $DestPath (Join-Path $CacheDir $zipName) -Force
    }
}

function Install-Verus {
    param(
        [string]$Dir,
        [string]$Version
    )

    $zipName = "verus-${Version}-x86-win.zip"

    # Validate that the install directory looks reasonable.
    if ($Dir -notmatch "verus") {
        Write-Err "Install directory '$Dir' does not contain 'verus'. Aborting for safety."
        exit 1
    }

    # Create a temporary directory for the download.
    $tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "verus-install-$([guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

    try {
        $zipPath = Join-Path $tmpDir $zipName
        Get-VerusArchive -DestPath $zipPath -Version $Version

        Write-Info "Extracting Verus to $Dir ..."
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $extractDir = Join-Path $tmpDir "extract"
        [System.IO.Compression.ZipFile]::ExtractToDirectory($zipPath, $extractDir)

        # Verify the expected subdirectory exists in the archive.
        $subDir = Join-Path $extractDir $VerusZipSubdir
        if (-not (Test-Path $subDir)) {
            Write-Err "Expected directory '$VerusZipSubdir' not found in archive."
            exit 1
        }

        # Replace the install directory contents.
        if (-not (Test-Path $Dir)) {
            New-Item -ItemType Directory -Path $Dir -Force | Out-Null
        } else {
            Get-ChildItem $Dir | Remove-Item -Recurse -Force
        }
        Get-ChildItem $subDir | Copy-Item -Destination $Dir -Recurse -Force

        Write-Ok "Verus $Version installed to $Dir."
    } finally {
        if (Test-Path $tmpDir) {
            Remove-Item $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

function Confirm-VerusToolchain {
    param([string]$Dir)

    $versionJson = Join-Path $Dir "version.json"
    if (-not (Test-Path $versionJson)) { return }
    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) { return }

    try {
        $data = Get-Content $versionJson -Raw | ConvertFrom-Json
        $toolchain = $data.verus.toolchain
        if (-not $toolchain) { return }

        $null = rustup run $toolchain rustc --version 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Info "Installing Rust toolchain '$toolchain' required by Verus..."
            rustup toolchain install $toolchain --profile minimal --component rust-src
        }
    } catch {
        Write-Warn "Failed to parse $versionJson; skipping toolchain check."
    }
}

# ==================================================================================================
# Main
# ==================================================================================================

$expectedVersion = Get-ExpectedVersion

if ($DownloadOnly) {
    $zipName = "verus-${expectedVersion}-x86-win.zip"
    $tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "verus-dl-$([guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null
    try {
        Get-VerusArchive -DestPath (Join-Path $tmpDir $zipName) -Version $expectedVersion

        # Validate the cached archive.
        $cachedZip = Join-Path $CacheDir $zipName
        if ((Test-Path $cachedZip) -and (Get-Item $cachedZip).Length -gt 0) {
            Add-Type -AssemblyName System.IO.Compression.FileSystem
            if (-not (Test-ZipArchive $cachedZip)) {
                Write-Err "Downloaded Verus archive failed validation: $cachedZip"
                Remove-Item $cachedZip -Force
                exit 1
            }
        } else {
            Write-Err "Verus archive not found in cache after download: $cachedZip"
            exit 1
        }

        Write-Ok "Verus $expectedVersion archive cached in $CacheDir."
    } finally {
        if (Test-Path $tmpDir) { Remove-Item $tmpDir -Recurse -Force -ErrorAction SilentlyContinue }
    }
    exit 0
}

$installedVersion = Get-InstalledVersion -Dir $InstallDir

if ($installedVersion -eq $expectedVersion) {
    Write-Info "Verus $expectedVersion is already installed in $InstallDir."
    Confirm-VerusToolchain -Dir $InstallDir
    exit 0
}

if ($installedVersion) {
    Write-Info "Verus version mismatch (found '$installedVersion', expected '$expectedVersion'). Updating..."
} else {
    Write-Info "Verus not found in $InstallDir. Installing..."
}

Install-Verus -Dir $InstallDir -Version $expectedVersion
Confirm-VerusToolchain -Dir $InstallDir
