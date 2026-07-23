# Building Nanvix (Linux)

> ℹ️ The instructions in this document assume that you have a system with the
development environment already set up. For more information on how to set up
your development environment, please refer to the [Linux setup](setup-linux.md) guide.

This document guides you through building Nanvix on Linux. You can either use the `z` utility
script for a simplified build process or do it manually.

## Table of Contents

- [Building Nanvix with `z`](#building-nanvix-with-z)
  - [Getting Started with `z`](#getting-started-with-z)
- [Building Individual Components](#building-individual-components)
- [Build Parameters](#build-parameters)
- [Code Quality Checks](#code-quality-checks)
- [Cleaning](#cleaning)
- [Formal Verification with Verus](#formal-verification-with-verus)

---

## Building Nanvix with `z`

`z` is a utility for building Nanvix. It provides you with a simplified interface for building
Nanvix using your local toolchain.

### Getting Started with `z`

For more information on how to use the `z` utility, you can run:

```bash
./z help
```

To build Nanvix with default build parameters, run:

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

# Set log level.
./z build -- all LOG_LEVEL=debug

# Combine multiple parameters.
./z build -- all RELEASE=yes LOG_LEVEL=error
```

Available build parameters:

| Parameter   | Default   | Values                                             |
| ----------- | --------- | -------------------------------------------------- |
| `LOG_LEVEL` | `error`   | `trace`, `debug`, `info`, `warn`, `error`, `panic` |
| `MACHINE`   | `microvm` | `microvm`                                          |
| `PROFILER`  | `no`      | `yes`, `no`                                        |
| `RELEASE`   | `no`      | `yes`, `no`                                        |
| `TARGET`    | `x86`     | `x86`                                              |
| `TIMEOUT`   | `600`     | Execution timeout in seconds                       |

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
kernel crates. The correct Verus version is pinned in `build/verus-version`.

Install the pinned release and run formal verification:

```bash
# Install Verus to ~/verus.
./z setup --verus

# Verify all annotated crates.
./z build -- verify

# Verify a single crate (e.g., bitmap).
./z build -- verify-bitmap
```

By default, verification uses `~/verus`, matching the setup command. Set
`VERUS_EXECUTABLE_DIR` to use a custom installation, or set it to an empty value to skip
verification explicitly:

```bash
# Install to a custom directory.
python3 scripts/setup/verus.py ~/toolchain/verus
./z build -- verify VERUS_EXECUTABLE_DIR=~/toolchain/verus

# Or use a source-built installation.
./z build -- verify VERUS_EXECUTABLE_DIR=~/verus/target-verus/release
```
