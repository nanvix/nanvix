# Building Nanvix (Windows)

> ℹ️ The instructions in this document assume that you have a system with the
development environment already set up. For more information on how to set up
your development environment, please refer to the [Windows setup](setup-windows.md) guide.

This document guides you through building Nanvix on Windows. On Windows, the `z.ps1` PowerShell
script provides the same interface as the Linux `z` utility. Host-side components are built natively,
while guest components are cross-compiled using a local toolchain.

## Table of Contents

- [Building Nanvix with `z.ps1`](#building-nanvix-with-zps1)
  - [Getting Started with `z.ps1`](#getting-started-with-zps1)
  - [Using `z.ps1` to Build Nanvix](#using-zps1-to-build-nanvix)
- [Building Individual Components](#building-individual-components)
- [Build Parameters](#build-parameters)
- [Code Quality Checks](#code-quality-checks)
- [Formal Verification with Verus](#formal-verification-with-verus)
- [Cleaning](#cleaning)

---

## Building Nanvix with `z.ps1`

`z.ps1` is a utility for building Nanvix on Windows. It provides you with a simplified interface
for building Nanvix using your local toolchain.

### Getting Started with `z.ps1`

For more information on how to use the `z.ps1` utility, you can run:

```powershell
.\z.ps1 help
```

### Using `z.ps1` to Build Nanvix

To build Nanvix using with default build parameters, run:

```powershell
.\z.ps1 build -- all
```

## Building Individual Components

```powershell
# Build only the kernel.
.\z.ps1 build -- kernel

# Build only nanvixd.
.\z.ps1 build -- nanvixd

# Build only nanvix-bench.
.\z.ps1 build -- nanvix-bench

# Build only the UserVM.
.\z.ps1 build -- uservm

# Build only guest components.
.\z.ps1 build -- guest
```

## Build Parameters

Pass build parameters after `--`:

```powershell
# Release build.
.\z.ps1 build -- all RELEASE=yes

# Set log level.
.\z.ps1 build -- all LOG_LEVEL=debug

# Combine multiple parameters.
.\z.ps1 build -- all RELEASE=yes LOG_LEVEL=error
```

Available build parameters:

| Parameter        | Default   | Values                                             |
| ---------------- | --------- | -------------------------------------------------- |
| `LOG_LEVEL`      | `error`   | `trace`, `debug`, `info`, `warn`, `error`, `panic` |
| `MACHINE`        | `microvm` | `microvm`                                          |
| `MESSAGE_FORMAT` | (none)    | `json`, `json-diagnostic-rendered-ansi`            |
| `RELEASE`        | `no`      | `yes`, `no`                                        |
| `TARGET`         | `x86`     | `x86`                                              |
| `TIMEOUT`        | `600`     | Execution timeout in seconds                       |

## Code Quality Checks

```powershell
# Run cargo check on host crates.
.\z.ps1 build -- check

# Check formatting.
.\z.ps1 build -- format-check

# Fix formatting.
.\z.ps1 build -- format

# Check linting.
.\z.ps1 build -- lint-check

# Fix linting.
.\z.ps1 build -- lint

# Spell checking.
.\z.ps1 build -- spellcheck
.\z.ps1 build -- spellcheck-fix

# Run unit tests.
.\z.ps1 build -- run-unit-tests

# Run standalone integration tests on Windows.
.\z.ps1 build -- run-nanvix-tests

# Run the ported POSIX C test suites on Windows (requires LLVM/Clang; see setup-windows.md).
.\z.ps1 build -- run-posix-tests
```

## Formal Verification with Verus

Nanvix uses [Verus](https://github.com/verus-lang/verus) for formal verification of selected
kernel crates. The correct Verus version is pinned in `build/verus-version`.

To install Verus on Windows, use the setup command or the PowerShell setup script:

```powershell
# Option 1: Automated install via z.ps1 (installs to %USERPROFILE%\verus).
.\z.ps1 setup --verus

# Option 2: Manual install to a custom directory.
.\scripts\setup\verus.ps1 C:\verus
```

To run formal verification:

```powershell
# Verify all annotated crates.
.\z.ps1 build -- verify

# Verify a single crate (e.g., bitmap).
.\z.ps1 build -- verify-bitmap
```

Verification uses `%USERPROFILE%\verus` by default. For a custom installation, set
`VERUS_EXECUTABLE_DIR` on the command line:

```powershell
.\z.ps1 build -- verify VERUS_EXECUTABLE_DIR=C:\verus
```

## Cleaning

```powershell
# Quick clean (removes build artifacts and cache).
.\z.ps1 clean

# Full clean (removes everything).
.\z.ps1 distclean
```
