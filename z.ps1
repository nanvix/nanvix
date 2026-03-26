# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

<#
.SYNOPSIS
    Utility for building and running Nanvix on Windows.

.DESCRIPTION
    Windows counterpart of the './z' bash script. Mirrors the same CLI interface.
    Builds the UserVM natively on Windows with the microvm or hyperlight backend,
    and builds guest components (kernel, hello-rust-nostd) via Docker.

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
$ZCacheFile = Join-Path $RootDir ".z.cache"
Set-Variable -Scope Script -Name DockerfileRelativePath -Value "scripts/setup/Dockerfile.build" -Option Constant

# ==================================================================================================
# Build Option Cache
# ==================================================================================================

function Write-ZCache {
    param([string[]]$Options)
    [System.IO.File]::WriteAllLines($ZCacheFile, $Options)
}

function Read-ZCache {
    if (-not (Test-Path $ZCacheFile)) {
        Write-Warn "No cached options found. Run a build first."
        return @()
    }
    $lines = @(Get-Content -Path $ZCacheFile -Encoding UTF8 | Where-Object { $_.Trim() -ne "" })
    if ($lines.Count -eq 0) {
        Write-Warn "Cache file is empty."
        return @()
    }
    # Extract only docker mode flags (stop at -- separator), matching Linux read_cache.
    $result = @()
    foreach ($line in $lines) {
        if ($line -eq "--") { break }
        switch ($line) {
            "--with-docker" { $result += $line }
            "--with-minimal-docker" { $result += $line }
        }
    }
    return $result
}

# Remove a directory junction (or plain directory) without following into the target.
function Remove-Junction {
    param([string]$Path)
    if (-not (Test-Path $Path)) { return }
    $item = Get-Item $Path -Force -ErrorAction SilentlyContinue
    if ($null -ne $item -and ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
        cmd /c "rmdir `"$Path`"" 2>$null
    } else {
        Remove-Item $Path -Recurse -Force -ErrorAction SilentlyContinue
    }
}

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
  .\z.ps1 test [-- TEST_PARAMETERS]
  .\z.ps1 clean
  .\z.ps1 distclean
  .\z.ps1 setup
  .\z.ps1 run [-- RUN_OPTIONS]
  .\z.ps1 help

Commands
  build       Builds Nanvix.
  test        Runs standalone integration tests on Windows.
  clean       Removes build artifacts (quick clean).
  distclean   Removes everything (full clean).
  setup       Sets up the development environment and installs Git hooks.
  run         Runs nanvixd in standalone mode.
  bench       Runs nanvix-bench benchmarks.
  help        Prints this help message.

Options
  --release               Build in release mode.
  --with-docker           Use the full Docker toolchain image.
  --with-minimal-docker   Use the minimal Docker toolchain image (default).
  --with-cached-options   Replay Docker build mode from the last successful build.

Build Targets (after --)
  all                     Build everything (guest + host).
  nanvix-bench            Build nanvix-bench only (native Windows).
  uservm                  Build UserVM only (native Windows).
  mkramfs                 Build mkramfs only (native Windows).
  guest                   Build guest components only (kernel + hello-rust-nostd).
  format-check            Check code formatting (native for host crates).
  lint-check              Check for linting issues (native for uservm).
  run-unit-tests          Run unit tests (native for uservm).
  format                  Fix code formatting (via Docker).
  lint                    Fix linting issues (via Docker).
  spellcheck              Check spelling (via Docker).
  spellcheck-fix          Fix spelling errors (via Docker).
  check                   Run cargo check on host crates (native, no Docker).
  check-uservm            Run cargo check on uservm only (native, no Docker).
  run-nanvix-tests        Run system integration tests (via Docker).
  test                    Run all tests (via Docker).
  verify                  Run Verus formal verification (via Docker).
  <any-make-target>       Any other target is forwarded to make via Docker.

Run Options (after --)
  -program <path>         Path to guest binary (default: bin/hello-rust-nostd.elf).

Build Parameters (after --)
  RELEASE=yes             Enable release mode.
  MACHINE=microvm         Target machine: microvm (default) or hyperlight.
  WHP=yes                 Enable WHP-specific guest kernel code for microvm builds.
  LOG_LEVEL=<level>       Log level (default: trace for debug, warn for release).

Test Parameters (after --)
  RELEASE=yes             Build nanvix-test in release mode if auto-build is needed.
  MACHINE=microvm         Build nanvix-test for microvm (default) or hyperlight.
  LOG_LEVEL=<level>       Set RUST_LOG for test execution.

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

function Get-SccacheArgs {
    $result = @()
    if ($env:SCCACHE_GHA_ENABLED) {
        $result += "--build-arg"
        $result += "SCCACHE_GHA_ENABLED=$env:SCCACHE_GHA_ENABLED"
    }
    if ($env:SCCACHE_GHA_CACHE_TO) {
        $result += "--build-arg"
        $result += "SCCACHE_GHA_CACHE_TO=$env:SCCACHE_GHA_CACHE_TO"
    }
    if ($env:SCCACHE_GHA_CACHE_FROM) {
        $result += "--build-arg"
        $result += "SCCACHE_GHA_CACHE_FROM=$env:SCCACHE_GHA_CACHE_FROM"
    }
    if ($env:ACTIONS_RESULTS_URL) {
        $result += "--secret"
        $result += "id=actions_results_url,env=ACTIONS_RESULTS_URL"
    }
    if ($env:ACTIONS_RUNTIME_TOKEN) {
        $result += "--secret"
        $result += "id=actions_runtime_token,env=ACTIONS_RUNTIME_TOKEN"
    }
    return $result
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

    $ghaSccacheArgs = Get-SccacheArgs

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
    $dockerArgs = @(
        "build",
        "--build-arg", "BASE_IMAGE=$imageName",
        "--build-arg", "BUILD_PARAMS=$BuildParams",
        "--build-arg", "SYSROOT_SUFFIX=$sysrootSuffix",
        "--build-arg", "WORKSPACE_PATH=/mnt",
        "--output", "type=local,dest=.",
        "--progress=plain",
        "-f", $dockerfilePath
    )
    if ($ghaSccacheArgs.Count -gt 0) {
        $dockerArgs += $ghaSccacheArgs
    }
    $dockerArgs += $RootDir

    $buildExitCode = 1
    try {
        & docker @dockerArgs
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

function Get-NativeCargoFeatures {
    param([string]$Machine = "microvm")
    $normalized = $Machine.Trim().ToLowerInvariant()
    if ($normalized -eq "hyperlight") {
        return "hyperlight"
    }
    elseif ($normalized -eq "microvm") {
        return "microvm,whp"
    }
    else {
        Write-Err "Unsupported machine type '$Machine'. Supported values are: microvm, hyperlight."
        exit 1
    }
}

function Build-UserVm {
    param([bool]$IsRelease, [string]$Machine = "microvm")

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $mode = if ($IsRelease) { "release" } else { "debug" }
    $buildProfile = if ($IsRelease) { "--release" } else { "" }
    $features = Get-NativeCargoFeatures -Machine $Machine

    Write-Info "Building UserVM ($Machine backend, $mode mode)..."

    if (-not (Test-Path $BinDir)) {
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    }

    $cmd = "cargo build --no-default-features --features `"$features`" -p uservm $buildProfile"
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

    # Prevent cleanup errors from aborting the build (consistent with other functions).
    $ErrorActionPreference = 'Continue'

    Write-Info "Building guest components (Docker)..."

    # Remove guest artifacts from previous Docker exports. Docker's
    # --output type=local is additive: it writes new files but never deletes
    # files that no longer exist in the output. Without this cleanup, stale
    # guest binaries (e.g., a removed .elf) persist across builds.
    # This cleanup is safe here because Build-Guest always does a full guest
    # rebuild (all-guest-staticlibs all-kernel all-guest-binaries). It must
    # NOT be placed in Invoke-DockerBuild, which is also called for partial
    # targets (e.g., kernel, format-check) that would not regenerate all files.
    foreach ($dir in @($BinDir, $LibDir)) {
        if (-not (Test-Path $dir)) { continue }
        Get-ChildItem -Path (Join-Path $dir '*') -File -Include "*.elf", "*.wasm", "*.a", "*.so", "*.img" -Recurse -ErrorAction SilentlyContinue |
            ForEach-Object { Remove-Item $_.FullName -Force -ErrorAction SilentlyContinue }
    }

    # Remove the sysroot directory matching the current build profile so that
    # stale sysroot artifacts do not survive across Docker exports.
    $sysrootSuffix = if ($IsRelease) { "release" } else { "debug" }
    $sysrootDir = Join-Path $RootDir "sysroot-$sysrootSuffix"
    if (Test-Path $sysrootDir) {
        Remove-Item $sysrootDir -Recurse -Force -ErrorAction SilentlyContinue
    }

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

    # Create sysroot junction after Docker build (parity with Linux ln -sfn).
    $sysrootTarget = Join-Path $RootDir "sysroot-$sysrootSuffix"
    if (Test-Path $sysrootTarget) {
        Remove-Junction -Path $SysrootLink
        New-Item -ItemType Junction -Path $SysrootLink -Target $sysrootTarget | Out-Null
        Write-Info "Sysroot junction: sysroot -> sysroot-$sysrootSuffix"
    }

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
    param([bool]$IsRelease, [string]$Machine = "microvm")

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $mode = if ($IsRelease) { "release" } else { "debug" }
    $buildProfile = if ($IsRelease) { "--release" } else { "" }
    $features = Get-NativeCargoFeatures -Machine $Machine

    Write-Info "Building nanvixd (standalone + $Machine, $mode mode)..."

    if (-not (Test-Path $BinDir)) {
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    }

    $cmd = "cargo build --no-default-features --features `"standalone,$features`" -p nanvixd $buildProfile"
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
    param([bool]$IsRelease, [string]$Machine = "microvm")

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $mode = if ($IsRelease) { "release" } else { "debug" }
    $buildProfile = if ($IsRelease) { "--release" } else { "" }
    $features = Get-NativeCargoFeatures -Machine $Machine

    New-StandaloneRootfsImage -IsRelease $IsRelease

    Write-Info "Building nanvix-test (standalone + $Machine, $mode mode)..."

    if (-not (Test-Path $BinDir)) {
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    }

    $cmd = "cargo build --no-default-features --features `"standalone,$features`" -p nanvix-test $buildProfile"
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

function Build-NanvixBench {
    param([bool]$IsRelease, [string]$Machine = "microvm", [string]$LogLevel = "")

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $mode = if ($IsRelease) { "release" } else { "debug" }
    $buildProfile = if ($IsRelease) { "--release" } else { "" }
    $features = Get-NativeCargoFeatures -Machine $Machine

    # Resolve LOG_LEVEL: explicit parameter > default based on release mode.
    # Release benchmarks require LOG_LEVEL=panic (enforced at runtime by nanvix-bench).
    if (-not $LogLevel) {
        $LogLevel = if ($IsRelease) { "panic" } else { "trace" }
    }

    Write-Info "Building nanvix-bench (standalone + $Machine, $mode mode)..."

    if (-not (Test-Path $BinDir)) {
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    }

    # Set compile-time environment variables that nanvix-bench checks via option_env!().
    $oldRelease = $env:RELEASE
    $oldLogLevel = $env:LOG_LEVEL
    try {
        $env:RELEASE = if ($IsRelease) { "yes" } else { "no" }
        $env:LOG_LEVEL = $LogLevel

        $cmd = "cargo build --no-default-features --features `"standalone,$features`" -p nanvix-bench $buildProfile"
        Write-Host "  $cmd" -ForegroundColor DarkGray
        Invoke-Expression $cmd
        if ($LASTEXITCODE -ne 0) {
            Write-Err "Failed to build nanvix-bench."
            exit 1
        }
    }
    finally {
        $env:RELEASE = $oldRelease
        $env:LOG_LEVEL = $oldLogLevel
    }

    $src = Join-Path (Join-Path (Join-Path $RootDir "target") $mode) "nanvix-bench.exe"
    $dst = Join-Path $BinDir "nanvix-bench.exe"
    if (Test-Path $src) {
        Copy-Item $src $dst -Force
        Write-Info "Output: $dst"
    }
    else {
        Write-Err "nanvix-bench binary not found at $src"
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
    cargo clean -p nanvix-bench 2>$null

    # Remove all guest binaries in bin/
    if (Test-Path $BinDir) {
        Get-ChildItem -Path (Join-Path $BinDir '*') -File -Include "*.elf", "*.wasm" -Recurse |
        Remove-Item -Force -ErrorAction SilentlyContinue
    }

    # Remove all guest libraries in lib/.
    if (Test-Path $LibDir) {
        Get-ChildItem -Path (Join-Path $LibDir '*') -File -Include "*.a", "*.so" -Recurse |
        Remove-Item -Force -ErrorAction SilentlyContinue
    }

    # Remove system image.
    if (Test-Path $SysImage) { Remove-Item $SysImage -Force }

    # Remove sysroot junction (without following into the target directory).
    Remove-Junction -Path $SysrootLink

    # Remove images directory.
    $imagesDir = Join-Path $RootDir "images"
    if (Test-Path $imagesDir) { Remove-Item $imagesDir -Recurse -Force }

    Write-Success "Quick clean complete."
}

function Invoke-DistClean {
    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    # Distclean depends on clean.
    Invoke-Clean

    Write-Info "Removing everything (full clean)..."

    # Clean all Rust build artifacts.
    $cargoAvailable = Get-Command cargo -ErrorAction SilentlyContinue
    if ($cargoAvailable) {
        cargo clean 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Warn "cargo clean failed with exit code $LASTEXITCODE."
        }
    }

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

    # Sysroot junction and images/ already removed by Invoke-Clean above.

    # Remove build option cache.
    if (Test-Path $ZCacheFile) { Remove-Item $ZCacheFile -Force }

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
# Host Binary Freshness
# ==================================================================================================

# During development, developers often run 'cargo build' directly for faster
# iteration. Cargo outputs to target/{debug,release}/ but z.ps1 run executes
# from bin/. This function detects when target/ has a newer host binary than
# bin/ and copies it, preventing stale artifact issues.
function Sync-HostBinaries {
    if (-not (Test-Path $BinDir)) { return }

    # Host binaries that are natively built on Windows and copied to bin/.
    $hostBinaries = @("nanvixd.exe", "uservm.exe")

    foreach ($binary in $hostBinaries) {
        $dst = Join-Path $BinDir $binary
        if (-not (Test-Path $dst)) { continue }
        $dstTime = (Get-Item $dst).LastWriteTime

        # Check both debug and release profiles for a newer build.
        foreach ($mode in @("debug", "release")) {
            $src = Join-Path $TargetDir (Join-Path $mode $binary)
            if (-not (Test-Path $src)) { continue }
            $srcTime = (Get-Item $src).LastWriteTime
            if ($srcTime -le $dstTime) { continue }

            $delta = [int]($srcTime - $dstTime).TotalSeconds
            Write-Warn ("Stale binary: bin\$binary is ${delta}s behind" +
                " target\${mode}\$binary. Syncing...")
            try {
                Copy-Item $src $dst -Force -ErrorAction Stop
                Write-Success "Updated bin\$binary from target\${mode}."
            }
            catch {
                Write-Err ("Failed to update bin\$binary from target\${mode}: " + $_.Exception.Message)
            }
            break
        }
    }
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

    # Sync host binaries from target/ to bin/ in case the developer built
    # directly with 'cargo build' instead of 'z.ps1 build'.
    Sync-HostBinaries

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

function Invoke-Test {
    param([bool]$IsRelease, [string[]]$TestArgs = @())

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $machine = "microvm"
    $logLevel = ""

    foreach ($arg in $TestArgs) {
        if ($arg -match '^MACHINE=') {
            $machine = ($arg -replace '^MACHINE=', '').Trim()
        }
        elseif ($arg -match '^LOG_LEVEL=') {
            $logLevel = ($arg -replace '^LOG_LEVEL=', '').Trim()
        }
        elseif ($arg -eq "RELEASE=yes") {
            continue
        }
        elseif ($arg -match '=') {
            Write-Warn "Ignoring unsupported test parameter: $arg"
        }
        else {
            Write-Warn "Ignoring unsupported test argument: $arg"
        }
    }

    # Validate machine type early using the same rules as native builds.
    Get-NativeCargoFeatures -Machine $machine | Out-Null

    $nanvixTestBin = Join-Path $BinDir "nanvix-test.exe"
    if (-not (Test-Path $nanvixTestBin)) {
        Write-Info "nanvix-test binary not found. Building it now..."
        Build-NanvixTest -IsRelease $IsRelease -Machine $machine
    }

    # Ensure the Windows standalone tests' daemon binary is available.
    $nanvixdBin = Join-Path $BinDir "nanvixd.exe"
    if (-not (Test-Path $nanvixdBin)) {
        Write-Err "Required daemon binary not found at $nanvixdBin."
        Write-Err "Build the UserVM and guest artifacts before running tests:"
        Write-Host "  .\z.ps1 build -- uservm" -ForegroundColor DarkGray
        Write-Host "  .\z.ps1 build -- guest" -ForegroundColor DarkGray
        exit 1
    }

    $testConfig = Join-Path $RootDir "test\test-standalone-windows.toml"
    if (-not (Test-Path $testConfig)) {
        Write-Err "Test configuration not found at $testConfig"
        exit 1
    }

    # Preserve existing RUST_LOG value (or absence) so we can restore it after the test run.
    $hadRustLog = Test-Path Env:RUST_LOG
    $priorRustLog = $null
    if ($hadRustLog) {
        $priorRustLog = $env:RUST_LOG
    }

    try {
        if (-not [string]::IsNullOrWhiteSpace($logLevel)) {
            $env:RUST_LOG = $logLevel
        }

        Write-Info "Running standalone integration tests on Windows..."
        Write-Host "  $nanvixTestBin $testConfig" -ForegroundColor DarkGray

        & $nanvixTestBin $testConfig
        if ($LASTEXITCODE -ne 0) {
            Write-Err "Tests failed with exit code $LASTEXITCODE."
            exit $LASTEXITCODE
        }

        Write-Success "Tests passed."
    }
    finally {
        if ($hadRustLog) {
            $env:RUST_LOG = $priorRustLog
        }
        else {
            Remove-Item Env:RUST_LOG -ErrorAction SilentlyContinue
        }
    }
}

# ==================================================================================================
# Bench
# ==================================================================================================

function Invoke-Bench {
    param([string[]]$BenchArgs = @())

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $benchBin = Join-Path $BinDir "nanvix-bench.exe"
    if (-not (Test-Path $benchBin)) {
        Write-Err "nanvix-bench binary not found at $benchBin. Build it first with: .\z.ps1 build -- nanvix-bench"
        exit 1
    }

    Write-Info "Running nanvix-bench..."
    Write-Host "  Args: $($BenchArgs -join ' ')" -ForegroundColor DarkGray

    & $benchBin @BenchArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Err "nanvix-bench exited with code $LASTEXITCODE."
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
    $knownCommands = @("build", "test", "clean", "distclean", "setup", "run", "bench", "help")
    if ($command -notin $knownCommands) {
        Write-Err "Unknown command: $command"
        Show-Help
        exit 1
    }

    # Parse options and positional arguments.
    $isRelease = $false
    $useMinimalDocker = $true
    $useCachedOptions = $false
    $dockerModeOptionSet = $false
    $buildParams = @()

    $pastSeparator = $false

    foreach ($arg in $remaining) {
        if ($arg -eq "--") { $pastSeparator = $true; continue }
        if (-not $pastSeparator -and $arg.StartsWith("--")) {
            switch ($arg) {
                "--release" { $isRelease = $true }
                "--with-docker" { $useMinimalDocker = $false; $dockerModeOptionSet = $true }
                "--with-minimal-docker" { $useMinimalDocker = $true; $dockerModeOptionSet = $true }
                "--with-cached-options" { $useCachedOptions = $true }
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

    # Apply cached options only when no docker mode flag was explicitly set on
    # the command line. This matches Linux z behavior: explicit flags always win.
    if ($useCachedOptions -and -not $dockerModeOptionSet) {
        $cached = Read-ZCache
        foreach ($opt in $cached) {
            switch ($opt) {
                "--with-docker" { $useMinimalDocker = $false }
                "--with-minimal-docker" { $useMinimalDocker = $true }
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

            # Extract MACHINE parameter for native builds.
            $machineParam = $makeParams | Where-Object { $_ -match '^MACHINE=' } | Select-Object -Last 1
            $machine = if ($machineParam) { $machineParam -replace '^MACHINE=', '' } else { 'microvm' }

            # Extract LOG_LEVEL parameter for native builds.
            $logLevelParam = $makeParams | Where-Object { $_ -match '^LOG_LEVEL=' } | Select-Object -Last 1
            $logLevel = if ($logLevelParam) { $logLevelParam -replace '^LOG_LEVEL=', '' } else { '' }

            foreach ($target in $targets) {
                switch ($target) {
                    "all" {
                        Build-Guest -IsRelease $isRelease -UseMinimal $useMinimalDocker -ExtraMakeParams $makeParams
                        Build-UserVm -IsRelease $isRelease -Machine $machine
                        Build-Nanvixd -IsRelease $isRelease -Machine $machine
                        Build-NanvixTest -IsRelease $isRelease -Machine $machine
                        Build-NanvixBench -IsRelease $isRelease -Machine $machine -LogLevel $logLevel
                    }
                    "uservm" {
                        Build-UserVm -IsRelease $isRelease -Machine $machine
                    }
                    "mkramfs" {
                        Build-Mkramfs -IsRelease $isRelease
                    }
                    "standalone-rootfs" {
                        New-StandaloneRootfsImage -IsRelease $isRelease
                    }
                    "nanvixd" {
                        Build-Nanvixd -IsRelease $isRelease -Machine $machine
                    }
                    "nanvix-test" {
                        Build-NanvixTest -IsRelease $isRelease -Machine $machine
                    }
                    "nanvix-bench" {
                        Build-NanvixBench -IsRelease $isRelease -Machine $machine -LogLevel $logLevel
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
                        $features = Get-NativeCargoFeatures -Machine $machine
                        Write-Info "Linting host crates (native, $machine backend)..."
                        $ErrorActionPreference = 'Continue'
                        cargo clippy --no-default-features --features "$features" -p uservm -- -D warnings
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
                        $features = Get-NativeCargoFeatures -Machine $machine
                        Write-Info "Running unit tests (native, $machine backend)..."
                        $ErrorActionPreference = 'Continue'
                        cargo test --no-default-features --features "$features" -p uservm
                        if ($LASTEXITCODE -ne 0) {
                            Write-Err "Unit tests failed for uservm."
                            exit 1
                        }
                        Write-Success "Unit tests passed."
                    }
                    "check-uservm" {
                        # Native cargo check for uservm only (no Docker required).
                        $features = Get-NativeCargoFeatures -Machine $machine
                        Write-Info "Checking uservm (native, $machine backend)..."
                        $ErrorActionPreference = 'Continue'

                        $fmtParam = $makeParams | Where-Object { $_ -match '^MESSAGE_FORMAT=' } | Select-Object -Last 1
                        $msgFmt = @()
                        if ($fmtParam) {
                            $fmt = ($fmtParam -replace '^MESSAGE_FORMAT=', '').Trim()
                            $allowedFormats = @('json', 'json-diagnostic-rendered-ansi')
                            if ([string]::IsNullOrWhiteSpace($fmt)) {
                                Write-Err "Invalid MESSAGE_FORMAT: value is empty. Allowed values: $($allowedFormats -join ', ')."
                                exit 1
                            }
                            if ($allowedFormats -notcontains $fmt) {
                                Write-Err "Invalid MESSAGE_FORMAT: '$fmt'. Allowed values: $($allowedFormats -join ', ')."
                                exit 1
                            }
                            $msgFmt = @("--message-format=$fmt")
                        }

                        cargo check --no-default-features --features "$features" -p uservm @msgFmt
                        if ($LASTEXITCODE -ne 0) {
                            Write-Err "Check failed for uservm."
                            exit 1
                        }

                        Write-Success "Check passed (uservm)."
                    }
                    "check" {
                        # Native cargo check for host crates (no Docker required).
                        # This avoids the infinite rebuild loop caused by Docker's
                        # symlink manipulation and file output when used with
                        # rust-analyzer. Pass MESSAGE_FORMAT=json for RA integration.
                        $features = Get-NativeCargoFeatures -Machine $machine
                        Write-Info "Checking host crates (native, $machine backend)..."
                        $ErrorActionPreference = 'Continue'

                        # Detect MESSAGE_FORMAT parameter (used by rust-analyzer).
                        $fmtParam = $makeParams | Where-Object { $_ -match '^MESSAGE_FORMAT=' } | Select-Object -Last 1
                        $msgFmt = @()
                        if ($fmtParam) {
                            $fmt = ($fmtParam -replace '^MESSAGE_FORMAT=', '').Trim()
                            $allowedFormats = @('json', 'json-diagnostic-rendered-ansi')
                            if ([string]::IsNullOrWhiteSpace($fmt)) {
                                Write-Err "Invalid MESSAGE_FORMAT: value is empty. Allowed values: $($allowedFormats -join ', ')."
                                exit 1
                            }
                            if ($allowedFormats -notcontains $fmt) {
                                Write-Err "Invalid MESSAGE_FORMAT: '$fmt'. Allowed values: $($allowedFormats -join ', ')."
                                exit 1
                            }
                            $msgFmt = @("--message-format=$fmt")
                        }

                        # Check uservm (machine features, no standalone).
                        cargo check --no-default-features --features "$features" -p uservm @msgFmt
                        if ($LASTEXITCODE -ne 0) {
                            Write-Err "Check failed for uservm."
                            exit 1
                        }

                        # Check mkramfs (no features).
                        cargo check -p mkramfs @msgFmt
                        if ($LASTEXITCODE -ne 0) {
                            Write-Err "Check failed for mkramfs."
                            exit 1
                        }

                        # Check nanvixd and nanvix-test (standalone + machine features).
                        cargo check --no-default-features --features "standalone,$features" -p nanvixd -p nanvix-test @msgFmt
                        if ($LASTEXITCODE -ne 0) {
                            Write-Err "Check failed for nanvixd/nanvix-test."
                            exit 1
                        }

                        Write-Success "Check passed."
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

            # Cache build options for --with-cached-options.
            # Write the full command line (matching Linux write_cache behavior).
            $cacheOpts = @("build")
            if ($isRelease) { $cacheOpts += "--release" }
            if ($useMinimalDocker) { $cacheOpts += "--with-minimal-docker" } else { $cacheOpts += "--with-docker" }
            $cacheOpts += "--"
            $cacheOpts += $buildParams
            try {
                Write-ZCache -Options $cacheOpts
            }
            catch {
                Write-Warn "Failed to write .z.cache: $($_.Exception.Message)"
            }
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

        "bench" {
            Invoke-Bench -BenchArgs $buildParams
        }

        "setup" {
            Invoke-Setup -UseMinimal $useMinimalDocker
        }

        "test" {
            Invoke-Test -IsRelease $isRelease -TestArgs $buildParams
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
