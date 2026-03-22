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
$LibDir = Join-Path $RootDir "lib"
$TargetDir = Join-Path $RootDir "target"
$CargoLock = Join-Path $RootDir "Cargo.lock"
$VenvDir = Join-Path $RootDir ".venv"
$SysImage = Join-Path $RootDir "nanvix.img"
$SysrootLink = Join-Path $RootDir "sysroot"
Set-Variable -Scope Script -Name DockerfileRelativePath -Value "scripts/setup/Dockerfile.build" -Option Constant

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
  setup       Sets up the development environment and installs Git hooks.
  run         Runs nanvixd in standalone mode.
  help        Prints this help message.

Options
  --release               Build in release mode.
  --with-docker           Use the full Docker toolchain image.
  --with-minimal-docker   Use the minimal Docker toolchain image (default).

Build Targets (after --)
  all                     Build everything (guest + host).
  uservm                  Build UserVM only (native Windows, microvm backend).
  mkramfs                 Build mkramfs only (native Windows).
  guest                   Build guest components only (kernel + hello-rust-nostd).
  format-check            Check code formatting (native for host crates).
  lint-check              Check for linting issues (native for uservm).
  run-unit-tests          Run unit tests (native for uservm).
  format                  Fix code formatting (via Docker).
  lint                    Fix linting issues (via Docker).
  spellcheck              Check spelling (via Docker).
  spellcheck-fix          Fix spelling errors (via Docker).
  check                   Run cargo check (via Docker).
  run-nanvix-tests        Run system integration tests (via Docker).
  test                    Run all tests (via Docker).
  verify                  Run Verus formal verification (via Docker).
  <any-make-target>       Any other target is forwarded to make via Docker.

Run Options (after --)
  -program <path>         Path to guest binary (default: bin/hello-rust-nostd.elf).

Build Parameters (after --)
  RELEASE=yes             Enable release mode.
  MACHINE=microvm         Target machine (default: microvm).
  WHP=yes                 Enable WHP-specific guest kernel code for microvm builds.
  LOG_LEVEL=<level>       Log level (default: trace for debug, warn for release).

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

        # If Git already checked out a native symlink, leave it alone. Docker can
        # consume the real symlink directly and overwriting it races with tools
        # like rust-analyzer that may have the path open.
        if (Test-Path $absPath) {
            $item = Get-Item $absPath -Force -ErrorAction SilentlyContinue
            if ($null -ne $item -and (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0)) {
                continue
            }
        }

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
            $item = Get-Item $absPath -Force -ErrorAction SilentlyContinue
            if ($null -eq $item) {
                Write-Warn "Cannot read attributes for '$filePath'; skipping removal."
            }
            elseif (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -eq 0) {
                Remove-Item $absPath -Recurse -Force -ErrorAction SilentlyContinue
            }
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
# Git Hooks
# ==================================================================================================

function Install-GitHooks {
    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $gitCommand = Get-Command git -ErrorAction SilentlyContinue
    if (-not $gitCommand) {
        Write-Warn "Skipping Git hook installation because git was not found in PATH."
        return
    }

    git rev-parse --is-inside-work-tree *> $null
    if ($LASTEXITCODE -ne 0) {
        Write-Warn "Skipping Git hook installation outside a Git worktree."
        return
    }

    Write-Info "Installing Git hooks from .githooks..."
    git config --local core.hooksPath .githooks

    if ($LASTEXITCODE -ne 0) {
        Write-Err "Failed to configure core.hooksPath."
        exit 1
    }

    Write-Success "Git hooks configured to use .githooks."
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

    $dockerfilePath = Join-Path $RootDir $DockerfileRelativePath
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
    if (Test-Path $VenvDir) {
        # Use cmd to remove in case of broken reparse points that PowerShell can't handle.
        cmd /c "rmdir /s /q `"$VenvDir`"" 2>$null
        if (Test-Path $VenvDir) {
            Remove-Item $VenvDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    # Use the repository directory directly as the Docker build context.
    # The .dockerignore file handles exclusions (mirroring the old robocopy filter).
    # Restore-GitSymlinks (called above) already materialized symlinks in-place,
    # so the context is complete. Symlinks are restored after the build via the
    # finally block below.
    $buildExitCode = 1
    try {
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
    }
    finally {
        Remove-RestoredSymlinks
    }

    if ($buildExitCode -ne 0) {
        Write-Err "Docker build failed (params: $BuildParams)."
        exit 1
    }
}

function Add-GuestMachineDefaults {
    param([string[]]$MakeParams = @())

    $resolvedParams = @($MakeParams)

    if (-not ($resolvedParams | Where-Object { $_ -match '^MACHINE=' })) {
        $resolvedParams += "MACHINE=microvm"
    }

    $machineParam = $resolvedParams | Where-Object { $_ -match '^MACHINE=' } | Select-Object -Last 1
    $machine = if ($machineParam) { $machineParam -replace '^MACHINE=', '' } else { 'microvm' }

    if ($machine -eq 'microvm' -and -not ($resolvedParams | Where-Object { $_ -match '^WHP=' })) {
        $resolvedParams += "WHP=yes"
    }

    return , $resolvedParams
}

# ==================================================================================================
# Build Functions
# ==================================================================================================

function Build-UserVm {
    param([bool]$IsRelease)

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $mode = if ($IsRelease) { "release" } else { "debug" }
    $buildProfile = if ($IsRelease) { "--release" } else { "" }

    Write-Info "Building UserVM (microvm backend, $mode mode)..."

    if (-not (Test-Path $BinDir)) {
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    }

    $cmd = "cargo build --no-default-features --features `"microvm,whp`" -p uservm $buildProfile"
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
    $buildParams = @("all-guest-staticlibs", "all-kernel", "all-guest-binaries", "DEPLOYMENT_MODE=standalone")
    if ($releaseFlag) {
        $buildParams += $releaseFlag
    }
    if ($ExtraMakeParams.Count -gt 0) {
        $buildParams += $ExtraMakeParams
    }
    $buildParams = Add-GuestMachineDefaults -MakeParams $buildParams
    Invoke-DockerBuild -BuildParams ($buildParams -join ' ') -IsRelease $IsRelease -UseMinimal $UseMinimal
    Write-Info "Guest components built successfully."
}

function Build-Mkramfs {
    param([bool]$IsRelease)

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $mode = if ($IsRelease) { "release" } else { "debug" }
    $buildProfile = if ($IsRelease) { "--release" } else { "" }

    Write-Info "Building mkramfs (native, $mode mode)..."

    if (-not (Test-Path $BinDir)) {
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    }

    $cmd = "cargo build -p mkramfs $buildProfile"
    Write-Host "  $cmd" -ForegroundColor DarkGray
    Invoke-Expression $cmd
    if ($LASTEXITCODE -ne 0) {
        Write-Err "Failed to build mkramfs."
        exit 1
    }

    $src = Join-Path (Join-Path (Join-Path $RootDir "target") $mode) "mkramfs.exe"
    $dst = Join-Path $BinDir "mkramfs.exe"
    if (Test-Path $src) {
        Copy-Item $src $dst -Force
        Write-Info "Output: $dst"
    }
    else {
        Write-Err "mkramfs binary not found at $src"
        exit 1
    }
}

function New-StandaloneRootfsImage {
    param([bool]$IsRelease)

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $mode = if ($IsRelease) { "release" } else { "debug" }

    # Ensure mkramfs is built before generating the rootfs image.
    Build-Mkramfs -IsRelease $IsRelease

    $mkramfs = Join-Path (Join-Path (Join-Path $RootDir "target") $mode) "mkramfs.exe"

    $seedDir = Join-Path $BinDir "standalone-rootfs-seed"
    $seedLibDir = Join-Path $seedDir "lib"
    $seedSrcDir = Join-Path $seedDir "src"
    $outputImg = Join-Path $BinDir "standalone-rootfs.img"

    Write-Info "Generating standalone-rootfs.img..."

    # Create seed directory structure.
    New-Item -ItemType Directory -Path $seedLibDir -Force | Out-Null
    New-Item -ItemType Directory -Path $seedSrcDir -Force | Out-Null

    # Populate seed with README and shared libraries.
    Copy-Item (Join-Path $RootDir "README.md") $seedDir -Force

    $libmul = Join-Path $LibDir "libmul.so"
    if (Test-Path $libmul) {
        Copy-Item $libmul $seedLibDir -Force
    }

    $libmulPie = Join-Path $LibDir "libmul-pie.so"
    if (Test-Path $libmulPie) {
        Copy-Item $libmulPie $seedLibDir -Force
    }

    # Generate the FAT32 rootfs image.
    $cmd = "& `"$mkramfs`" -o `"$outputImg`" `"$seedDir`""
    Write-Host "  $cmd" -ForegroundColor DarkGray
    Invoke-Expression $cmd
    if ($LASTEXITCODE -ne 0) {
        Write-Err "Failed to generate standalone-rootfs.img."
        exit 1
    }

    Write-Info "Output: $outputImg"
}

function Build-Nanvixd {
    param([bool]$IsRelease)

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $mode = if ($IsRelease) { "release" } else { "debug" }
    $buildProfile = if ($IsRelease) { "--release" } else { "" }

    Write-Info "Building nanvixd (standalone + microvm, $mode mode)..."

    if (-not (Test-Path $BinDir)) {
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    }

    $cmd = "cargo build --no-default-features --features `"standalone,microvm,whp`" -p nanvixd $buildProfile"
    Write-Host "  $cmd" -ForegroundColor DarkGray
    Invoke-Expression $cmd
    if ($LASTEXITCODE -ne 0) {
        Write-Err "Failed to build nanvixd."
        exit 1
    }

    $src = Join-Path (Join-Path (Join-Path $RootDir "target") $mode) "nanvixd.exe"
    $dst = Join-Path $BinDir "nanvixd.exe"
    if (Test-Path $src) {
        Copy-Item $src $dst -Force
        Write-Info "Output: $dst"
    }
    else {
        Write-Err "nanvixd binary not found at $src"
        exit 1
    }
}

function Build-NanvixTest {
    param([bool]$IsRelease)

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $mode = if ($IsRelease) { "release" } else { "debug" }
    $buildProfile = if ($IsRelease) { "--release" } else { "" }

    New-StandaloneRootfsImage -IsRelease $IsRelease

    Write-Info "Building nanvix-test (standalone + microvm, $mode mode)..."

    if (-not (Test-Path $BinDir)) {
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    }

    $cmd = "cargo build --no-default-features --features `"standalone,microvm,whp`" -p nanvix-test $buildProfile"
    Write-Host "  $cmd" -ForegroundColor DarkGray
    Invoke-Expression $cmd
    if ($LASTEXITCODE -ne 0) {
        Write-Err "Failed to build nanvix-test."
        exit 1
    }

    $src = Join-Path (Join-Path (Join-Path $RootDir "target") $mode) "nanvix-test.exe"
    $dst = Join-Path $BinDir "nanvix-test.exe"
    if (Test-Path $src) {
        Copy-Item $src $dst -Force
        Write-Info "Output: $dst"
    }
    else {
        Write-Err "nanvix-test binary not found at $src"
        exit 1
    }
}

# ==================================================================================================
# Clean
# ==================================================================================================

function Invoke-Clean {
    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    Write-Info "Removing build artifacts (quick clean)..."

    # Clean native host packages.
    cargo clean -p uservm 2>$null
    cargo clean -p nanvixd 2>$null
    cargo clean -p nanvix-test 2>$null

    # Remove all guest binaries in bin/
    if (Test-Path $BinDir) {
        Get-ChildItem -Path $BinDir -File -Include "*.elf", "*.wasm" -Recurse |
        Remove-Item -Force -ErrorAction SilentlyContinue
    }

    # Remove all guest libraries in lib/.
    if (Test-Path $LibDir) {
        Get-ChildItem -Path $LibDir -File -Include "*.a", "*.so" -Recurse |
        Remove-Item -Force -ErrorAction SilentlyContinue
    }

    # Remove system image.
    if (Test-Path $SysImage) { Remove-Item $SysImage -Force }

    Write-Success "Quick clean complete."
}

function Invoke-DistClean {
    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    # Distclean depends on clean.
    Invoke-Clean

    Write-Info "Removing everything (full clean)..."

    # Remove Cargo.lock.
    if (Test-Path $CargoLock) { Remove-Item $CargoLock -Force }

    # Remove target/.
    if (Test-Path $TargetDir) { Remove-Item $TargetDir -Recurse -Force }

    # Remove lib/.
    if (Test-Path $LibDir) { Remove-Item $LibDir -Recurse -Force }

    # Remove bin/.
    if (Test-Path $BinDir) { Remove-Item $BinDir -Recurse -Force }

    # Remove .venv/. Use cmd rmdir first because Docker builds on Windows can
    # leave broken reparse points (e.g., lib64 -> lib) that PowerShell's
    # Remove-Item cannot handle.
    if (Test-Path $VenvDir) {
        cmd /c "rmdir /s /q `"$VenvDir`"" 2>$null
        if (Test-Path $VenvDir) {
            Remove-Item $VenvDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    # Remove sysroot-debug/ and sysroot-release/ (SYSROOT_DIR).
    foreach ($suffix in @("sysroot-debug", "sysroot-release")) {
        $sysrootDir = Join-Path $RootDir $suffix
        if (Test-Path $sysrootDir) { Remove-Item $sysrootDir -Recurse -Force }
    }

    # Remove sysroot symlink (SYSROOT_LINK).
    if (Test-Path $SysrootLink) { Remove-Item $SysrootLink -Force }

    # Clean up Docker build cache for Nanvix.
    $dockerAvailable = Get-Command docker -ErrorAction SilentlyContinue
    if ($dockerAvailable) {
        Write-Info "Pruning Docker build cache..."
        docker builder prune --force
        if ($LASTEXITCODE -ne 0) {
            Write-Warn "Docker build cache prune failed with exit code $LASTEXITCODE."
        }
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

    # Default value.
    $program = Join-Path $BinDir "hello-rust-nostd.elf"

    # Parse run options.
    for ($i = 0; $i -lt $RunArgs.Count; $i++) {
        switch ($RunArgs[$i]) {
            "-program" {
                if ($i + 1 -ge $RunArgs.Count) {
                    Write-Err "Missing value for -program. Usage: .\z.ps1 run -- -program <path>"
                    exit 1
                }
                $i++
                $program = $RunArgs[$i]
            }
            default { Write-Warn "Unknown run option: $($RunArgs[$i])" }
        }
    }

    $nanvixdBin = Join-Path $BinDir "nanvixd.exe"
    if (-not (Test-Path $nanvixdBin)) {
        Write-Err "nanvixd binary not found at $nanvixdBin. Build it first with: .\z.ps1 build -- nanvixd"
        exit 1
    }

    Write-Info "Running nanvixd in standalone mode..."
    Write-Host "  Program: $program" -ForegroundColor DarkGray

    & $nanvixdBin -- $program
    if ($LASTEXITCODE -ne 0) {
        Write-Err "nanvixd exited with code $LASTEXITCODE."
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
    Install-GitHooks
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

    # Reject unknown commands early to avoid running setup or build steps with invalid parameters.
    $knownCommands = @("build", "clean", "distclean", "setup", "run", "help")
    if ($command -notin $knownCommands) {
        Write-Err "Unknown command: $command"
        Show-Help
        exit 1
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
            if ($arg -ne '') {
                $buildParams += $arg
            }
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
                        Build-Nanvixd -IsRelease $isRelease
                        Build-NanvixTest -IsRelease $isRelease
                    }
                    "uservm" {
                        Build-UserVm -IsRelease $isRelease
                    }
                    "mkramfs" {
                        Build-Mkramfs -IsRelease $isRelease
                    }
                    "standalone-rootfs" {
                        New-StandaloneRootfsImage -IsRelease $isRelease
                    }
                    "nanvixd" {
                        Build-Nanvixd -IsRelease $isRelease
                    }
                    "nanvix-test" {
                        Build-NanvixTest -IsRelease $isRelease
                    }
                    "guest" {
                        Build-Guest -IsRelease $isRelease -UseMinimal $useMinimalDocker -ExtraMakeParams $makeParams
                    }
                    "format-check" {
                        # Native format check for host crates (no Docker required).
                        Write-Info "Checking code formatting (native)..."
                        $ErrorActionPreference = 'Continue'
                        cargo fmt -p uservm -p nanvixd -p mkramfs --check
                        if ($LASTEXITCODE -ne 0) {
                            Write-Err "Format check failed."
                            exit 1
                        }
                        Write-Success "Format check passed."
                    }
                    "lint-check" {
                        # Native lint check for host crates that compile on Windows.
                        Write-Info "Linting host crates (native)..."
                        $ErrorActionPreference = 'Continue'
                        cargo clippy --no-default-features --features "microvm,whp" -p uservm -- -D warnings
                        if ($LASTEXITCODE -ne 0) {
                            Write-Err "Lint check failed for uservm."
                            exit 1
                        }
                        cargo clippy -p mkramfs -- -D warnings
                        if ($LASTEXITCODE -ne 0) {
                            Write-Err "Lint check failed for mkramfs."
                            exit 1
                        }
                        Write-Success "Lint check passed."
                    }
                    "run-unit-tests" {
                        # Native unit tests for host crates (no Docker required).
                        Write-Info "Running unit tests (native)..."
                        $ErrorActionPreference = 'Continue'
                        cargo test --no-default-features --features "microvm,whp" -p uservm
                        if ($LASTEXITCODE -ne 0) {
                            Write-Err "Unit tests failed for uservm."
                            exit 1
                        }
                        Write-Success "Unit tests passed."
                    }
                    "spellcheck" {
                        # Spellcheck requires pyspelling which is only available inside
                        # Docker. When Docker is not available or not running, skip gracefully.
                        $ErrorActionPreference = 'Continue'
                        $dockerCmd = Get-Command docker -ErrorAction SilentlyContinue
                        $dockerRunning = $false
                        if ($dockerCmd) {
                            docker info >$null 2>&1
                            $dockerRunning = ($LASTEXITCODE -eq 0)
                        }
                        if (-not $dockerRunning) {
                            Write-Warn "Skipping spellcheck (Docker not available or not running). Run with Docker to enable."
                        }
                        else {
                            $dockerParams = Add-GuestMachineDefaults `
                                -MakeParams (@("spellcheck") + $makeParams)
                            Write-Info "Running spellcheck via Docker..."
                            Invoke-DockerBuild -BuildParams ($dockerParams -join ' ') -IsRelease $isRelease -UseMinimal $useMinimalDocker
                        }
                    }
                    default {
                        # Forward any other target to Docker make (mirrors bash z behavior).
                        $dockerParams = Add-GuestMachineDefaults -MakeParams (@($target) + $makeParams)
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
