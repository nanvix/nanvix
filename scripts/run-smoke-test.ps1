# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

<#
.SYNOPSIS
    Windows smoke test for nanvixd: launch nanvixd.exe directly under WHP and
    verify the kernel boots correctly.

.DESCRIPTION
    Mirrors the WHP-probe pattern used by the upstream nanvix CI (see
    .github/workflows/ci.yml, "Check Windows Hypervisor Platform" step):
    spawn nanvixd.exe with Start-Process, wait up to a timeout, then either
    check the exit code (release mode) or scan the captured console output
    for the kernel's magic string (debug mode).

    Standalone mode uses WHP and does not require cloud-hypervisor, so this
    script does NOT depend on the run-nanvixd.sh helper.

.PARAMETER Image
    Path to the multibin system image (e.g. nanvix.img).

.PARAMETER MagicString
    Kernel magic string to look for in debug mode.

.PARAMETER ExpectedExitCode
    Exit code expected from nanvixd in release mode.

.PARAMETER Timeout
    Maximum number of seconds to wait for the smoke test to complete.

.PARAMETER Release
    When set, validate exit code instead of waiting for the magic string.
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Image,
    [string]$MagicString = "hello, world!",
    [int]$ExpectedExitCode = 4,
    [int]$Timeout = 120,
    [switch]$Release
)

$ErrorActionPreference = "Stop"

$Nanvixd = Join-Path -Path "." -ChildPath "bin\nanvixd.exe"
if (-not (Test-Path $Nanvixd)) {
    Write-Error "nanvixd.exe not found at $Nanvixd. Run 'z.ps1 build -- all' first."
    exit 1
}
if (-not (Test-Path $Image)) {
    Write-Error "Image not found: $Image"
    exit 1
}

$LogDir = "logs"
if (-not (Test-Path $LogDir)) { New-Item -ItemType Directory -Path $LogDir | Out-Null }
$ConsoleFile = Join-Path $LogDir "smoke-console.log"
$StdoutFile  = Join-Path $LogDir "smoke-stdout.log"
$StderrFile  = Join-Path $LogDir "smoke-stderr.log"
Remove-Item -Force -ErrorAction SilentlyContinue $ConsoleFile, $StdoutFile, $StderrFile

Write-Host "====================================================================="
Write-Host "NANVIXD          = $Nanvixd"
Write-Host "IMAGE            = $Image"
Write-Host "CONSOLE_FILE     = $ConsoleFile"
Write-Host "TIMEOUT          = $Timeout"
if ($Release) {
    Write-Host "MODE             = release (expected exit code=$ExpectedExitCode)"
} else {
    Write-Host "MODE             = debug (waiting for magic string '$MagicString')"
}
Write-Host "====================================================================="

# Spawn nanvixd as a child process. Mirrors the upstream CI WHP-probe pattern.
$proc = Start-Process -FilePath $Nanvixd `
    -ArgumentList @("-console-file", $ConsoleFile, "--", $Image) `
    -PassThru -NoNewWindow `
    -RedirectStandardOutput $StdoutFile `
    -RedirectStandardError $StderrFile

if (-not $Release) {
    # Debug mode: poll for the magic string in the captured output.
    $found = $false
    for ($elapsed = 0; $elapsed -lt $Timeout; $elapsed++) {
        $haystacks = @($ConsoleFile, $StdoutFile, $StderrFile) | Where-Object { Test-Path $_ }
        if ($haystacks -and (Select-String -Path $haystacks -SimpleMatch -Pattern $MagicString -Quiet)) {
            $found = $true
            break
        }
        if ($proc.HasExited) { break }
        Start-Sleep -Seconds 1
    }

    if (-not $proc.HasExited) {
        $proc | Stop-Process -Force -ErrorAction SilentlyContinue
        $proc | Wait-Process -Timeout 5 -ErrorAction SilentlyContinue | Out-Null
    }

    if (-not $found) {
        Write-Host "ERROR: Smoke test failed: magic string '$MagicString' not found within ${Timeout}s."
        Write-Host "--- $ConsoleFile ---"; Get-Content -ErrorAction SilentlyContinue $ConsoleFile | Out-Host
        Write-Host "--- $StdoutFile ---";  Get-Content -ErrorAction SilentlyContinue $StdoutFile  | Out-Host
        Write-Host "--- $StderrFile ---";  Get-Content -ErrorAction SilentlyContinue $StderrFile  | Out-Host
        exit 1
    }

    Write-Host "Smoke test passed (magic string found)."
    exit 0
}

# Release mode: wait up to the timeout, then check the exit code.
$proc | Wait-Process -Timeout $Timeout -ErrorAction SilentlyContinue | Out-Null
if (-not $proc.HasExited) {
    $proc | Stop-Process -Force -ErrorAction SilentlyContinue
    $proc | Wait-Process -Timeout 5 -ErrorAction SilentlyContinue | Out-Null
    Write-Host "ERROR: Smoke test failed: nanvixd did not exit within ${Timeout}s."
    exit 1
}

if ($proc.ExitCode -ne $ExpectedExitCode) {
    Write-Host "ERROR: Smoke test failed: expected exit code $ExpectedExitCode, got $($proc.ExitCode)."
    Write-Host "--- $ConsoleFile ---"; Get-Content -ErrorAction SilentlyContinue $ConsoleFile | Out-Host
    Write-Host "--- $StdoutFile ---";  Get-Content -ErrorAction SilentlyContinue $StdoutFile  | Out-Host
    Write-Host "--- $StderrFile ---";  Get-Content -ErrorAction SilentlyContinue $StderrFile  | Out-Host
    exit 1
}

Write-Host "Smoke test passed (exit code=$ExpectedExitCode)."
exit 0
