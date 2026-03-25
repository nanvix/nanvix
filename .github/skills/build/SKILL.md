---
name: build
description: Guide for building, formatting, linting, and spell-checking Nanvix with z. Use this when asked to build or validate the repository.
---

# Build Nanvix

Use this skill when the user asks to build, compile, format, lint, or spell-check Nanvix. This
covers all build-system operations exposed through the `z` utility.

## Prerequisites

- Development environment set up per `doc/setup.md`.
- Either a local cross-compilation toolchain (`toolchain/`) or Docker installed.
- **Windows 11:** Docker Desktop (Linux containers), Windows Hypervisor Platform enabled,
  Developer Mode enabled, and Rust toolchain installed. See `doc/setup.md` for details.

## Windows Setup Summary

Before building on Windows, ensure:

1. **Windows Hypervisor Platform** is enabled (`Enable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -All`, then reboot).
2. **Developer Mode** is on (Settings > Privacy & Security > For developers).
3. **Docker Desktop** is installed and configured for Linux containers.
4. **Rust toolchain** is installed via `winget install Rustlang.Rustup`.
5. Repository was cloned with `git clone -c core.symlinks=true`.
6. Docker toolchain image is pulled: `./z.ps1 setup`.

## Building

### Preferred Build Commands (using `z` utility)

```bash
# Build everything with previously cached build options.
./z build --with-cached-options -- all

# Build everything with Docker.
./z build --with-docker -- all

# Build everything with the local toolchain.
./z build -- all
```

### Building Individual Components

```bash
# Kernel only.
./z build --with-cached-options -- kernel
# Nanvixd only.
./z build --with-cached-options -- all-nanvixd
# UserVM only.
./z build --with-cached-options -- all-uservm
```

### Build Parameters

Set these as environment variables or pass them after `--` in the `z` command:

| Parameter        | Values                   | Default         |
|------------------|--------------------------|-----------------|
| `MACHINE`        | `microvm`, `hyperlight`  | `microvm`       |
| `TARGET`         | `x86`                    | `x86`           |
| `RELEASE`        | `yes`, `no`              | `no`            |
| `LOG_LEVEL`      | `trace`, `debug`,        | `warn`          |
|                  | `info`, `warn`,          |                 |
|                  | `error`, `panic`         |                 |
| `PROFILER`       | `yes`, `no`              | `no`            |
| `DEPLOYMENT_MODE`| `standalone`,            | `multi-process` |
|                  | `single-process`,        |                 |
|                  | `multi-process`, `l2`    |                 |

Example with custom parameters:

```bash
./z build -- all MACHINE=hyperlight \
    RELEASE=yes LOG_LEVEL=error
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
cross-compiled inside Docker; host binaries (`nanvixd`, `uservm`) are built natively with the
microvm backend (WHP on Windows).

```powershell
# Build everything (guest via Docker + host binaries natively).
.\z.ps1 build -- all

# Build only the UserVM (native Windows build).
.\z.ps1 build -- uservm

# Build only nanvixd (native Windows build).
.\z.ps1 build -- nanvixd

# Build only nanvix-bench (native Windows build).
.\z.ps1 build -- nanvix-bench

# Build only guest components (kernel + hello-rust-nostd, via Docker).
.\z.ps1 build -- guest

# Release build.
.\z.ps1 build -- all RELEASE=yes

# Use the full (non-minimal) Docker image.
.\z.ps1 build --with-docker -- all
```

Any unrecognized target is forwarded to `make` via Docker, just like on Linux:

```powershell
.\z.ps1 build -- kernel
.\z.ps1 build -- format-check
.\z.ps1 build -- lint-check
```

## Code Quality

### Formatting

```bash
# Check formatting issues.
./z build --with-cached-options -- format-check
# Auto-fix formatting issues.
./z build --with-cached-options -- format
```

### Linting

```bash
# Check linting issues.
./z build --with-cached-options -- lint-check
# Auto-fix linting issues.
./z build --with-cached-options -- lint
```

### Spell Checking

```bash
# Check spelling errors.
./z build --with-cached-options -- spellcheck
# Fix spelling errors.
./z build --with-cached-options -- spellcheck-fix
```

## Formal Verification

Nanvix uses Verus for formal verification of selected kernel crates. The expected Verus version is pinned in `build/verus-version` and auto-installed to `$(TOOLCHAIN_DIR)/verus` on the first run.

```bash
# Verify all annotated crates.
./z build --with-cached-options -- verify

# Verify a single crate.
./z build --with-cached-options -- verify-bitmap
```

- The `ensure-verus` prerequisite downloads the correct Verus release automatically.
- Override the install location with `VERUS_EXECUTABLE_DIR=/path/to/verus`.
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
on host crates (`uservm`, `nanvixd`, `nanvix-test`, `mkramfs`) without Docker.

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
> crates require the Docker-based cross-toolchain and are not checked by Rust Analyzer. Run
> `.\z.ps1 build -- check-kernel check-guest-binaries` via Docker for a full cross-target check.

## Troubleshooting Build Issues

- If builds fail with toolchain errors, verify `toolchain/` symlink points to a valid toolchain.
- If Docker builds fail, ensure Docker is running and the image is available.
- Use `./z help` for usage information.
- Cached build options are stored in `.z.cache` — delete this file to reset.
- **Windows:** Use `.\z.ps1 help` for Windows-specific usage information.
- **Windows:** If Docker builds fail with symlink errors, `z.ps1` automatically restores Git
  symlinks as file copies. Ensure Docker Desktop is running with Linux containers enabled.
- **Windows:** If the UserVM build fails, verify that the Windows Hypervisor Platform feature is
  enabled (`Get-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform`).
