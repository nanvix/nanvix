<#
.SYNOPSIS
    Applies system tuning to reduce noise during benchmarking.
    Requires administrator privileges (will not self-elevate).

.DESCRIPTION
    Configures the system for reproducible benchmark results:
    - Switches to High Performance power plan with CPU locked at 100%
    - Disables frequency scaling (P-state and EPP)
    - Adds Windows Defender exclusion for the repository
    - Stops noisy background services

    Two modes of operation:
    - With -SaveState: save current settings for later restore via bench-teardown.ps1
    - Without -SaveState: apply settings without saving (for dedicated machines)

.PARAMETER SaveState
    Save current system settings to .bench-state.json for later restoration
    via bench-teardown.ps1. Use this for local/user profiling sessions.

.PARAMETER RepoPath
    Path to the nanvix repository. Defaults to the parent of the scripts directory.

.PARAMETER Quiet
    Suppress informational output.

.EXAMPLE
    # Save state for later restore
    .\bench-setup.ps1 -SaveState

    # Apply without saving (dedicated machine)
    .\bench-setup.ps1
#>
param(
    [switch]$SaveState,
    [string]$RepoPath,
    [switch]$Quiet
)

$ErrorActionPreference = 'Stop'

# ── Require admin ─────────────────────────────────────────────────────────────
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    ).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Error "[bench-setup] This script requires administrator privileges. Run from an elevated shell."
    exit 1
}

# ── Resolve paths ─────────────────────────────────────────────────────────────
if (-not $RepoPath) {
    $RepoPath = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
    if (-not (Test-Path (Join-Path $RepoPath 'Cargo.toml'))) {
        $RepoPath = Split-Path $PSScriptRoot -Parent
    }
}
$stateFile = Join-Path $PSScriptRoot '.bench-state.json'

function Log($msg) { if (-not $Quiet) { Write-Host "[bench-setup] $msg" -ForegroundColor Cyan } }

# ── 1. Optionally save current state ─────────────────────────────────────────
if ($SaveState) {
    Log "Saving current system state to $stateFile..."

    $currentScheme = (powercfg /getactivescheme |
        Select-String '([0-9a-fA-F-]{36})').Matches[0].Value

    $procMin = (powercfg /query SCHEME_CURRENT SUB_PROCESSOR PROCTHROTTLEMIN |
        Select-String 'Current AC.*:\s*(0x[0-9a-fA-F]+)').Matches[0].Groups[1].Value
    $procMax = (powercfg /query SCHEME_CURRENT SUB_PROCESSOR PROCTHROTTLEMAX |
        Select-String 'Current AC.*:\s*(0x[0-9a-fA-F]+)').Matches[0].Groups[1].Value

    $wsearchStatus = (Get-Service WSearch -ErrorAction SilentlyContinue).Status
    $diagTrackStatus = (Get-Service DiagTrack -ErrorAction SilentlyContinue).Status
    $sysMainStatus = (Get-Service SysMain -ErrorAction SilentlyContinue).Status
    $tabletInputStatus = (Get-Service TabletInputService -ErrorAction SilentlyContinue).Status

    $state = @{
        PowerScheme    = $currentScheme
        ProcMin        = $procMin
        ProcMax        = $procMax
        Services       = @{
            WSearch            = "$wsearchStatus"
            DiagTrack          = "$diagTrackStatus"
            SysMain            = "$sysMainStatus"
            TabletInputService = "$tabletInputStatus"
        }
        RepoPath       = $RepoPath
        Timestamp      = (Get-Date -Format o)
    }
    # State is written to disk after Defender exclusion check (section 3)
    # so we can record whether setup added it.
}

# ── 2. Power plan: High Performance, lock CPU at 100% ────────────────────────
Log "Switching to High Performance power plan..."

$hpLine = powercfg /list | Select-String "High performance"
if ($hpLine) {
    $hpGuid = ($hpLine.ToString() -replace '.*:\s*([0-9a-fA-F-]{36}).*', '$1')
} else {
    Log "High Performance plan not found, creating from template..."
    $out = powercfg /duplicatescheme 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c
    $hpGuid = ($out | Select-String '([0-9a-fA-F-]{36})').Matches[0].Value
}

powercfg /setactive $hpGuid

# Lock processor frequency: no P-state or C-state transitions
powercfg -setacvalueindex SCHEME_CURRENT SUB_PROCESSOR PROCTHROTTLEMIN 100
powercfg -setacvalueindex SCHEME_CURRENT SUB_PROCESSOR PROCTHROTTLEMAX 100
# Energy Performance Preference: 0 = max performance
powercfg -setacvalueindex SCHEME_CURRENT SUB_PROCESSOR PERFEPP 0
powercfg -setactive SCHEME_CURRENT

$verify = powercfg /getactivescheme
Log "Active plan: $verify"

# ── 3. Windows Defender exclusion ─────────────────────────────────────────────
$currentExcl = @((Get-MpPreference).ExclusionPath | Where-Object { $_ })
$defenderExclusionAdded = $false
if ($currentExcl -notcontains $RepoPath) {
    Log "Adding Defender exclusion for $RepoPath..."
    Add-MpPreference -ExclusionPath $RepoPath
    $defenderExclusionAdded = $true
} else {
    Log "Defender exclusion already set for $RepoPath"
}

# Write saved state now that all pre-existing values are captured.
if ($SaveState -and $state) {
    $state['DefenderExclusionAdded'] = $defenderExclusionAdded
    $state | ConvertTo-Json -Depth 3 | Set-Content $stateFile -Encoding UTF8
    Log "State saved to $stateFile"
}

# ── 4. Stop noisy background services ────────────────────────────────────────
$noisyServices = @('WSearch', 'DiagTrack', 'SysMain', 'TabletInputService')
foreach ($svc in $noisyServices) {
    $s = Get-Service $svc -ErrorAction SilentlyContinue
    if ($s -and $s.Status -eq 'Running') {
        Log "Stopping $svc..."
        Stop-Service $svc -Force -ErrorAction SilentlyContinue
    }
}

# ── 5. Summary ────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "========================================================" -ForegroundColor Green
Write-Host "  Benchmark environment ready" -ForegroundColor Green
Write-Host "" -ForegroundColor Green
Write-Host "  - Power plan: High Performance (CPU locked 100%)" -ForegroundColor Green
Write-Host "  - Defender: repo excluded" -ForegroundColor Green
Write-Host "  - Background services: stopped" -ForegroundColor Green
if ($SaveState) {
    Write-Host "  - State saved: run bench-teardown.ps1 to restore" -ForegroundColor Green
}
Write-Host "" -ForegroundColor Green
Write-Host "  Run: .\scripts\bench\bench-run.ps1 -Iterations 50" -ForegroundColor Green
if ($SaveState) {
    Write-Host "  Revert: .\scripts\bench\bench-teardown.ps1" -ForegroundColor Green
}
Write-Host "========================================================" -ForegroundColor Green
