# Building Nanvix

> ℹ️ The instructions in this document assume that you have a system with the
development environment already set up. For more information on how to set up
your development environment, please refer to the [setup.md](setup.md) document.

This document guides you through building Nanvix. You can either use the `z` utility script for a
simplified build process or do it manually.

## Table of Contents

- [Building Nanvix on Linux](#building-nanvix-on-linux)
  - [Building Nanvix with `z` (Preferred Method)](#building-nanvix-with-z-preferred-method)
  - [Building Nanvix Manually](#building-nanvix-manually)
  - [Formal Verification with Verus](#formal-verification-with-verus)
- [Building Nanvix on Windows](#building-nanvix-on-windows)
  - [Building Everything](#building-everything)
  - [Building Individual Components](#building-individual-components)
  - [Build Parameters (Windows)](#build-parameters-windows)
  - [Code Quality Checks (Windows)](#code-quality-checks-windows)
  - [Cleaning (Windows)](#cleaning-windows)

## Building Nanvix on Linux

> ℹ️ Ensure you have completed the [Linux setup](setup.md#linux-setup) before proceeding.

### Building Nanvix with `z` (Preferred Method)

`z` is a utility for building Nanvix. It provides you with a simplified interface for building
Nanvix either using Docker or your local toolchain.

#### Getting Started with `z`

For more information on how to use the `z` utility, you can run:

```bash
./z help
```

#### Using `z` to Build Nanvix with Docker

To build Nanvix using the latest Docker image and default build parameters, run:

```bash
./z build --with-docker -- all
```

#### Using `z` to Build Nanvix with a Local Toolchain

To build Nanvix using your local toolchain and default build parameters, run:

```bash
./z build -- all
```

### Building Nanvix Manually

Instead of using the `z` utility, you can build Nanvix manually.

#### Manually Building Nanvix with Docker

To build Nanvix using the latest Docker image and default build parameters, run:

```bash
DOCKER_BUILDKIT=1 docker build \
    --build-arg BASE_IMAGE="nanvix/toolchain:v1.0.x-minimal" \
    --build-arg BUILD_PARAMS="all" \
    --build-arg SYSROOT_SUFFIX="debug" \
    --build-arg WORKSPACE_PATH="$(pwd -P)" \
    --output type=local,dest=. \
    --progress=plain \
    -f scripts/setup/Dockerfile.build \
    .
```

> ℹ️ The `SYSROOT_SUFFIX` and `WORKSPACE_PATH` arguments are optional. `SYSROOT_SUFFIX`
> defaults to `debug` (use `release` for release builds). `WORKSPACE_PATH` defaults
> to `/mnt`, but should be set to `$(pwd -P)` so that Python and other binaries with
> embedded absolute paths can find their libraries at runtime.

#### Manually Building Nanvix with a Local Toolchain

To build Nanvix using your local toolchain and default build parameters, run:

```bash
make all
```

You can also build in **standalone mode**, which excludes `linuxd` and routes all I/O through the
local in-memory VFS layer and kernel debug serial port:

```bash
make all DEPLOYMENT_MODE=standalone
```

### Formal Verification with Verus

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

## Building Nanvix on Windows

On Windows, the `z.ps1` PowerShell script provides the same interface as the Linux `z` utility.
Guest components (kernel, user binaries) are cross-compiled inside Docker, while the UserVM is
built natively using the Windows Hypervisor Platform (WHP) backend. The WHP backend is
automatically selected when building with the `microvm` feature on Windows.

> ℹ️ Ensure you have completed the [Windows setup](setup.md#windows-setup) before proceeding.

### Building Everything

Build all components:

```powershell
.\z.ps1 build -- all
```

### Building Individual Components

```powershell
# Build only the UserVM.
.\z.ps1 build -- uservm

# Build only guest components.
.\z.ps1 build -- guest
```

### Build Parameters (Windows)

Pass build parameters after `--`, just like on Linux:

```powershell
# Release build.
.\z.ps1 build -- all RELEASE=yes

# Use the full (non-minimal) Docker image.
.\z.ps1 build --with-docker -- all
```

Any make target not handled by `z.ps1` directly is forwarded to `make` via Docker:

```powershell
# Forward a custom make target to Docker.
.\z.ps1 build -- kernel
```

### Code Quality Checks (Windows)

Code quality checks run inside Docker and work the same as on Linux:

```powershell
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

### Cleaning (Windows)

```powershell
# Quick clean (removes build artifacts and cache).
.\z.ps1 clean

# Full clean (removes everything).
.\z.ps1 distclean
```
