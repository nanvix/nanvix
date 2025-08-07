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
  - [Using `z` to Build Nanvix with a Native Toolchain](#using-z-to-build-nanvix-with-a-native-toolchain)
- [Building Nanvix Manually](#building-nanvix-manually)
  - [Manually Building Nanvix with Docker](#manually-building-nanvix-with-docker)
  - [Manually Building Nanvix with a Native Toolchain](#manually-building-nanvix-with-a-native-toolchain)
- [Build Parameters](#build-parameters)
  - [Default Build Parameters](#default-build-parameters)
  - [Optional Build Parameters](#optional-build-parameters)

## Building Nanvix with `z` (Preferred Method)

`z` is a utility for building Nanvix. It provides you with a simplified interface for building
Nanvix either using Docker or your native toolchain.

### Getting Started with `z`

For more information on how to use the `z` utility, you can run:

```bash
./z help
```

### Using `z` to Build Nanvix with Docker

To build Nanvix using the latest Docker image and default build parameters, simply run:

```bash
./z build --with-docker -- all
```

> ℹ️  Refer to the [Build Parameters](#build-parameters) section for more information on how to
customize the build process.

### Using `z` to Build Nanvix with a Native Toolchain

To build Nanvix using your native toolchain and default build parameters, simply run:

```bash
./z build -- all
```

> ℹ️  Refer to the [Build Parameters](#build-parameters) section for more information on how to
customize the build process.

## Building Nanvix Manually

Instead of using the `z` utility, you can build Nanvix manually.

### Manually Building Nanvix with Docker

To build Nanvix using the latest Docker image and default build parameters, simply run:

```bash
docker run \
  -it --rm -v"$(pwd):/mnt" \
  nanvix/toolchain \
  /bin/bash -l -c "\
    cd /mnt ; \
    git config --global --add safe.directory /mnt ; \
    make TOOLCHAIN_DIR=/opt/nanvix all ; \
    chown -R $(id -u):$(id -g) . "
```

> ℹ️  Refer to the [Build Parameters](#build-parameters) section for more information on how to
customize the build process.

### Manually Building Nanvix with a Native Toolchain

To build Nanvix using your native toolchain and default build parameters, simply run:

```bash
make all
```

> ℹ️  Refer to the [Build Parameters](#build-parameters) section for more information on how to
customize the build process.

## Build Parameters

The following parameters can be used to customize the build process.

### Default Build Parameters

- `TOOLCHAIN_DIR=$PWD/toolchain`: Set the toolchain directory to the current working directory.
- `MACHINE=microvm`: Set the target machine to `microvm`.
- `TARGET=x86`: Set the target CPU architecture to `x86`.
- `LOG_LEVEL=warn`: Set the output log level to `warn`.
- `RELEASE=no`: Disable release build (implies debug build).
- `PROFILER=no`: Disable profiler for MicroVM.
- `IMAGE=bin/hello-rust-nostd.elf`: Set the system image to `bin/hello-rust-nostd.elf`.

### Optional Build Parameters

- `TOOLCHAIN_DIR=</path/to/toolchain>`: Set the toolchain directory.
- `LOG_LEVEL=<trace|info|warn|error>`: Set the output log level.
- `MACHINE=<hyperlight|microvm|qemu-pc>`: Set the target machine.
- `PROFILER=<yes|no>`: Enable/Disable profiler for MicroVM.
- `RELEASE=<yes|no>`: Enable/Disable release build.
- `TARGET=<architecture>`: Set the target CPU architecture.
- `TIMEOUT=<seconds>`: Set the execution timeout.
- `IMAGE=<image>`: Set system image.
