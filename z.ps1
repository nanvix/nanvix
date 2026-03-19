# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

<#
.SYNOPSIS
    Utility for building and running Nanvix on Windows.

.DESCRIPTION
    Windows counterpart of the './z' bash script. Mirrors the same CLI interface.
    Builds the UserVM natively on Windows with the microvm backend, and builds guest
    components (kernel, hello-rust-nostd) via Docker.

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

$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $RootDir) { $RootDir = Get-Location }
$BinDir = Join-Path $RootDir "bin"

# ==================================================================================================
# Logging
# ==================================================================================================

function Write-Info { param([string]$Msg) Write-Host "[INFO] $Msg" -ForegroundColor Cyan }
function Write-Success { param([string]$Msg) Write-Host "[OK]   $Msg" -ForegroundColor Green }
function Write-Warn { param([string]$Msg) Write-Host "[WARN] $Msg" -ForegroundColor Yellow }
function Write-Err { param([string]$Msg) Write-Host "[ERROR] $Msg" -ForegroundColor Red }

# ==================================================================================================
# Help
# ==================================================================================================

function Show-Help {
    Write-Host @"

Utility for building Nanvix on Windows.

Usage:
  .\z.ps1 COMMAND [OPTIONS] [-- BUILD_TARGET BUILD_PARAMETERS]
  .\z.ps1 build [-- BUILD_TARGET BUILD_PARAMETERS]
  .\z.ps1 clean
  .\z.ps1 distclean
  .\z.ps1 setup
  .\z.ps1 run [-- RUN_OPTIONS]
  .\z.ps1 help

Commands
  build       Builds Nanvix.
  clean       Removes build artifacts (quick clean).
  distclean   Removes everything (full clean).
  setup       Sets up the Nanvix development environment (pulls Docker image).
  run         Runs UserVM in standalone mode.
  help        Prints this help message.

Options
  --release               Build in release mode.
  --with-docker           Use the full Docker toolchain image.
  --with-minimal-docker   Use the minimal Docker toolchain image (default).

Build Targets (after --)
  all                     Build everything (guest + uservm).
  uservm                  Build UserVM only (native Windows, microvm backend).
  guest                   Build guest components only (kernel + hello-rust-nostd).
  format-check            Check code formatting (via Docker).
  lint-check              Check for linting issues (via Docker).
  format                  Fix code formatting (via Docker).
  lint                    Fix linting issues (via Docker).
  spellcheck              Check spelling (via Docker).
  spellcheck-fix          Fix spelling errors (via Docker).
  check                   Run cargo check (via Docker).
  run-unit-tests          Run unit tests (via Docker).
  run-nanvix-tests        Run system integration tests (via Docker).
  test                    Run all tests (via Docker).
  verify                  Run Verus formal verification (via Docker).
  <any-make-target>       Any other target is forwarded to make via Docker.

Run Options (after --)
  -kernel <path>          Path to kernel binary (default: bin/kernel.elf).
  -initrd <path>          Path to guest binary (default: bin/hello-rust-nostd.elf).

Build Parameters (after --)
  RELEASE=yes             Enable release mode.
  MACHINE=microvm         Target machine (default: microvm).
  LOG_LEVEL=warn          Log level (default: warn).

Prerequisites
  - Docker Desktop for Windows (with Linux containers enabled).
  - Windows Hypervisor Platform enabled (for running the UserVM).
  - Rust toolchain on Windows (via rustup).

"@
}

# ==================================================================================================
# Symlink Helpers
# ==================================================================================================

# On Windows with core.symlinks=false, Git checks out symlinks as missing files (or small text files
# containing the target path). This function detects such entries and materializes them as copies of
# the target so that the Docker build context includes the correct file content.
function Restore-GitSymlinks {
    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    Write-Info "Restoring Git symlinks as file copies (Windows workaround)..."
    $restored = 0
    # List all git symlinks (mode 120000).
    $symlinkLines = git ls-files -s 2>$null | Where-Object { $_ -match '^120000' }
    foreach ($line in $symlinkLines) {
        # Format: "120000 <hash> <stage>\t<path>"
        $parts = $line -split '\t', 2
        if ($parts.Count -lt 2) { continue }
        $filePath = $parts[1].Trim()
        if (-not $filePath) { continue }

        # Extract the blob hash from the metadata portion.
        $metaParts = $parts[0] -split '\s+'
        if ($metaParts.Count -lt 2) { continue }
        $blobHash = $metaParts[1]

        $absPath = Join-Path $RootDir $filePath

        # Read the raw symlink target from the blob object.
        # For mode 120000, the blob content is the relative target path.
        # NOTE: Do NOT use 'git show HEAD:<path>' - it follows symlinks
        # and returns the resolved file content instead of the target path.
        $target = (git cat-file blob $blobHash 2>$null)
        if (-not $target) { continue }
        # cat-file may return an array of lines; join them first.
        if ($target -is [array]) { $target = $target -join "`n" }
        $target = $target.Trim()

        # Sanity check: symlink targets are short relative paths (e.g.
        # "../../shared/build.rs"). If the blob content looks like source
        # code (multi-line or very long), the entry is not a real symlink
        # or has already been resolved - skip it.
        if ($target.Contains("`n") -or $target.Length -gt 500) {
            continue
        }

        # Resolve the target relative to the symlink's directory.
        $fileDir = Split-Path $absPath -Parent
        $targetAbsPath = Join-Path $fileDir $target
        $targetAbsPath = [System.IO.Path]::GetFullPath($targetAbsPath)

        if (-not (Test-Path $targetAbsPath)) {
            Write-Warn "Symlink target not found: $filePath -> $target"
            continue
        }

        # Create parent directory if needed.
        $parentDir = Split-Path $absPath -Parent
        if (-not (Test-Path $parentDir)) {
            New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
        }

        # Copy the target content as the symlink file.
        if (Test-Path $targetAbsPath -PathType Container) {
            # Target is a directory; copy as directory.
            if (Test-Path $absPath) { Remove-Item $absPath -Recurse -Force }
            Copy-Item -Path $targetAbsPath -Destination $absPath -Recurse -Force
        }
        else {
            Copy-Item -Path $targetAbsPath -Destination $absPath -Force
        }
        $restored++
    }
    if ($restored -gt 0) {
        Write-Info "Restored $restored symlink(s) as file copies."
    }
    else {
        Write-Info "No symlinks needed restoring."
    }
}

# Removes the materialized symlink copies and restores the original Git symlink
# text files so they don't pollute the working tree.
function Remove-RestoredSymlinks {
    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $symlinkPaths = @()
    $symlinkLines = git ls-files -s 2>$null | Where-Object { $_ -match '^120000' }
    foreach ($line in $symlinkLines) {
        $parts = $line -split '\t', 2
        if ($parts.Count -lt 2) { continue }
        $filePath = $parts[1].Trim()
        if (-not $filePath) { continue }
        $absPath = Join-Path $RootDir $filePath
        if (Test-Path $absPath) {
            Remove-Item $absPath -Recurse -Force -ErrorAction SilentlyContinue
        }
        $symlinkPaths += $filePath
    }

    # Restore the original symlink text files from Git so the working tree stays clean.
    if ($symlinkPaths.Count -gt 0) {
        foreach ($sp in $symlinkPaths) {
            git checkout -- $sp 2>$null
        }
    }
}

# ==================================================================================================
# Docker
# ==================================================================================================

function Assert-DockerAvailable {
    $dockerAvailable = Get-Command docker -ErrorAction SilentlyContinue
    if (-not $dockerAvailable) {
        Write-Err "Docker is not available. Install Docker Desktop for Windows."
        exit 1
    }
}

function Assert-DockerImageAvailable {
    param([string]$ImageName)
    # Temporarily suppress errors from native commands so that Docker stderr
    # does not trigger a terminating error under $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'SilentlyContinue'
    docker image inspect $ImageName >$null 2>&1
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    if ($exitCode -ne 0) {
        Write-Err "Docker image '$ImageName' not found locally. Run '.\z setup' to pull it."
        exit 1
    }
}

function Get-DockerImageName {
    param([bool]$UseMinimal = $true)
    $cargoToml = Join-Path $RootDir "Cargo.toml"
    $versionLine = Select-String -Path $cargoToml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if ($versionLine -and $versionLine.Matches[0].Groups[1].Value) {
        $v = $versionLine.Matches[0].Groups[1].Value -split '\.'
        $imageTag = "v$($v[0]).$($v[1]).x"
    }
    else {
        $imageTag = "latest"
        Write-Warn "Could not parse version from Cargo.toml, using '$imageTag'."
    }
    $suffix = if ($UseMinimal) { "-minimal" } else { "" }
    return "nanvix/toolchain:$imageTag$suffix"
}

function Invoke-DockerBuild {
    param([string]$BuildParams, [bool]$IsRelease = $false, [bool]$UseMinimal = $true)

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    Assert-DockerAvailable

    $imageName = Get-DockerImageName -UseMinimal $UseMinimal

    # Validate that the Docker image exists locally before attempting the build.
    Assert-DockerImageAvailable -ImageName $imageName

    # Restore Git symlinks as file copies so the Docker build context is complete.
    Restore-GitSymlinks

    Write-Host "  Image: $imageName" -ForegroundColor DarkGray

    $sysrootSuffix = if ($IsRelease) { "release" } else { "debug" }

    $dockerfilePath = Join-Path $RootDir "scripts/setup/Dockerfile.build"
    if (-not (Test-Path $dockerfilePath)) {
        Remove-RestoredSymlinks
        Write-Err "Build Dockerfile not found at $dockerfilePath"
        exit 1
    }

    $env:DOCKER_BUILDKIT = "1"
    Write-Host "  docker build (params: $BuildParams)" -ForegroundColor DarkGray

    # Remove the local .venv directory if it exists. Docker builds may create a
    # .venv inside the container with a lib64 -> lib symlink (standard Linux Python
    # venv). If a previous Docker export left a broken reparse point at .venv\lib64
    # on Windows, the output exporter fails with "The file cannot be accessed by
    # the system." Removing it beforehand prevents this.
    $localVenv = Join-Path $RootDir ".venv"
    if (Test-Path $localVenv) {
        # Use cmd to remove in case of broken reparse points that PowerShell can't handle.
        cmd /c "rmdir /s /q `"$localVenv`"" 2>$null
        if (Test-Path $localVenv) {
            Remove-Item $localVenv -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    docker build `
        --build-arg "BASE_IMAGE=$imageName" `
        --build-arg "BUILD_PARAMS=$BuildParams" `
        --build-arg "SYSROOT_SUFFIX=$sysrootSuffix" `
        --build-arg "WORKSPACE_PATH=/mnt" `
        --output "type=local,dest=." `
        --progress=plain `
        -f $dockerfilePath `
        $RootDir

    $buildExitCode = $LASTEXITCODE

    # Clean up materialized symlink copies.
    Remove-RestoredSymlinks

    if ($buildExitCode -ne 0) {
        Write-Err "Docker build failed (params: $BuildParams)."
        exit 1
    }
}

# ==================================================================================================
# Build Functions
# ==================================================================================================

function Build-UserVm {
    param([bool]$IsRelease)

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $mode = if ($IsRelease) { "release" } else { "debug" }
    $profile = if ($IsRelease) { "--release" } else { "" }

    Write-Info "Building UserVM (microvm backend, $mode mode)..."

    if (-not (Test-Path $BinDir)) {
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    }

    $cmd = "cargo build --no-default-features --features `"microvm`" -p uservm $profile"
    Write-Host "  $cmd" -ForegroundColor DarkGray
    Invoke-Expression $cmd
    if ($LASTEXITCODE -ne 0) {
        Write-Err "Failed to build UserVM."
        exit 1
    }

    $src = Join-Path (Join-Path (Join-Path $RootDir "target") $mode) "uservm.exe"
    $dst = Join-Path $BinDir "uservm.exe"
    if (Test-Path $src) {
        Copy-Item $src $dst -Force
        Write-Info "Output: $dst"
    }
    else {
        Write-Err "UserVM binary not found at $src"
        exit 1
    }
}

function Build-Guest {
    param([bool]$IsRelease, [bool]$UseMinimal = $true, [string[]]$ExtraMakeParams = @())

    Write-Info "Building guest components (Docker)..."
    $releaseFlag = if ($IsRelease) { "RELEASE=yes" } else { "" }
    $buildParams = "all-guest-staticlibs all-kernel all-guest-binaries $releaseFlag MACHINE=microvm DEPLOYMENT_MODE=standalone"
    if ($ExtraMakeParams.Count -gt 0) {
        $buildParams += " " + ($ExtraMakeParams -join ' ')
    }
    Invoke-DockerBuild -BuildParams $buildParams -IsRelease $IsRelease -UseMinimal $UseMinimal
    Write-Info "Guest components built successfully."
}

# ==================================================================================================
# Clean
# ==================================================================================================

function Invoke-Clean {
    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    Write-Info "Removing build artifacts (quick clean)..."
    cargo clean -p uservm 2>$null
    cargo clean -p nanvixd 2>$null
    cargo clean -p nanvix-test 2>$null
    $uvmBin = Join-Path $BinDir "uservm.exe"
    if (Test-Path $uvmBin) { Remove-Item $uvmBin -Force }
    $nanvixdBin = Join-Path $BinDir "nanvixd.exe"
    if (Test-Path $nanvixdBin) { Remove-Item $nanvixdBin -Force }
    $nanvixTestBin = Join-Path $BinDir "nanvix-test.exe"
    if (Test-Path $nanvixTestBin) { Remove-Item $nanvixTestBin -Force }
    Write-Success "Quick clean complete."
}

function Invoke-DistClean {
    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    Write-Info "Removing everything (full clean)..."
    cargo clean 2>$null
    if (Test-Path $BinDir) {
        $uvmBin = Join-Path $BinDir "uservm.exe"
        if (Test-Path $uvmBin) { Remove-Item $uvmBin -Force }
        $nanvixdBin = Join-Path $BinDir "nanvixd.exe"
        if (Test-Path $nanvixdBin) { Remove-Item $nanvixdBin -Force }
    }
    Write-Success "Full cleanup complete."
}

# ==================================================================================================
# Run
# ==================================================================================================

function Invoke-Run {
    param([string[]]$RunArgs = @())

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    # Default values.
    $kernel = Join-Path $BinDir "kernel.elf"
    $initrd = Join-Path $BinDir "hello-rust-nostd.elf"

    # Parse run options.
    for ($i = 0; $i -lt $RunArgs.Count; $i++) {
        switch ($RunArgs[$i]) {
            "-kernel"  {
                if ($i + 1 -ge $RunArgs.Count) {
                    Write-Err "Missing value for -kernel. Usage: .\z.ps1 run -- -kernel <path> [-initrd <path>]"
                    exit 1
                }
                $i++
                $kernel = $RunArgs[$i]
            }
            "-initrd"  {
                if ($i + 1 -ge $RunArgs.Count) {
                    Write-Err "Missing value for -initrd. Usage: .\z.ps1 run -- [-kernel <path>] -initrd <path>"
                    exit 1
                }
                $i++
                $initrd = $RunArgs[$i]
            }
            default    { Write-Warn "Unknown run option: $($RunArgs[$i])" }
        }
    }

    $uvmBin = Join-Path $BinDir "uservm.exe"
    if (-not (Test-Path $uvmBin)) {
        Write-Err "UserVM binary not found at $uvmBin. Build it first with: .\z.ps1 build -- uservm"
        exit 1
    }

    Write-Info "Running UserVM in standalone mode..."
    Write-Host "  Kernel: $kernel" -ForegroundColor DarkGray
    Write-Host "  Initrd: $initrd" -ForegroundColor DarkGray

    & $uvmBin -kernel $kernel -initrd $initrd -standalone
    if ($LASTEXITCODE -ne 0) {
        Write-Err "UserVM exited with code $LASTEXITCODE."
        exit $LASTEXITCODE
    }
}

# ==================================================================================================
# Setup
# ==================================================================================================

function Invoke-Setup {
    param([bool]$UseMinimal = $true)

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    Write-Info "Setting up Nanvix development environment..."

    Assert-DockerAvailable

    $imageName = Get-DockerImageName -UseMinimal $UseMinimal
    Write-Info "Pulling Docker image: $imageName"
    docker pull $imageName

    if ($LASTEXITCODE -ne 0) {
        Write-Err "Failed to pull Docker image '$imageName'."
        exit 1
    }

    Write-Success "Docker image '$imageName' downloaded successfully."
    Write-Success "Setup complete."
}

# ==================================================================================================
# Main
# ==================================================================================================

function Main {
    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    if ($args.Count -eq 0) {
        Write-Err "No command provided."
        Show-Help
        exit 1
    }

    $command = $args[0]
    $remaining = @()
    if ($args.Count -gt 1) { $remaining = $args[1..($args.Count - 1)] }

    # If the first argument looks like a build target (not a known command),
    # treat it as an implicit "build <target>" for convenience.
    $knownCommands = @("build", "clean", "distclean", "setup", "run", "help")
    if ($command -notin $knownCommands) {
        $remaining = @($command) + $remaining
        $command = "build"
    }

    # Parse options and positional arguments.
    $isRelease = $false
    $useMinimalDocker = $true
    $buildParams = @()

    foreach ($arg in $remaining) {
        if ($arg -eq "--") { continue }
        if ($arg.StartsWith("--")) {
            switch ($arg) {
                "--release" { $isRelease = $true }
                "--with-docker" { $useMinimalDocker = $false }
                "--with-minimal-docker" { $useMinimalDocker = $true }
                default {
                    Write-Err "Unknown option: $arg"
                    exit 1
                }
            }
        }
        else {
            $buildParams += $arg
        }
    }

    # Check for RELEASE=yes in build parameters.
    foreach ($p in $buildParams) {
        if ($p -eq "RELEASE=yes") { $isRelease = $true }
    }

    switch ($command) {

        "build" {
            if ($buildParams.Count -eq 0) {
                Write-Err "No build target specified. Use: .\z.ps1 build <all|uservm|guest>"
                exit 1
            }

            Write-Info "Command:  build"
            Write-Info "Release:  $isRelease"
            Write-Info "Minimal:  $useMinimalDocker"
            Write-Info "Targets:  $($buildParams -join ' ')"
            Write-Host ""

            # Separate targets and make-style key=value parameters.
            $targets = @()
            $makeParams = @()
            foreach ($param in $buildParams) {
                if ($param -match '=') {
                    $makeParams += $param
                }
                else {
                    $targets += $param
                }
            }

            foreach ($target in $targets) {
                switch ($target) {
                    "all" {
                        Build-Guest -IsRelease $isRelease -UseMinimal $useMinimalDocker -ExtraMakeParams $makeParams
                        Build-UserVm -IsRelease $isRelease
                    }
                    "uservm" {
                        Build-UserVm -IsRelease $isRelease
                    }
                    "guest" {
                        Build-Guest -IsRelease $isRelease -UseMinimal $useMinimalDocker -ExtraMakeParams $makeParams
                    }
                    default {
                        # Forward any other target to Docker make (mirrors bash z behavior).
                        $dockerParams = @($target) + $makeParams
                        # Add default MACHINE if not specified by the user.
                        if (-not ($makeParams | Where-Object { $_ -match '^MACHINE=' })) {
                            $dockerParams += "MACHINE=microvm"
                        }
                        Write-Info "Forwarding '$target' to Docker..."
                        Invoke-DockerBuild -BuildParams ($dockerParams -join ' ') -IsRelease $isRelease -UseMinimal $useMinimalDocker
                    }
                }
            }

            Write-Host ""
            Write-Success "Build complete."
        }

        "clean" {
            Invoke-Clean
        }

        "distclean" {
            Invoke-DistClean
        }

        "run" {
            Invoke-Run -RunArgs $buildParams
        }

        "setup" {
            Invoke-Setup -UseMinimal $useMinimalDocker
        }

        "help" {
            Show-Help
        }

        default {
            Write-Err "Unknown command: '$command'"
            Show-Help
            exit 1
        }
    }
}

Main @args
