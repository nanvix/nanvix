---
name: build
description: Guide for building, formatting, linting, and spell-checking Nanvix with z. Use this when asked to build or validate the repository.
---

# Build Nanvix

Use this skill when the user asks to build, compile, format, lint, or spell-check Nanvix. This
covers all build-system operations exposed through the `z` utility.

## Prerequisites

- Development environment set up per `doc/setup.md`.
- A local cross-compilation toolchain (`toolchain/`).
- **Windows 11:** GNU Make on PATH, Windows Hypervisor Platform enabled,
  Developer Mode enabled, and Rust toolchain installed. See `doc/setup.md` for details.

## Windows Setup Summary

Before building on Windows, ensure:

1. **Windows Hypervisor Platform** is enabled (`Enable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -All`, then reboot).
2. **Developer Mode** is on (Settings > Privacy & Security > For developers).
3. **GNU Make** is on PATH (`winget install ezwinports.make`).
4. **Rust toolchain** is installed via `winget install Rustlang.Rustup`.
5. Repository was cloned with `git clone -c core.symlinks=true`.
6. Git hooks are installed: `./z.ps1 setup`.

## Building

### Preferred Build Commands (using `z` utility)

```bash
# Build everything with the local toolchain.
./z build -- all
```

### Building Individual Components

```bash
# Kernel only.
./z build -- kernel
# Nanvixd only.
./z build -- all-nanvixd
# UserVM only.
./z build -- all-uservm
```

### Build Parameters

Set these as environment variables or pass them after `--` in the `z` command:

| Parameter        | Values                   | Default         |
|------------------|--------------------------|-----------------|
| `MACHINE`        | `microvm`                | `microvm`       |
| `TARGET`         | `x86`                    | `x86`           |
| `RELEASE`        | `yes`, `no`              | `no`            |
| `LOG_LEVEL`      | `trace`, `debug`,        | `error`         |
|                  | `info`, `warn`,          |                 |
|                  | `error`, `panic`         |                 |
| `PROFILER`       | `yes`, `no`              | `no`            |
| `DEPLOYMENT_MODE`| `standalone`,            | `standalone`    |
|                  | `single-process`,        |                 |
|                  | `multi-process`, `l2`    |                 |

Example with custom parameters:

```bash
./z build -- all RELEASE=yes LOG_LEVEL=error
```

### Manual Build Variants

```bash
# Default debug build.
./z build -- all
# Release build.
./z build -- all RELEASE=yes LOG_LEVEL=panic
# For echo-breakdown benchmark.
./z build -- all RELEASE=yes LOG_LEVEL=panic TIMESTAMP_MSG=yes
```

### Building on Windows (using `z.ps1`)

On Windows 11, the `z.ps1` PowerShell script provides the same CLI interface. Guest components are
cross-compiled using a local toolchain; host binaries (`nanvixd`, `uservm`) are built natively with the
microvm backend (WHP on Windows).

```powershell
# Build everything (guest via make + host binaries natively).
.\z.ps1 build -- all

# Build only the UserVM (native Windows build).
.\z.ps1 build -- uservm

# Build only nanvixd (native Windows build).
.\z.ps1 build -- nanvixd

# Build only nanvix-bench (native Windows build).
.\z.ps1 build -- nanvix-bench

# Build only guest components (kernel + hello-rust-nostd).
.\z.ps1 build -- guest

# Release build.
.\z.ps1 build -- all RELEASE=yes
```

Any unrecognized target is forwarded to `make`, just like on Linux:

```powershell
.\z.ps1 build -- kernel
.\z.ps1 build -- format-check
.\z.ps1 build -- lint-check
```

## Code Quality

### Formatting

```bash
# Check formatting issues.
./z build -- format-check
# Auto-fix formatting issues.
./z build -- format
```

### Linting

```bash
# Check linting issues.
./z build -- lint-check
# Auto-fix linting issues.
./z build -- lint
```

### Spell Checking

```bash
# Check spelling errors.
./z build -- spellcheck
# Fix spelling errors.
./z build -- spellcheck-fix
```

## Formal Verification

Nanvix uses Verus for formal verification of selected kernel crates. The expected Verus version is pinned in `build/verus-version`. Verification requires `VERUS_EXECUTABLE_DIR` to be set; when unset, `make verify` is a no-op.

```bash
# Install verus and run verification.
./scripts/setup/verus.sh ~/toolchain/verus
./z build -- verify VERUS_EXECUTABLE_DIR=~/toolchain/verus

# Verify a single crate.
./z build -- verify-bitmap VERUS_EXECUTABLE_DIR=~/toolchain/verus
```

- Set `VERUS_EXECUTABLE_DIR` to the directory containing the `verus` binary.
- Use `scripts/setup/verus.sh <dir>` to download the pinned release.
- The `vstd` crate version in `Cargo.toml` is exact-pinned (`=`) to match the Verus binary.

## Cleaning

```bash
./z clean        # Clean build artifacts.
./z distclean    # Remove all generated files.
```

### Cleaning on Windows

```powershell
.\z.ps1 clean        # Quick clean (UserVM artifacts + cache).
.\z.ps1 distclean    # Full clean (cargo clean + all artifacts).
```

## CI/CD Pipeline

```bash
# Run the full CI pipeline locally.
./scripts/pipeline.sh
```

The pipeline covers: spell checking, formatting, linting, building, and testing across multiple
machine and deployment configurations.

## IDE Setup (Optional)

### Visual Studio Code

Use the host-specific settings template. The Linux template invokes `./z`, while the Windows
template routes Rust Analyzer through `./z.bat build -- check`, which runs native `cargo check`
on host crates (`uservm`, `nanvixd`, `nanvix-test`, `mkramfs`).

**Linux:**

```bash
mkdir -p .vscode && cd .vscode
ln -s ../scripts/setup/vscode/settings-linux.json settings.json
```

**Windows (PowerShell):**

```powershell
New-Item -ItemType Directory -Path .vscode -Force
Copy-Item scripts\setup\vscode\settings-windows.json .vscode\settings.json
```

> **Note:** The `check` target in `z.ps1` only checks host crates natively. Guest and kernel
> crates require the cross-compilation toolchain. Run
> `.\z.ps1 build -- check-kernel check-guest-binaries` for a full cross-target check.

## Troubleshooting Build Issues

- If builds fail with toolchain errors, verify `toolchain/` symlink points to a valid toolchain.
- Use `./z help` for usage information.
- **Windows:** Use `.\z.ps1 help` for Windows-specific usage information.
- **Windows:** If the UserVM build fails, verify that the Windows Hypervisor Platform feature is
  enabled (`Get-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform`).
