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
./z build -- all-kernel
# Nanvixd (also builds mkramfs and the guest binaries).
./z build -- all-nanvixd
# UserVM only.
./z build -- all-uservm
```

### Build Parameters

Set these as environment variables or pass them after `--` in the `z` command:

| Parameter        | Values                   | Default         |
|------------------|--------------------------|-----------------|
| `MACHINE`        | `microvm`                | `microvm`       |
| `TARGET`         | `x86`, `x86_64`          | `x86`           |
| `RELEASE`        | `yes`, `no`              | `no`            |
| `LOG_LEVEL`      | `trace`, `debug`,        | `trace`         |
|                  | `info`, `warn`,          |                 |
|                  | `error`, `panic`         |                 |
| `PROFILER`       | `yes`, `no`              | `no`            |

In release builds (`RELEASE=yes`), `LOG_LEVEL` defaults to `error`.

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
```

### Building on Windows (using `z.ps1`)

On Windows 11, the `z.ps1` PowerShell script provides the same CLI interface. Guest components are
cross-compiled using a local toolchain; host binaries (`nanvixd`, `uservm`) are built natively with the
microvm backend (WHP on Windows).

```powershell
# Build everything.
.\z.ps1 build -- all

# Build only the UserVM.
.\z.ps1 build -- all-uservm

# Build nanvixd (also builds mkramfs and the guest binaries).
.\z.ps1 build -- all-nanvixd

# Build only nanvix-bench.
.\z.ps1 build -- all-nanvix-bench

# Build only guest components (the kernel, libraries, and binaries).
.\z.ps1 build -- all-kernel all-guest-binaries

# Release build.
.\z.ps1 build -- all RELEASE=yes
```

`z.ps1` forwards all targets to `make`, just like `./z` on Linux, so target names are the same on
both hosts:

```powershell
.\z.ps1 build -- check
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

When spell checking flags a legitimate domain term or acronym (for example, `vas` for
Virtual Address Space, or `slab`), add it to `.codespellrc` rather than rewording the code
or documentation.

### Test Layout Check

```bash
# Check that tests live in their own files instead of mainline source.
./z build -- test-layout-check
```

The checker is `scripts/check-test-layout.py` (Python standard library only). See `doc/test.md`
("Test layout") for the rules and the allowlist of seams.

## Formal Verification

Nanvix uses Verus for formal verification of selected kernel crates. The expected Verus version is
pinned in `build/verus-version`. Verification uses `~/verus` by default, matching the setup
command. Set `VERUS_EXECUTABLE_DIR` only for a custom installation.

```bash
# Install Verus and run verification.
./z setup --verus
./z build -- verify

# Verify a single crate.
./z build -- verify-bitmap
```

- Set `VERUS_EXECUTABLE_DIR` to the directory containing the `verus` binary.
- Use `python3 scripts/setup/verus.py <dir>` to download the pinned release on Linux, or
  `python scripts/setup/verus.py <dir>` on Windows.
- The `vstd` crate version in `Cargo.toml` is exact-pinned (`=`) to match the Verus binary.

## Cleaning

```bash
./z clean        # Clean build artifacts.
./z distclean    # Remove all generated files.
```

### Cleaning on Windows

```powershell
.\z.ps1 clean        # Quick clean (build artifacts of all components).
.\z.ps1 distclean    # Full clean (cargo clean + all artifacts).
```

## CI/CD Pipeline

```bash
# Run the full CI pipeline locally.
./scripts/pipeline.sh
```

The pipeline covers spell checking, formatting, linting, building, and testing for `microvm` in
debug and release configurations. Run `./scripts/pipeline.sh` before declaring broad changes done.

## IDE Setup (Optional)

### Visual Studio Code

Use the host-specific settings template. The Linux template invokes `./z`, while the Windows
template routes Rust Analyzer through `./z.bat build -- check`, which runs `cargo check` on all
crates: host crates (`uservm`, `nanvixd`, `nanvix-test`, `mkramfs`, ...) natively, and the kernel
and guest crates by cross-compiling them.

**Linux:**

```bash
mkdir -p .vscode && cd .vscode
ln -s ../scripts/setup/vscode/settings-linux.json settings.json
```

**Windows (PowerShell):**

```powershell
New-Item -ItemType Directory -Path .vscode -Force
Copy-Item scripts\setup\vscode\settings-windows.json .vscode\settings.json
Copy-Item scripts\setup\vscode\tasks-windows.json .vscode\tasks.json
Copy-Item scripts\setup\vscode\launch-windows.json .vscode\launch.json
Copy-Item scripts\setup\vscode\extensions-windows.json .vscode\extensions.json
```

The Windows tasks run `z.ps1` (`build -- all`, `build -- all-uservm`, `build -- all-nanvixd`,
`run`, `test`, `build -- run-unit-tests`, `build -- run-nanvix-tests`). The launch configurations
debug `bin\uservm.exe` and `bin\nanvixd.exe` with `cppvsdbg` from the C/C++ extension, because
these MSVC binaries carry PDB debug information. The executables embed only the PDB file name, so
`uservm.mk` and `nanvixd.mk` copy each PDB next to its executable in `bin\`. The `preLaunchTask`
values must match the task labels in `tasks-windows.json`. See `doc/setup-windows.md`, "Visual
Studio Code".

> **Note:** The `check` target checks every crate, including the kernel and guest crates. To check
> fewer crates, run per-component targets instead, for example `.\z.ps1 build -- check-uservm` or
> `.\z.ps1 build -- check-kernel check-guest-binaries`.

## Troubleshooting Build Issues

- If builds fail with toolchain errors, verify `toolchain/` symlink points to a valid toolchain.
- Use `./z help` for usage information.
- **Windows:** Use `.\z.ps1 help` for Windows-specific usage information.
- **Windows:** If the UserVM build fails, verify that the Windows Hypervisor Platform feature is
  enabled (`Get-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform`).
