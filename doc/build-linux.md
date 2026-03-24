# Building Nanvix (Linux)

> ℹ️ The instructions in this document assume that you have a system with the
development environment already set up. For more information on how to set up
your development environment, please refer to the [Linux setup](setup-linux.md) guide.

This document guides you through building Nanvix on Linux. You can either use the `z` utility
script for a simplified build process or do it manually.

## Table of Contents

- [Building Nanvix with `z`](#building-nanvix-with-z)
  - [Getting Started with `z`](#getting-started-with-z)
  - [Using `z` to Build Nanvix with Docker](#using-z-to-build-nanvix-with-docker)
  - [Using `z` to Build Nanvix with a Local Toolchain](#using-z-to-build-nanvix-with-a-local-toolchain)
- [Building Individual Components](#building-individual-components)
- [Build Parameters](#build-parameters)
- [Code Quality Checks](#code-quality-checks)
- [Cleaning](#cleaning)
- [Formal Verification with Verus](#formal-verification-with-verus)

---

## Building Nanvix with `z`

`z` is a utility for building Nanvix. It provides you with a simplified interface for building
Nanvix either using Docker or your local toolchain.

### Getting Started with `z`

For more information on how to use the `z` utility, you can run:

```bash
./z help
```

### Using `z` to Build Nanvix with Docker

To build Nanvix using the latest Docker image and default build parameters, run:

```bash
./z build --with-docker -- all
```

### Using `z` to Build Nanvix with a Local Toolchain

To build Nanvix using your local toolchain and default build parameters, run:

```bash
./z build -- all
```

## Building Individual Components

```bash
# Build only the kernel.
./z build -- kernel

# Build only nanvixd.
./z build -- nanvixd

# Build only the UserVM.
./z build -- uservm

# Build only guest components.
./z build -- guest
```

## Build Parameters

Pass build parameters after `--`:

```bash
# Release build.
./z build -- all RELEASE=yes

# Set deployment mode.
./z build -- all DEPLOYMENT_MODE=standalone

# Set log level.
./z build -- all LOG_LEVEL=debug

# Combine multiple parameters.
./z build -- all RELEASE=yes DEPLOYMENT_MODE=multi-process LOG_LEVEL=warn
```

Available build parameters:

| Parameter         | Default         | Values                                                             |
|-------------------|-----------------|--------------------------------------------------------------------|
| `DEPLOYMENT_MODE` | `multi-process` | `standalone`, `single-process`, `multi-process`, `l2`              |
| `LOG_LEVEL`       | `warn`          | `trace`, `debug`, `info`, `warn`, `error`, `panic`                 |
| `MACHINE`         | `microvm`       | `hyperlight`, `microvm`, `qemu-pc`, `qemu-isapc`, `qemu-baremetal` |
| `PROFILER`        | `no`            | `yes`, `no`                                                        |
| `RELEASE`         | `no`            | `yes`, `no`                                                        |
| `TARGET`          | `x86`           | `x86`                                                              |
| `TIMEOUT`         | `600`           | Execution timeout in seconds                                       |

## Code Quality Checks

```bash
# Check formatting.
./z build -- format-check

# Fix formatting.
./z build -- format

# Check linting.
./z build -- lint-check

# Fix linting.
./z build -- lint

# Spell checking.
./z build -- spellcheck
./z build -- spellcheck-fix

# Run all validation checks.
./z build -- check

# Run unit tests.
./z build -- run-unit-tests
```

## Cleaning

```bash
# Quick clean (removes build artifacts and cache).
./z clean

# Full clean (removes everything).
./z distclean
```

## Formal Verification with Verus

Nanvix uses [Verus](https://github.com/verus-lang/verus) for formal verification of selected
kernel crates. The correct Verus version is pinned in `build/verus-version` and is automatically
downloaded on the first verification run.

To run formal verification:

```bash
# Verify all annotated crates.
./z build --with-cached-options -- verify

# Verify a single crate (e.g., bitmap).
./z build --with-cached-options -- verify-bitmap
```

Verus is installed to `$(TOOLCHAIN_DIR)/verus` by default. When `VERUS_EXECUTABLE_DIR` points
to a custom location, the build assumes pre-built or locally compiled Verus binaries are already
present there and skips the automatic download (the directory is used read-only; nothing is
written to it). `VERUS_EXECUTABLE_DIR` must point to the directory that contains the `verus`
executable (and required companion binaries), not just the Verus source tree.

```bash
# Use a custom Verus installation (pre-built or source-built).
# VERUS_EXECUTABLE_DIR must be the directory that contains the verus binary.
./z build --with-cached-options -- verify VERUS_EXECUTABLE_DIR=~/verus/target-verus/release
```
