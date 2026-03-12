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
| `MACHINE`        | `microvm`, `hyperlight`, | `microvm`       |
|                  | `qemu-pc`, `qemu-isapc`, |                 |
|                  | `qemu-baremetal`         |                 |
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

## CI/CD Pipeline

```bash
# Run the full CI pipeline locally.
./scripts/pipeline.sh
```

The pipeline covers: spell checking, formatting, linting, building, and testing across multiple
machine and deployment configurations.

## Troubleshooting Build Issues

- If builds fail with toolchain errors, verify `toolchain/` symlink points to a valid toolchain.
- If Docker builds fail, ensure Docker is running and the image is available.
- Use `./z help` for usage information.
- Cached build options are stored in `.z.cache` — delete this file to reset.
