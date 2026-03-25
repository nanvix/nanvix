# Building Nanvix (Windows)

> ℹ️ The instructions in this document assume that you have a system with the
development environment already set up. For more information on how to set up
your development environment, please refer to the [Windows setup](setup-windows.md) guide.

This document guides you through building Nanvix on Windows. On Windows, the `z.ps1` PowerShell
script provides the same interface as the Linux `z` utility. Host-side components are built natively,
while guest components are cross-compiled inside Docker.

## Table of Contents

- [Building Nanvix with `z.ps1`](#building-nanvix-with-zps1)
  - [Getting Started with `z.ps1`](#getting-started-with-zps1)
  - [Using `z.ps1` to Build Nanvix](#using-zps1-to-build-nanvix)
- [Building Individual Components](#building-individual-components)
- [Build Parameters](#build-parameters)
- [Code Quality Checks](#code-quality-checks)
- [Cleaning](#cleaning)

---

## Building Nanvix with `z.ps1`

`z.ps1` is a utility for building Nanvix on Windows. It provides you with a simplified interface
for building Nanvix either using Docker or your local toolchain.

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

# Set deployment mode.
.\z.ps1 build -- all DEPLOYMENT_MODE=standalone

# Set log level.
.\z.ps1 build -- all LOG_LEVEL=debug

# Combine multiple parameters.
.\z.ps1 build -- all RELEASE=yes DEPLOYMENT_MODE=standalone LOG_LEVEL=warn
```

Available build parameters:

| Parameter         | Default      | Values                                             |
|-------------------|--------------|----------------------------------------------------|
| `DEPLOYMENT_MODE` | `standalone` | `standalone`                                       |
| `LOG_LEVEL`       | `warn`       | `trace`, `debug`, `info`, `warn`, `error`, `panic` |
| `MACHINE`         | `microvm`    | `microvm`, `hyperlight`                            |
| `MESSAGE_FORMAT`  | (none)       | `json`, `json-diagnostic-rendered-ansi`             |
| `RELEASE`         | `no`         | `yes`, `no`                                        |
| `TARGET`          | `x86`        | `x86`                                              |
| `TIMEOUT`         | `600`        | Execution timeout in seconds                       |

## Code Quality Checks

```powershell
# Run cargo check on host crates (native, no Docker).
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
```

## Cleaning

```powershell
# Quick clean (removes build artifacts and cache).
.\z.ps1 clean

# Full clean (removes everything).
.\z.ps1 distclean
```
