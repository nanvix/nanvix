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
    .\z.ps1 build -- verify
    .\z.ps1 clean
    .\z.ps1 distclean
    .\z.ps1 run
    .\z.ps1 setup
#>

# ==================================================================================================
# Configuration
# ==================================================================================================

# Minimum major and minor Python versions required to run z.py. This should be kept in sync with the
# minimum version specified in pyproject.toml.
$MinPythonMajor = 3
$MinPythonMinor = 10

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $ScriptDir) { $ScriptDir = Get-Location }
$ZPy = Join-Path $ScriptDir "z.py"

# Return $true when the executable at $Path reports Python >= $MinPythonMajor.$MinPythonMinor.
function Test-PythonMinVersion {
    param([string]$Path)
    try {
        $output = & $Path --version 2>&1
        if ($LASTEXITCODE -ne 0) { return $false }
        # Expected format: "Python 3.x.y"
        if ($output -match 'Python\s+(\d+)\.(\d+)') {
            $major = [int]$Matches[1]
            $minor = [int]$Matches[2]
            return ($major -gt $MinPythonMajor -or ($major -eq $MinPythonMajor -and $minor -ge $MinPythonMinor))
        }
        return $false
    } catch {
        return $false
    }
}

# Find Python — verify the candidate actually runs (filters out Windows Store stubs).
function Find-Python {
    # 1. Try names on PATH.
    foreach ($name in @("python", "python3")) {
        $cmd = Get-Command $name -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($cmd -and (Test-PythonMinVersion $cmd.Source)) {
            return $cmd.Source
        }
    }

    # 2. Fall back to common install locations (3.10+).
    $locations = @(
        "$env:LOCALAPPDATA\Programs\Python\Python31*\python.exe",
        "$env:LOCALAPPDATA\Programs\Python\Python32*\python.exe",
        "$env:ProgramFiles\Python31*\python.exe",
        "$env:ProgramFiles\Python32*\python.exe",
        "${env:ProgramFiles(x86)}\Python31*\python.exe",
        "${env:ProgramFiles(x86)}\Python32*\python.exe",
        "C:\Python31*\python.exe",
        "C:\Python32*\python.exe"
    )
    foreach ($pattern in $locations) {
        $found = Get-Item $pattern -ErrorAction SilentlyContinue |
                 Sort-Object FullName -Descending |
                 Select-Object -First 1
        if ($found -and (Test-PythonMinVersion $found.FullName)) {
            return $found.FullName
        }
    }

    return $null
}

$PythonExe = Find-Python
if (-not $PythonExe) {
    Write-Host "[ERROR] Python $MinPythonMajor.$MinPythonMinor+ is required but was not found." -ForegroundColor Red
    Write-Host "        Install Python $MinPythonMajor.$MinPythonMinor+ (e.g., via winget or the official installer)" -ForegroundColor Red
    Write-Host "        and make sure it is on PATH, then re-run this command." -ForegroundColor Red
    exit 1
}

# Use Continue so that stderr output from native commands (e.g., rustfmt
# warnings) is not treated as a terminating error by PowerShell.
$ErrorActionPreference = "Continue"
& $PythonExe $ZPy @args
exit $LASTEXITCODE
