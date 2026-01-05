# Building Nanvix

> ℹ️ The instructions in this document assume that you have a system with the
development environment already set up. For more information on how to set up
your development environment, please refer to the [setup.md](setup.md) document.

This document guides you through building Nanvix. You can either use the `z` utility script for a
simplified build process or do it manually.

## Table of Contents

- [Building Nanvix with `z` (Preferred Method)](#building-nanvix-with-z-preferred-method)
  - [Getting Started with `z`](#getting-started-with-z)
  - [Using `z` to Build Nanvix with Docker](#using-z-to-build-nanvix-with-docker)
  - [Using `z` to Build Nanvix with a Local Toolchain](#using-z-to-build-nanvix-with-a-local-toolchain)
  - [Selecting a Deployment Mode](#selecting-a-deployment-mode)
- [Building Nanvix Manually](#building-nanvix-manually)
  - [Manually Building Nanvix with Docker](#manually-building-nanvix-with-docker)
  - [Manually Building Nanvix with a Local Toolchain](#manually-building-nanvix-with-a-local-toolchain)

## Building Nanvix with `z` (Preferred Method)

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

### Selecting a Deployment Mode

Nanvix builds default to the single-process deployment path (`SINGLE_PROCESS=yes`). In this mode the
Linux Daemon and UserVM execute within the same process, which is ideal for local development and
faster iteration cycles. Set `SINGLE_PROCESS=no` whenever you need the multi-process deployment, which
enables the `multi-process` Cargo feature, builds the standalone `linuxd` and `uservm` binaries, and
matches the production topology.

#### Examples

Build multi-process artifacts with the `z` helper:

```bash
./z build -- all SINGLE_PROCESS=no
```

Or do the same through the raw Makefile interface:

```bash
make SINGLE_PROCESS=no all
```

You can freely switch between deployment modes by rebuilding with the desired `SINGLE_PROCESS`
setting; subsequent `make run` or `nanvixd` invocations will use the binaries produced by the latest
build.

## Building Nanvix Manually

Instead of using the `z` utility, you can build Nanvix manually.

### Manually Building Nanvix with Docker

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

### Manually Building Nanvix with a Local Toolchain

To build Nanvix using your local toolchain and default build parameters, run:

```bash
make all
```
