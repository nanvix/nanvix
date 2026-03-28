# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

<#
.SYNOPSIS
    Utility for building and running Nanvix on Windows.

.DESCRIPTION
    Windows counterpart of the './z' bash script. Mirrors the same CLI interface.
    Builds the UserVM natively on Windows with the microvm or hyperlight backend,
    and builds guest components using a local cross-compilation toolchain.

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

# Remove a directory junction (or plain directory) without following into the target.
function Remove-Junction {
    param([string]$Path)
    if (-not (Test-Path $Path)) { return }
    $item = Get-Item $Path -Force -ErrorAction SilentlyContinue
    if ($null -ne $item -and ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
        cmd /c "rmdir `"$Path`"" 2>$null
    }
    else {
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
  --release                Build in release mode.

Build Targets (after --)
  all                     Build everything (guest + host).
  nanvix-bench            Build nanvix-bench only (native Windows).
  uservm                  Build UserVM only (native Windows).
  mkramfs                 Build mkramfs only (native Windows).
  guest                   Build guest components only (kernel + hello-rust-nostd).
  format-check            Check code formatting (native for host crates).
  lint-check              Check for linting issues (native for uservm).
  run-unit-tests          Run unit tests (native for uservm).
  format                  Fix code formatting.
  lint                    Fix linting issues.
  spellcheck              Check spelling.
  spellcheck-fix          Fix spelling errors.
  check                   Run cargo check on host crates (native).
  check-uservm            Run cargo check on uservm only (native).
  run-nanvix-tests        Run system integration tests.
  test                    Run all tests.
  verify                  Run Verus formal verification.
  <any-make-target>       Any other target is forwarded to make.

Run Options (after --)
  -program <path>         Path to guest binary (default: bin/hello-rust-nostd.elf).

Build Parameters (after --)
  RELEASE=yes             Enable release mode.
  MACHINE=microvm         Target machine: microvm (default) or hyperlight.
  WHP=yes                 Enable WHP-specific guest kernel code for microvm builds.
  LOG_LEVEL=<level>       Log level (default: trace for debug, error for release).

Test Parameters (after --)
  RELEASE=yes             Build nanvix-test in release mode if auto-build is needed.
  MACHINE=microvm         Build nanvix-test for microvm (default) or hyperlight.
  LOG_LEVEL=<level>       Set RUST_LOG for test execution.

Prerequisites
  - GNU Make on PATH (for cross-compiling guest components).
  - Windows Hypervisor Platform enabled (for running the UserVM).
  - Rust toolchain on Windows (via rustup).

"@
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
# Symlink Helpers
# ==================================================================================================

# On Windows without Developer Mode, Git cannot create native symlinks and
# checks them out as small text files containing the target path. This function
# detects such entries and copies the target content in-place so that Rust and
# Make can find the real files.
function Restore-GitSymlinks {
    $ErrorActionPreference = 'Continue'

    $restored = 0
    $symlinkLines = git ls-files -s 2>$null | Where-Object { $_ -match '^120000' }
    foreach ($line in $symlinkLines) {
        $parts = $line -split '\t', 2
        if ($parts.Count -lt 2) { continue }
        $filePath = $parts[1].Trim()
        if (-not $filePath) { continue }

        $absPath = Join-Path $RootDir $filePath

        # Skip if the entry is already a real symlink or directory junction.
        if (Test-Path $absPath) {
            $item = Get-Item $absPath -Force -ErrorAction SilentlyContinue
            if ($null -ne $item -and (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0)) {
                continue
            }
        }

        # Read the first line; if it looks like a short relative path, it is a
        # broken symlink text stub.
        $content = Get-Content $absPath -Raw -ErrorAction SilentlyContinue
        if (-not $content) { continue }
        $target = $content.Trim()
        if ($target.Contains("`n") -or $target.Length -gt 500) { continue }

        # Resolve the target relative to the symlink's parent directory.
        $fileDir = Split-Path $absPath -Parent
        $targetAbs = [System.IO.Path]::GetFullPath((Join-Path $fileDir $target))
        if (-not (Test-Path $targetAbs)) { continue }

        # Replace the text stub with a copy of the target.
        if (Test-Path $targetAbs -PathType Container) {
            if (Test-Path $absPath) { Remove-Item $absPath -Recurse -Force }
            Copy-Item -Path $targetAbs -Destination $absPath -Recurse -Force
        }
        else {
            Copy-Item -Path $targetAbs -Destination $absPath -Force
        }
        $restored++
    }
    if ($restored -gt 0) {
        Write-Info "Restored $restored symlink(s) as file copies (Developer Mode not enabled)."
    }
}

# ==================================================================================================
# Make
# ==================================================================================================

# Creates an ld.exe shim from rust-lld so that the kernel and guest linker
# ("linker": "ld" in target specs) works on Windows. rust-lld auto-detects
# GNU ld flavor when its executable name is 'ld' or 'ld.lld'.
function Ensure-LdShim {
    $shimDir = Join-Path $RootDir ".z.shims"
    $shimExe = Join-Path $shimDir "ld.exe"

    # Check whether an existing ld.exe on PATH supports ELF linking.
    # Some Windows environments (e.g., CI runners) ship an ld.exe that
    # only supports PE formats (i386pep, i386pe). We need one that
    # supports ELF cross-linking for guest binaries. rust-lld (LLD)
    # supports both ELF and PE, so we check for "LLD" in the version
    # string to detect a capable linker.
    $existingLd = Get-Command ld.exe -ErrorAction SilentlyContinue
    if ($existingLd) {
        $ldVersion = & $existingLd.Source -V 2>&1 | Out-String
        if ($ldVersion -match 'LLD') {
            # Existing ld is LLD-based (supports ELF) — no shim needed.
            return
        }
        # Existing ld does not support ELF; fall through to create the shim
        # and prepend it to PATH so it takes precedence.
    }

    # Only recreate if missing.
    if (Test-Path $shimExe) {
        if ($env:Path -notlike "*$shimDir*") {
            $env:Path = "$shimDir;$env:Path"
        }
        return
    }

    # Locate rust-lld inside the active Rust toolchain.
    $rustcPath = Get-Command rustc -ErrorAction SilentlyContinue
    if (-not $rustcPath) {
        Write-Err "rustc not found. Install the Rust toolchain."
        exit 1
    }
    $sysroot = (& rustc --print sysroot 2>$null)
    if (-not $sysroot -or -not (Test-Path $sysroot)) {
        Write-Err "Could not determine Rust sysroot."
        exit 1
    }
    # Derive the active host triple from rustc -vV instead of hardcoding MSVC.
    $rustcVersionInfo = (& rustc -vV 2>$null)
    if (-not $rustcVersionInfo) {
        Write-Err "Could not query rustc version info to determine host triple."
        exit 1
    }
    $hostTriple = $null
    foreach ($line in ($rustcVersionInfo -split "`n")) {
        if ($line -like "host:*") {
            $hostTriple = $line.Split(":", 2)[1].Trim()
            break
        }
    }
    if (-not $hostTriple) {
        Write-Err "Could not determine Rust host triple from 'rustc -vV' output."
        exit 1
    }

    $lldPath = Join-Path $sysroot ("lib\rustlib\{0}\bin\rust-lld.exe" -f $hostTriple)
    if (-not (Test-Path $lldPath)) {
        Write-Err "rust-lld not found at $lldPath. Cannot cross-link guest ELF binaries."
        exit 1
    }

    # Copy rust-lld.exe as ld.exe and ld.lld.exe. rust-lld detects GNU linker
    # flavor from its executable name (ld, ld.lld), so no wrapper is needed.
    # ld.exe is needed by cargo (target spec "linker": "ld").
    # ld.lld.exe is needed by clang -fuse-ld=lld for C cross-compilation.
    if (-not (Test-Path $shimDir)) {
        New-Item -ItemType Directory -Path $shimDir -Force | Out-Null
    }
    Copy-Item -Path $lldPath -Destination $shimExe -Force
    $shimLld = Join-Path $shimDir "ld.lld.exe"
    if (-not (Test-Path $shimLld)) {
        Copy-Item -Path $lldPath -Destination $shimLld -Force
    }

    if ($env:Path -notlike "*$shimDir*") {
        $env:Path = "$shimDir;$env:Path"
    }
}

function Invoke-Make {
    param([string[]]$MakeParams = @())

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $makeCmd = Get-Command make -ErrorAction SilentlyContinue
    if (-not $makeCmd) {
        # Auto-discover make from common winget install locations.
        $wingetMake = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\*make*\bin\make.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($wingetMake) {
            $env:Path = "$($wingetMake.DirectoryName);$env:Path"
            $makeCmd = Get-Command make -ErrorAction SilentlyContinue
        }
    }
    if (-not $makeCmd) {
        Write-Err "GNU Make is not available. Install it (winget install ezwinports.make) and restart your terminal."
        exit 1
    }

    # Auto-discover clang from the default LLVM install location.
    if (-not (Get-Command clang -ErrorAction SilentlyContinue)) {
        $llvmBin = "C:\Program Files\LLVM\bin"
        if (Test-Path "$llvmBin\clang.exe") {
            $env:Path = "$llvmBin;$env:Path"
        }
    }

    # The Makefile uses $(HOME)/.cargo/bin/cargo. On Windows, HOME is not set
    # by default so the path resolves to /.cargo/bin/cargo. Set it from
    # USERPROFILE with forward slashes so that MSYS sh does not interpret
    # backslashes as escape characters.
    if (-not $env:HOME) {
        $env:HOME = $env:USERPROFILE -replace '\\', '/'
    }

    # GNU Make needs a POSIX shell (sh) for recipe lines that use Unix syntax
    # (rm, cp, mkdir -p, env-var prefixes). Git for Windows ships sh.exe and
    # the required coreutils in its usr/bin directory. Add it to PATH so that
    # make auto-detects sh.exe as its shell.
    $gitCmd = Get-Command git -ErrorAction SilentlyContinue
    if ($gitCmd) {
        $gitUsrBin = Join-Path (Split-Path (Split-Path $gitCmd.Source)) "usr\bin"
        if ((Test-Path $gitUsrBin) -and ($env:Path -notlike "*$gitUsrBin*")) {
            $env:Path = "$gitUsrBin;$env:Path"
        }
    }

    # Ensure Python Scripts directory (codespell, etc.) is on PATH for make.
    # Get-Command may return the WindowsApps stub; ask Python for its real
    # scripts directory instead.
    $pythonCmd = Get-Command python -ErrorAction SilentlyContinue
    if ($pythonCmd) {
        $pythonScripts = & python -c "import sysconfig; print(sysconfig.get_path('scripts'))" 2>$null
        if ($pythonScripts -and (Test-Path $pythonScripts) -and ($env:Path -notlike "*$pythonScripts*")) {
            $env:Path = "$pythonScripts;$env:Path"
        }
    }

    # Guest target specs use "linker": "ld" (GNU ld for ELF linking). On
    # Windows there is no system ld, but the Rust toolchain ships rust-lld
    # which is compatible with GNU ld. Create an ld.exe shim.
    Ensure-LdShim

    # If symlinks are checked out as text stubs (Developer Mode not enabled),
    # restore them as file copies so that cargo and make find real content.
    Restore-GitSymlinks

    Write-Host "  make $($MakeParams -join ' ')" -ForegroundColor DarkGray

    & make @MakeParams
    if ($LASTEXITCODE -ne 0) {
        Write-Err "make failed (params: $($MakeParams -join ' '))."
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

    # Windows only supports standalone deployment mode.
    if (-not ($resolvedParams | Where-Object { $_ -match '^DEPLOYMENT_MODE=' })) {
        $resolvedParams += "DEPLOYMENT_MODE=standalone"
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
    param([bool]$IsRelease, [string[]]$ExtraMakeParams = @())

    # Prevent cleanup errors from aborting the build (consistent with other functions).
    $ErrorActionPreference = 'Continue'

    Write-Info "Building guest components..."

    # Remove guest artifacts from previous builds. This cleanup ensures stale
    # guest binaries (e.g., a removed .elf) do not persist across builds.
    foreach ($dir in @($BinDir, $LibDir)) {
        if (-not (Test-Path $dir)) { continue }
        Get-ChildItem -Path (Join-Path $dir '*') -File -Include "*.elf", "*.wasm", "*.a", "*.so", "*.img" -Recurse -ErrorAction SilentlyContinue |
        ForEach-Object { Remove-Item $_.FullName -Force -ErrorAction SilentlyContinue }
    }

    # Remove the sysroot directory matching the current build profile so that
    # stale sysroot artifacts do not survive across builds.
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
    Invoke-Make -MakeParams $buildParams

    # Create sysroot junction after build (parity with Linux ln -sfn).
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
    param([bool]$IsRelease, [string]$Machine = "microvm", [bool]$IsProfile = $false)

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $mode = if ($IsRelease) { "release" } else { "debug" }
    $buildProfile = if ($IsRelease) { "--release" } else { "" }
    $features = Get-NativeCargoFeatures -Machine $Machine
    if ($IsProfile) { $features = "$features,profile-time" }

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
    param([bool]$IsRelease, [string]$Machine = "microvm", [string]$LogLevel = "", [bool]$IsProfile = $false)

    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    $mode = if ($IsRelease) { "release" } else { "debug" }
    $buildProfile = if ($IsRelease) { "--release" } else { "" }
    $features = Get-NativeCargoFeatures -Machine $Machine
    if ($IsProfile) { $features = "$features,profile-time" }

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

    # Remove .venv/.
    if (Test-Path $VenvDir) {
        Remove-Item $VenvDir -Recurse -Force -ErrorAction SilentlyContinue
        if (Test-Path $VenvDir) {
            # Fallback for broken reparse points from older Docker-based workflows.
            & cmd /c "rmdir /s /q `"$VenvDir`"" *> $null
        }
    }

    # Remove sysroot-debug/ and sysroot-release/ (SYSROOT_DIR).
    foreach ($suffix in @("sysroot-debug", "sysroot-release")) {
        $sysrootDir = Join-Path $RootDir $suffix
        if (Test-Path $sysrootDir) { Remove-Item $sysrootDir -Recurse -Force }
    }

    # Sysroot junction and images/ already removed by Invoke-Clean above.

    # Remove linker shim directory.
    $shimDir = Join-Path $RootDir ".z.shims"
    if (Test-Path $shimDir) { Remove-Item $shimDir -Recurse -Force }

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

function Assert-WindowsVersion {
    $build = [System.Environment]::OSVersion.Version.Build
    if ($build -lt 22000) {
        Write-Warn "Windows 11 (build 22000+) is recommended. Current build: $build."
        Write-Warn "Some features (e.g., WHP-backed UserVM workflows) may not be available on this host."
        return
    }
    Write-Success "Windows 11 detected (build $build)."
}

function Assert-DeveloperMode {
    $regPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock"
    $val = Get-ItemProperty -Path $regPath -Name AllowDevelopmentWithoutDevLicense -ErrorAction SilentlyContinue
    if (-not $val -or $val.AllowDevelopmentWithoutDevLicense -ne 1) {
        Write-Warn "Developer Mode is not enabled. Symlinks will be restored as file copies."
        Write-Warn "  Enable it in Settings > Privacy & Security > For developers."
        return
    }
    Write-Success "Developer Mode is enabled."
}

function Assert-HypervisorEnabled {
    $cs = Get-CimInstance Win32_ComputerSystem
    if (-not $cs.HypervisorPresent) {
        Write-Warn "No hypervisor detected. The UserVM will not run without WHP."
        Write-Warn "  Enable it in an elevated PowerShell prompt and restart:"
        Write-Warn "  Enable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -All"
        return
    }
    Write-Success "Hypervisor is active."
}

function Install-GnuMake {
    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    if (Get-Command make -ErrorAction SilentlyContinue) {
        Write-Success "GNU Make is already installed."
        return
    }

    # Check winget package directory (portable zip — not on PATH by default).
    $wingetMake = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\*make*\bin\make.exe" `
        -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($wingetMake) {
        $env:Path = "$($wingetMake.DirectoryName);$env:Path"
        Write-Success "GNU Make found at $($wingetMake.FullName)."
        return
    }

    $wingetCmd = Get-Command winget -ErrorAction SilentlyContinue
    if (-not $wingetCmd) {
        Write-Err "winget is not available. Install GNU Make manually: https://sourceforge.net/projects/ezwinports/files/"
        exit 1
    }

    Write-Info "Installing GNU Make via winget..."
    winget install ezwinports.make --accept-source-agreements --accept-package-agreements
    if ($LASTEXITCODE -ne 0) {
        Write-Err "Failed to install GNU Make. Install it manually: winget install ezwinports.make"
        exit 1
    }

    # Refresh discovery after installation.
    $wingetMake = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\*make*\bin\make.exe" `
        -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($wingetMake) {
        $env:Path = "$($wingetMake.DirectoryName);$env:Path"
    }

    if (-not (Get-Command make -ErrorAction SilentlyContinue)) {
        Write-Warn "GNU Make installed but not on PATH. Restart your terminal and re-run setup."
        exit 1
    }
    Write-Success "GNU Make installed."
}

function Install-Llvm {
    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    if (Get-Command clang -ErrorAction SilentlyContinue) {
        Write-Success "LLVM/Clang is already installed."
        return
    }

    # Check default LLVM install location.
    $llvmBin = "C:\Program Files\LLVM\bin"
    if (Test-Path "$llvmBin\clang.exe") {
        $env:Path = "$llvmBin;$env:Path"
        Write-Success "LLVM/Clang found at $llvmBin."
        return
    }

    $wingetCmd = Get-Command winget -ErrorAction SilentlyContinue
    if (-not $wingetCmd) {
        Write-Err "winget is not available. Install LLVM manually: https://releases.llvm.org/"
        exit 1
    }

    Write-Info "Installing LLVM/Clang via winget..."
    winget install LLVM.LLVM --accept-source-agreements --accept-package-agreements --silent
    if ($LASTEXITCODE -ne 0) {
        Write-Err "Failed to install LLVM. Install it manually: winget install LLVM.LLVM"
        exit 1
    }

    # Refresh PATH after installation.
    if ((Test-Path $llvmBin) -and ($env:Path -notlike "*$llvmBin*")) {
        $env:Path = "$llvmBin;$env:Path"
    }

    if (-not (Get-Command clang -ErrorAction SilentlyContinue)) {
        Write-Warn "LLVM installed but not on PATH. Restart your terminal and re-run setup."
        exit 1
    }
    Write-Success "LLVM/Clang installed."
}

function Install-RustToolchain {
    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    if (Get-Command rustc -ErrorAction SilentlyContinue) {
        Write-Success "Rust toolchain is already installed."
        return
    }

    # Check if cargo bin exists but is not on PATH.
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path (Join-Path $cargoBin "rustc.exe")) {
        if ($env:Path -notlike "*$cargoBin*") {
            $env:Path = "$cargoBin;$env:Path"
        }
        Write-Success "Rust toolchain is already installed."
        return
    }

    $wingetCmd = Get-Command winget -ErrorAction SilentlyContinue
    if (-not $wingetCmd) {
        Write-Err "winget is not available. Install Rust manually: https://rustup.rs"
        exit 1
    }

    Write-Info "Installing Rust toolchain via winget..."
    winget install Rustlang.Rustup --accept-source-agreements --accept-package-agreements
    if ($LASTEXITCODE -ne 0) {
        Write-Err "Failed to install Rust. Install it manually: https://rustup.rs"
        exit 1
    }

    # Add cargo bin to PATH for the current session.
    if ((Test-Path $cargoBin) -and ($env:Path -notlike "*$cargoBin*")) {
        $env:Path = "$cargoBin;$env:Path"
    }

    if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
        Write-Warn "Rust installed but not on PATH. Restart your terminal and re-run setup."
        exit 1
    }
    Write-Success "Rust toolchain installed."
}

function Invoke-Setup {
    # Prevent native command stderr from triggering $ErrorActionPreference = "Stop".
    $ErrorActionPreference = 'Continue'

    Write-Info "Setting up Nanvix development environment..."

    # Step 1: Verify Windows version.
    Assert-WindowsVersion

    # Step 2: Verify Developer Mode.
    Assert-DeveloperMode

    # Step 3: Verify Windows Hypervisor Platform.
    Assert-HypervisorEnabled

    # Step 4: Install GNU Make if missing.
    Install-GnuMake

    # Step 5: Install LLVM/Clang if missing.
    Install-Llvm

    # Step 6: Install Rust toolchain if missing.
    Install-RustToolchain

    # Step 7: Configure Git hooks.
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
    $isProfile = $false
    $buildParams = @()

    $pastSeparator = $false

    for ($i = 0; $i -lt $remaining.Count; $i++) {
        $arg = $remaining[$i]
        if ($arg -eq "--") { $pastSeparator = $true; continue }
        if (-not $pastSeparator -and $arg.StartsWith("--")) {
            switch ($arg) {
                "--release" { $isRelease = $true }
                "--profile" { $isProfile = $true; $isRelease = $true }
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
                        Build-Guest -IsRelease $isRelease -ExtraMakeParams $makeParams
                        Build-UserVm -IsRelease $isRelease -Machine $machine
                        Build-Nanvixd -IsRelease $isRelease -Machine $machine -IsProfile $isProfile
                        Build-NanvixTest -IsRelease $isRelease -Machine $machine
                        Build-NanvixBench -IsRelease $isRelease -Machine $machine -LogLevel $logLevel -IsProfile $isProfile
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
                        Build-Nanvixd -IsRelease $isRelease -Machine $machine -IsProfile $isProfile
                    }
                    "nanvix-test" {
                        Build-NanvixTest -IsRelease $isRelease -Machine $machine
                    }
                    "nanvix-bench" {
                        Build-NanvixBench -IsRelease $isRelease -Machine $machine -LogLevel $logLevel -IsProfile $isProfile
                    }
                    "guest" {
                        Build-Guest -IsRelease $isRelease -ExtraMakeParams $makeParams
                    }
                    "format-check" {
                        # Format check for host crates.
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
                        # Unit tests for host crates.
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
                        # Cargo check for uservm only.
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
                        # Cargo check for host crates.
                        # Pass MESSAGE_FORMAT=json for Rust Analyzer integration.
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
                        $spellParams = Add-GuestMachineDefaults `
                            -MakeParams (@("spellcheck") + $makeParams)
                        Write-Info "Running spellcheck..."
                        Invoke-Make -MakeParams $spellParams
                    }
                    "test" {
                        # Build everything and run unit tests (Windows standalone).
                        Build-Guest -IsRelease $isRelease -ExtraMakeParams $makeParams
                        Build-UserVm -IsRelease $isRelease -Machine $machine
                        Build-Nanvixd -IsRelease $isRelease -Machine $machine -IsProfile $isProfile
                        Build-NanvixTest -IsRelease $isRelease -Machine $machine
                        Build-NanvixBench -IsRelease $isRelease -Machine $machine -LogLevel $logLevel -IsProfile $isProfile

                        # Run unit tests for host crates that compile on Windows.
                        $features = Get-NativeCargoFeatures -Machine $machine
                        Write-Info "Running unit tests (native, $machine backend)..."
                        $ErrorActionPreference = 'Continue'
                        cargo test --no-default-features --features "$features" -p uservm
                        if ($LASTEXITCODE -ne 0) {
                            Write-Err "Unit tests failed for uservm."
                            exit 1
                        }
                        Write-Success "Unit tests passed."

                        # Run guest rlib unit tests via make.
                        $guestTestParams = Add-GuestMachineDefaults -MakeParams (@("test-guest-rlibs") + $makeParams)
                        Write-Info "Running guest rlib tests..."
                        Invoke-Make -MakeParams $guestTestParams
                    }
                    "run-nanvix-tests" {
                        # Run integration tests using the Windows-specific config.
                        Invoke-Test -IsRelease $isRelease -TestArgs $makeParams
                    }
                    default {
                        # Forward any other target to make (mirrors bash z behavior).
                        $fwdParams = Add-GuestMachineDefaults -MakeParams (@($target) + $makeParams)
                        Write-Info "Forwarding '$target' to make..."
                        Invoke-Make -MakeParams $fwdParams
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

        "bench" {
            Invoke-Bench -BenchArgs $buildParams
        }

        "setup" {
            Invoke-Setup
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
