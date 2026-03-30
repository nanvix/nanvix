# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

<#
.SYNOPSIS
    Install per-user prerequisites for Nanvix on Windows Server 2022.

.DESCRIPTION
    This script runs in a regular (non-elevated) PowerShell session. It installs
    the Rust toolchain under the current user profile.

    The system-wide prerequisites must already be installed by running the
    administrator setup script (windows-server-admin.ps1).

    After running this script, clone the repository and run z.ps1 setup as
    described in doc/setup-windows-server.md.

.EXAMPLE
    # Run in a regular PowerShell prompt.
    .\scripts\setup\windows-server-user.ps1
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ==============================================================================
# Helpers
# ==============================================================================

function Write-Step  { param([string]$Msg) Write-Host "[STEP]  $Msg" -ForegroundColor Cyan }
function Write-Ok    { param([string]$Msg) Write-Host "[OK]    $Msg" -ForegroundColor Green }
function Write-Fail  { param([string]$Msg) Write-Host "[ERROR] $Msg" -ForegroundColor Red }

function Update-SessionPath {
    $env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
                [Environment]::GetEnvironmentVariable("Path", "User")
}

# ==============================================================================
# Pre-flight Checks
# ==============================================================================

Write-Step "Checking system-wide prerequisites..."
Update-SessionPath

$missing = @()
if (-not (Get-Command git    -ErrorAction SilentlyContinue)) { $missing += "Git" }
if (-not (Get-Command python -ErrorAction SilentlyContinue)) { $missing += "Python" }
if (-not (Get-Command make   -ErrorAction SilentlyContinue)) { $missing += "GNU Make" }

if ($missing.Count -gt 0) {
    Write-Fail "Missing system prerequisites: $($missing -join ', ')"
    Write-Host "  Run the administrator setup script first:" -ForegroundColor Yellow
    Write-Host "    .\scripts\setup\windows-server-admin.ps1" -ForegroundColor Cyan
    exit 1
}
Write-Ok "System prerequisites found (git, python, make)."

# ==============================================================================
# Install Rust Toolchain
# ==============================================================================

Write-Step "Installing Rust toolchain..."
$cargoDir = Join-Path $env:USERPROFILE ".cargo\bin"

if (Get-Command rustc -ErrorAction SilentlyContinue) {
    Write-Ok "Rust already installed: $(rustc --version)"
} elseif (Test-Path (Join-Path $cargoDir "rustc.exe")) {
    # rustc exists but is not on PATH yet.
    $env:Path = "$env:Path;$cargoDir"
    Write-Ok "Rust found at $cargoDir`: $(rustc --version)"
} else {
    $ProgressPreference = "SilentlyContinue"
    $installer = Join-Path $env:TEMP "rustup-init.exe"
    Write-Host "       Downloading rustup-init.exe..."
    Invoke-WebRequest `
        -Uri "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe" `
        -OutFile $installer -UseBasicParsing
    Write-Host "       Installing Rust (this may take a minute)..."
    & $installer -y --default-toolchain stable
    Remove-Item $installer -Force -ErrorAction SilentlyContinue

    # Add cargo bin to current session.
    if (Test-Path $cargoDir) {
        $env:Path = "$env:Path;$cargoDir"
    }
    Update-SessionPath

    if (Get-Command rustc -ErrorAction SilentlyContinue) {
        Write-Ok "Rust installed: $(rustc --version)"
    } else {
        Write-Fail "Rust installation failed. Ensure $cargoDir is on PATH."
        exit 1
    }
}

# ==============================================================================
# Summary
# ==============================================================================

Write-Host ""
Write-Host "======================================" -ForegroundColor Green
Write-Host " User setup complete." -ForegroundColor Green
Write-Host "======================================" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps (see doc/setup-windows-server.md):"
Write-Host "  1. Clone the repository:"
Write-Host "       git clone -c core.symlinks=true https://github.com/nanvix/nanvix.git" -ForegroundColor Cyan
Write-Host "  2. Run project setup:"
Write-Host "       cd nanvix && .\z.ps1 setup" -ForegroundColor Cyan
Write-Host ""
