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

## Building Nanvix Manually

Instead of using the `z` utility, you can build Nanvix manually.

### Manually Building Nanvix with Docker

To build Nanvix using the latest Docker image and default build parameters, run:

```bash
docker run \
  -it --rm \
  -v"$(pwd -P):$(pwd -P)" \
  -w"$(pwd -P)" \
  nanvix/toolchain \
  /bin/bash -l -c "\
    set -e; \
    git config --global --add safe.directory '*' ; \
    make TOOLCHAIN_DIR=/opt/nanvix all ; \
    chown -R $(id -u):$(id -g) . "
```

### Manually Building Nanvix with a Local Toolchain

To build Nanvix using your local toolchain and default build parameters, run:

```bash
make all
```
