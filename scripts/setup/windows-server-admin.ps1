# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

<#
.SYNOPSIS
    Install system-wide prerequisites for Nanvix on Windows Server 2022.

.DESCRIPTION
    This script must be run in an elevated (Administrator) PowerShell session.
    It installs Chocolatey, Git, Python 3.12, GNU Make, Visual Studio Build
    Tools (C++ workload), enables Developer Mode, and enables the Windows
    Hypervisor Platform.

    After running this script, each developer must run the user-level setup
    script (windows-server-user.ps1) in a regular PowerShell session.

.EXAMPLE
    # Run in an elevated PowerShell prompt.
    .\scripts\setup\windows-server-admin.ps1
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ==============================================================================
# Helpers
# ==============================================================================

function Write-Step  { param([string]$Msg) Write-Host "[STEP]  $Msg" -ForegroundColor Cyan }
function Write-Ok    { param([string]$Msg) Write-Host "[OK]    $Msg" -ForegroundColor Green }
function Write-Warn  { param([string]$Msg) Write-Host "[WARN]  $Msg" -ForegroundColor Yellow }
function Write-Fail  { param([string]$Msg) Write-Host "[ERROR] $Msg" -ForegroundColor Red }

function Assert-Admin {
    $current = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    if (-not $current.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Write-Fail "This script must be run as Administrator."
        exit 1
    }
}

# Refresh the session PATH from the registry so newly installed tools are found.
function Update-SessionPath {
    $env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
                [Environment]::GetEnvironmentVariable("Path", "User")
}

# ==============================================================================
# Main
# ==============================================================================

Assert-Admin
$ProgressPreference = "SilentlyContinue"

# --- Enable Developer Mode ---------------------------------------------------
Write-Step "Enabling Developer Mode..."
reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock" `
    /t REG_DWORD /f /v AllowDevelopmentWithoutDevLicense /d 1 | Out-Null
Write-Ok "Developer Mode enabled."

# --- Install Chocolatey -------------------------------------------------------
Write-Step "Installing Chocolatey..."
if (Get-Command choco -ErrorAction SilentlyContinue) {
    Write-Ok "Chocolatey already installed."
} else {
    Set-ExecutionPolicy Bypass -Scope Process -Force
    [System.Net.ServicePointManager]::SecurityProtocol =
        [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
    Invoke-Expression (
        (New-Object System.Net.WebClient).DownloadString(
            "https://community.chocolatey.org/install.ps1"
        )
    )
    Update-SessionPath
    Write-Ok "Chocolatey installed."
}

# --- Install Git --------------------------------------------------------------
Write-Step "Installing Git..."
Update-SessionPath
if (Get-Command git -ErrorAction SilentlyContinue) {
    Write-Ok "Git already installed: $(git --version)"
} else {
    choco install git -y --no-progress | Out-Null
    Update-SessionPath
    if (Get-Command git -ErrorAction SilentlyContinue) {
        Write-Ok "Git installed: $(git --version)"
    } else {
        Write-Fail "Git installation failed."
        exit 1
    }
}

# --- Install Python 3.12 -----------------------------------------------------
Write-Step "Installing Python 3.12..."
Update-SessionPath
if (Get-Command python -ErrorAction SilentlyContinue) {
    $pyVer = (python --version 2>&1).ToString()
    Write-Ok "Python already installed: $pyVer"
} else {
    choco install python312 -y --no-progress | Out-Null
    Update-SessionPath
    if (Get-Command python -ErrorAction SilentlyContinue) {
        Write-Ok "Python installed: $(python --version 2>&1)"
    } else {
        Write-Fail "Python installation failed."
        exit 1
    }
}

# --- Install GNU Make ---------------------------------------------------------
Write-Step "Installing GNU Make..."
Update-SessionPath
if (Get-Command make -ErrorAction SilentlyContinue) {
    Write-Ok "GNU Make already installed: $(make --version | Select-Object -First 1)"
} else {
    choco install make -y --no-progress | Out-Null
    Update-SessionPath
    if (Get-Command make -ErrorAction SilentlyContinue) {
        Write-Ok "GNU Make installed: $(make --version | Select-Object -First 1)"
    } else {
        Write-Fail "GNU Make installation failed."
        exit 1
    }
}

# --- Install Visual Studio Build Tools ----------------------------------------
Write-Step "Installing Visual Studio Build Tools (C++ workload)..."
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $vswhere) {
    $vsPath = & $vswhere -latest -property installationPath 2>$null
    if ($vsPath) {
        Write-Ok "Visual Studio Build Tools already installed at: $vsPath"
    }
} else {
    Write-Host "       Downloading installer (this may take a few minutes)..."
    $installer = Join-Path $env:TEMP "vs_buildtools.exe"
    Invoke-WebRequest -Uri "https://aka.ms/vs/17/release/vs_buildtools.exe" `
        -OutFile $installer -UseBasicParsing
    Write-Host "       Installing (this may take several minutes)..."
    Start-Process -FilePath $installer -ArgumentList `
        "--quiet", "--wait", "--norestart", "--nocache",
        "--add", "Microsoft.VisualStudio.Workload.VCTools",
        "--includeRecommended" -Wait -NoNewWindow
    Remove-Item $installer -Force -ErrorAction SilentlyContinue
    Write-Ok "Visual Studio Build Tools installed."
}

# --- Enable Windows Hypervisor Platform ---------------------------------------
Write-Step "Enabling Windows Hypervisor Platform..."
$whp = Get-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform
if ($whp.State -eq "Enabled") {
    Write-Ok "Windows Hypervisor Platform already enabled."
} else {
    Enable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -All -NoRestart | Out-Null
    Write-Ok "Windows Hypervisor Platform enabled."
    Write-Warn "A reboot is required before WHP can be used."
}

# --- Summary ------------------------------------------------------------------
Write-Host ""
Write-Host "======================================" -ForegroundColor Green
Write-Host " Administrator setup complete." -ForegroundColor Green
Write-Host "======================================" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. Reboot if WHP was just enabled."
Write-Host "  2. Open a regular (non-elevated) PowerShell session."
Write-Host "  3. Run the user setup script:"
Write-Host "       .\scripts\setup\windows-server-user.ps1" -ForegroundColor Cyan
Write-Host ""
