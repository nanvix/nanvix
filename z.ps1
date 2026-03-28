# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

<#
.SYNOPSIS
    Thin wrapper that delegates to the unified Python build backend (z.py).

.DESCRIPTION
    All build logic lives in z.py. This script ensures Python is available
    and forwards all arguments to the Python backend.

.EXAMPLE
    .\z.ps1 help
    .\z.ps1 build -- all
    .\z.ps1 build -- uservm
    .\z.ps1 build -- guest
    .\z.ps1 build -- lint-check
    .\z.ps1 clean
    .\z.ps1 distclean
    .\z.ps1 run
    .\z.ps1 setup
#>

# ==================================================================================================
# Configuration
# ==================================================================================================

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $ScriptDir) { $ScriptDir = Get-Location }
$ZPy = Join-Path $ScriptDir "z.py"

# Find Python.
$Python = $null
foreach ($name in @("python", "python3")) {
    $Python = Get-Command $name -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($Python) { break }
}

if (-not $Python) {
    Write-Host "[ERROR] Python 3.10+ is required but was not found on PATH." -ForegroundColor Red
    Write-Host "        Install Python 3.10+ (e.g., via winget or the official installer), then re-run this command." -ForegroundColor Red
    exit 1
}

# Use Continue so that stderr output from native commands (e.g., rustfmt
# warnings) is not treated as a terminating error by PowerShell.
$ErrorActionPreference = "Continue"
& $Python.Source $ZPy @args
exit $LASTEXITCODE
