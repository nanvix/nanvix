<#
.SYNOPSIS
    Reverts all changes made by bench-setup.ps1 -SaveState.
    Requires administrator privileges (will not self-elevate).

.DESCRIPTION
    Reads the saved state from .bench-state.json and restores:
    - Original power plan and processor throttle settings
    - Previously running background services
    - Removes Windows Defender exclusion (optional)

.PARAMETER KeepDefenderExclusion
    Keep the Defender exclusion in place (useful for dev work).
    Default behavior is to remove it.

.EXAMPLE
    .\bench-teardown.ps1
    .\bench-teardown.ps1 -KeepDefenderExclusion
#>
param(
    [switch]$KeepDefenderExclusion
)

$ErrorActionPreference = 'Stop'
$stateFile = Join-Path $PSScriptRoot '.bench-state.json'

# ── Require admin ─────────────────────────────────────────────────────────────
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    ).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Error "[bench-teardown] This script requires administrator privileges. Run from an elevated shell."
    exit 1
}

function Log($msg) { Write-Host "[bench-teardown] $msg" -ForegroundColor Yellow }

# ── Check for state file ─────────────────────────────────────────────────────
if (-not (Test-Path $stateFile)) {
    Write-Host "[bench-teardown] No state file found at $stateFile" -ForegroundColor Red
    Write-Host "[bench-teardown] Nothing to revert. Was bench-setup.ps1 run with -SaveState?" -ForegroundColor Red
    exit 0
}

$state = Get-Content $stateFile -Encoding UTF8 | ConvertFrom-Json
Log "Restoring state from $($state.Timestamp)..."

# ── 1. Restore power plan ────────────────────────────────────────────────────
Log "Restoring power plan to $($state.PowerScheme)..."
powercfg /setactive $state.PowerScheme

# Restore processor throttle values
$minHex = $state.ProcMin -replace '^0x', ''
$maxHex = $state.ProcMax -replace '^0x', ''
$minVal = [Convert]::ToInt32($minHex, 16)
$maxVal = [Convert]::ToInt32($maxHex, 16)
powercfg -setacvalueindex SCHEME_CURRENT SUB_PROCESSOR PROCTHROTTLEMIN $minVal
powercfg -setacvalueindex SCHEME_CURRENT SUB_PROCESSOR PROCTHROTTLEMAX $maxVal
powercfg -setactive SCHEME_CURRENT

$verify = powercfg /getactivescheme
Log "Active plan: $verify"

# ── 2. Restart services that were running before ─────────────────────────────
$servicesToRestore = @('WSearch', 'DiagTrack', 'SysMain', 'TabletInputService')
foreach ($svcName in $servicesToRestore) {
    $savedStatus = $state.Services.$svcName
    if ($savedStatus -eq 'Running') {
        $svc = Get-Service $svcName -ErrorAction SilentlyContinue
        if ($svc -and $svc.Status -ne 'Running') {
            Log "Restarting $svcName..."
            Start-Service $svcName -ErrorAction SilentlyContinue
        }
    }
}

# ── 3. Optionally remove Defender exclusion ──────────────────────────────────
$setupAddedExclusion = ($state.PSObject.Properties.Name -contains 'DefenderExclusionAdded') -and [bool]$state.DefenderExclusionAdded
if (-not $KeepDefenderExclusion -and $setupAddedExclusion -and $state.RepoPath) {
    $currentExcl = @((Get-MpPreference).ExclusionPath | Where-Object { $_ })
    if ($currentExcl -contains $state.RepoPath) {
        Log "Removing Defender exclusion for $($state.RepoPath)..."
        Remove-MpPreference -ExclusionPath $state.RepoPath
    }
} else {
    if (-not $setupAddedExclusion -and -not $KeepDefenderExclusion) {
        Log "Defender exclusion was not added by setup; leaving it in place."
    } else {
        Log "Keeping Defender exclusion in place."
    }
}

# ── 4. Cleanup ────────────────────────────────────────────────────────────────
Remove-Item $stateFile -Force
Log "State file cleaned up."

Write-Host ""
Write-Host "========================================================" -ForegroundColor Yellow
Write-Host "  System restored to normal state" -ForegroundColor Yellow
Write-Host "========================================================" -ForegroundColor Yellow
