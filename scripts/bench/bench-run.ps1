<#
.SYNOPSIS
    Runs nanvix-bench with CPU pinning, Realtime priority, and high timer resolution.
    Optionally captures a WPR trace during execution.
    Requires administrator privileges (will not self-elevate).

.DESCRIPTION
    This script is the core benchmark runner for local profiling.
    It does NOT call bench-setup or bench-teardown — the caller is responsible.

    Features:
    - Sets CPU affinity to isolate benchmark from OS noise
    - Sets Realtime process priority
    - Sets timer resolution to 0.5ms
    - Captures stdout and stderr to separate files
    - Optionally runs WPR trace capture around the benchmark

.PARAMETER Benchmark
    Benchmark name (default: cold-start)

.PARAMETER Iterations
    Number of iterations (default: 50)

.PARAMETER AffinityMask
    CPU affinity mask. Default 0xF00 = LPs 8-11 (physical cores 4-5 on 6C/12T).

.PARAMETER OutputDir
    Directory for output files. Default: current directory.

.PARAMETER WPR
    Enable WPR trace capture during the benchmark run.
    Requires wpr-profile.wprp in the same directory as this script.

.PARAMETER ExtraArgs
    Additional arguments passed to nanvix-bench.exe

.EXAMPLE
    # Basic run
    .\bench-run.ps1 -Iterations 50

    # With WPR trace
    .\bench-run.ps1 -Iterations 10 -WPR -OutputDir .\results

    # Custom affinity and benchmark
    .\bench-run.ps1 -Benchmark warm-start-vmm -AffinityMask 0xC00 -Iterations 30
#>
param(
    [string]$Benchmark = 'cold-start',
    [int]$Iterations = 50,
    [UInt64]$AffinityMask = 0xF00,
    [string]$OutputDir = '.',
    [switch]$WPR,
    [string[]]$ExtraArgs = @()
)

$ErrorActionPreference = 'Stop'

# ── Require admin ─────────────────────────────────────────────────────────────
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    ).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Error "[bench-run] This script requires administrator privileges. Run from an elevated shell."
    exit 1
}

# ── Resolve paths ─────────────────────────────────────────────────────────────
$scriptDir = $PSScriptRoot
$repoRoot = Split-Path (Split-Path $scriptDir -Parent) -Parent
if (-not (Test-Path (Join-Path $repoRoot 'Cargo.toml'))) {
    $repoRoot = Split-Path $scriptDir -Parent
}
$benchExe = Join-Path $repoRoot 'bin\nanvix-bench.exe'

if (-not (Test-Path $benchExe)) {
    Write-Error "[bench-run] $benchExe not found. Build first: .\z.ps1 build --profile --release -- LOG_LEVEL=panic"
    exit 1
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$stdoutFile = Join-Path $OutputDir "${Benchmark}-${timestamp}-stdout.txt"
$stderrFile = Join-Path $OutputDir "${Benchmark}-${timestamp}-stderr.txt"

# ── WPR profile path ─────────────────────────────────────────────────────────
$wprProfile = Join-Path $scriptDir 'wpr-profile.wprp'
if ($WPR -and -not (Test-Path $wprProfile)) {
    Write-Error "[bench-run] WPR profile not found at $wprProfile"
    exit 1
}
$etlFile = Join-Path $OutputDir "${Benchmark}-${timestamp}.etl"

# ── Set timer resolution to 0.5ms ────────────────────────────────────────────
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class BenchTimerRes {
    [DllImport("ntdll.dll")] public static extern int NtSetTimerResolution(uint DesiredRes, bool SetRes, out uint CurrentRes);
}
'@

$curRes = [uint32]0
[void][BenchTimerRes]::NtSetTimerResolution(5000, $true, [ref]$curRes)
Write-Host "[bench-run] Timer resolution set to $($curRes / 10000)ms" -ForegroundColor Cyan

# Ensure timer resolution is restored even if the script fails.
$timerRestored = $false
function Restore-TimerResolution {
    if (-not $script:timerRestored) {
        $restoreRes = [uint32]0
        [void][BenchTimerRes]::NtSetTimerResolution(5000, $false, [ref]$restoreRes)
        $script:timerRestored = $true
    }
}
trap { Restore-TimerResolution; throw }

# ── Verify system setup ──────────────────────────────────────────────────────
$plan = (powercfg /getactivescheme).ToString()
if ($plan -notmatch 'High performance') {
    Write-Host "[bench-run] WARNING: Not on High Performance plan. Run bench-setup.ps1 first!" -ForegroundColor Yellow
}

# ── Format affinity info ─────────────────────────────────────────────────────
$pinnedCores = @()
for ($i = 0; $i -lt 64; $i++) {
    if ($AffinityMask -band ([UInt64]1 -shl $i)) { $pinnedCores += $i }
}
Write-Host "[bench-run] CPU affinity: 0x$($AffinityMask.ToString('X')) (LPs: $($pinnedCores -join ', '))" -ForegroundColor Cyan

# ── Build argument list ──────────────────────────────────────────────────────
$benchArgs = @('-benchmark', $Benchmark, '-iterations', $Iterations) + $ExtraArgs
Write-Host "[bench-run] Running: nanvix-bench.exe $($benchArgs -join ' ')" -ForegroundColor Cyan
Write-Host "[bench-run] Priority: Realtime | Output: $stdoutFile" -ForegroundColor Cyan
Write-Host "--------------------------------------------------------" -ForegroundColor DarkGray

# ── Start WPR trace ──────────────────────────────────────────────────────────
if ($WPR) {
    Write-Host "[bench-run] Starting WPR trace..." -ForegroundColor Magenta
    # Cancel any stale session first (errors are expected if no session is active).
    $ErrorActionPreference = 'Continue'
    & wpr -cancel 2>$null
    & wpr -start "$wprProfile!NanvixBench" -filemode
    $wprStartResult = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    if ($wprStartResult -ne 0) {
        Write-Host "[bench-run] WARNING: WPR start failed (exit $wprStartResult). Continuing without trace." -ForegroundColor Yellow
        $WPR = $false
    } else {
        Write-Host "[bench-run] WPR trace active." -ForegroundColor Magenta
    }
}

# ── Launch benchmark ─────────────────────────────────────────────────────────
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $benchExe
$psi.Arguments = $benchArgs -join ' '
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.WorkingDirectory = $repoRoot

$proc = [System.Diagnostics.Process]::Start($psi)

try {
    # Cast UInt64 to IntPtr via Int64 to avoid type conversion errors on 64-bit systems.
    $proc.ProcessorAffinity = [IntPtr][Int64]$AffinityMask
    $proc.PriorityClass = [System.Diagnostics.ProcessPriorityClass]::RealTime
} catch {
    Write-Host "[bench-run] WARNING: Could not set affinity/priority: $_" -ForegroundColor Yellow
}

# Read stdout/stderr asynchronously to avoid pipe deadlocks.
$stdoutTask = $proc.StandardOutput.ReadToEndAsync()
$stderrTask = $proc.StandardError.ReadToEndAsync()
$proc.WaitForExit()
$stdout = $stdoutTask.GetAwaiter().GetResult()
$stderr = $stderrTask.GetAwaiter().GetResult()

# ── Stop WPR trace ───────────────────────────────────────────────────────────
if ($WPR) {
    Write-Host "[bench-run] Stopping WPR trace..." -ForegroundColor Magenta
    $ErrorActionPreference = 'Continue'
    & wpr -stop "$etlFile" "Nanvix $Benchmark benchmark"
    $wprStopResult = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    if ($wprStopResult -eq 0) {
        Write-Host "[bench-run] ETL saved: $etlFile" -ForegroundColor Magenta

        # Merge trace for symbol resolution (xperf -merge embeds image info)
        $mergedEtl = [IO.Path]::ChangeExtension($etlFile, '-merged.etl')
        $xperfPath = "C:\Program Files (x86)\Windows Kits\10\Windows Performance Toolkit\xperf.exe"
        if (-not (Test-Path $xperfPath)) {
            $xperfPath = "C:\Program Files\Windows Kits\10\Windows Performance Toolkit\xperf.exe"
        }
        if (Test-Path $xperfPath) {
            Write-Host "[bench-run] Merging trace for symbol resolution..." -ForegroundColor Magenta
            & $xperfPath -merge $etlFile $mergedEtl
            if ($LASTEXITCODE -eq 0 -and (Test-Path $mergedEtl)) {
                $mergedMB = [math]::Round((Get-Item $mergedEtl).Length / 1MB, 1)
                Write-Host "[bench-run] Merged ETL: $mergedEtl ($mergedMB MB)" -ForegroundColor Magenta
            } else {
                Write-Host "[bench-run] WARNING: xperf -merge failed. Use raw ETL for analysis." -ForegroundColor Yellow
            }
        } else {
            Write-Host "[bench-run] WARNING: xperf not found -- skipping merge. Install Windows Performance Toolkit." -ForegroundColor Yellow
        }
    } else {
        Write-Host "[bench-run] WARNING: WPR stop failed (exit $wprStopResult)." -ForegroundColor Yellow
    }
}

# ── Save outputs ─────────────────────────────────────────────────────────────
$stdout | Set-Content $stdoutFile -Encoding UTF8
$stderr | Set-Content $stderrFile -Encoding UTF8

Write-Host "--------------------------------------------------------" -ForegroundColor DarkGray

# Print stdout to console
if ($stdout) { Write-Host $stdout }

# Print stderr summary (PERF_TIMINGS lines are verbose — just count them)
$perfLines = ($stderr -split "`n") | Where-Object { $_ -match 'PERF_TIMINGS:' }
if ($perfLines.Count -gt 0) {
    Write-Host "[bench-run] Captured $($perfLines.Count) PERF_TIMINGS records in $stderrFile" -ForegroundColor Cyan
}
$nonPerfStderr = ($stderr -split "`n") | Where-Object { $_ -and $_ -notmatch 'PERF_TIMINGS:' }
if ($nonPerfStderr.Count -gt 0) {
    Write-Host ($nonPerfStderr -join "`n") -ForegroundColor Yellow
}

# ── Restore timer resolution ─────────────────────────────────────────────────
Restore-TimerResolution

# ── Summary ──────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "[bench-run] Exit code: $($proc.ExitCode)" -ForegroundColor $(if ($proc.ExitCode -eq 0) { 'Green' } else { 'Red' })
Write-Host "[bench-run] stdout: $stdoutFile" -ForegroundColor Cyan
Write-Host "[bench-run] stderr: $stderrFile" -ForegroundColor Cyan
if ($WPR) {
    Write-Host "[bench-run] ETL:    $etlFile" -ForegroundColor Cyan
    if ($mergedEtl -and (Test-Path $mergedEtl)) {
        Write-Host "[bench-run] Merged: $mergedEtl (use with analyze-etl.py --stacks)" -ForegroundColor Cyan
    }
}

exit $proc.ExitCode
