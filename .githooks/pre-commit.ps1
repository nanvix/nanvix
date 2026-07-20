# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Use Continue so that stderr output from native commands (e.g., rustfmt
# warnings, Make $(warning ...)) is not treated as a terminating error.
$ErrorActionPreference = "Continue"

$MachineTypes = @("microvm")

function Write-Info {
    param([string]$Msg)
    Write-Host "[INFO] $Msg" -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Msg)
    Write-Host "[OK]   $Msg" -ForegroundColor Green
}

function Write-Err {
    param([string]$Msg)
    Write-Host "[ERROR] $Msg" -ForegroundColor Red
}

function Get-ReleaseFlag {
    param([string]$BuildType)

    switch ($BuildType) {
        "debug" { return "RELEASE=no" }
        "release" { return "RELEASE=yes" }
        default {
            Write-Err "(pre-commit) Invalid build type: $BuildType"
            exit 1
        }
    }
}

function Invoke-Check {
    param(
        [string]$ZScript,
        [string]$Machine,
        [string]$ReleaseFlag,
        [string]$Target,
        [string]$TempFile,
        [string]$BuildType
    )

    Clear-Content -Path $TempFile

    $buildArgs = @(
        "build",
        "--",
        "MACHINE=$Machine",
        "LOG_LEVEL=trace",
        $ReleaseFlag,
        $Target
    )

    & $ZScript @buildArgs *> $TempFile
    if ($LASTEXITCODE -eq 0) {
        return
    }

    Get-Content -Path $TempFile
    switch ($Target) {
        "format-check" {
            Write-Err "(pre-commit) Format check failed for: $BuildType, $Machine."
        }
        "lint-check" {
            Write-Err "(pre-commit) Lint check failed for: $BuildType, $Machine."
        }
        "spellcheck" {
            Write-Err "(pre-commit) Spell check failed for: $BuildType, $Machine."
        }
        default {
            Write-Err "(pre-commit) Check '$Target' failed for: $BuildType, $Machine."
        }
    }

    exit 1
}

git rev-parse --is-inside-work-tree *> $null
if ($LASTEXITCODE -ne 0) {
    Write-Err "(pre-commit) This script must be run inside a Git repository."
    exit 1
}

$repoRootDir = git rev-parse --show-toplevel
$zScript = Join-Path $repoRootDir "z.ps1"
$tempFile = [System.IO.Path]::GetTempFileName()

try {
    $totalConfigs = 0
    $checkedConfigs = 0
    $buildType = "debug"
    $releaseFlag = Get-ReleaseFlag -BuildType $buildType

    Write-Info "(pre-commit) Running pre-commit checks for all CI configurations..."

    foreach ($machine in $MachineTypes) {
        $totalConfigs += 1
        $checkedConfigs += 1

        Write-Info "(pre-commit) Checking configuration: $buildType, $machine..."

        $checkArgs = @{
            ZScript     = $zScript
            Machine     = $machine
            ReleaseFlag = $releaseFlag
            TempFile    = $tempFile
            BuildType   = $buildType
        }

        Invoke-Check @checkArgs -Target "format-check"
        Invoke-Check @checkArgs -Target "lint-check"
        Invoke-Check @checkArgs -Target "spellcheck"

        Write-Success "(pre-commit) Configuration passed: $buildType, $machine."
    }

    Write-Success "(pre-commit) All checks passed successfully ($checkedConfigs/$totalConfigs configurations checked)."
}
finally {
    Remove-Item -Path $tempFile -ErrorAction SilentlyContinue
}
