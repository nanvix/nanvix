# Building Nanvix

> ℹ️ The instructions in this document assume that you have a system with the
development environment already set up. For more information on how to set up
your development environment, please refer to the [setup.md](setup.md) document.

This document guides you through building Nanvix. You can either use native tools or a Docker image.

## Table of Contents

- [Building Nanvix with Native Tools](#building-nanvix-with-native-tools)
- [Building Nanvix with Docker](#building-nanvix-with-docker)
- [Default Build Parameters](#default-build-parameters)
- [List of Optional Build Parameters](#list-of-optional-build-parameters)

## Building Nanvix with Native Tools

> ℹ️ This builds Nanvix with the default parameters.

```bash
# Build Nanvix with default parameters:
make all
```

## Building Nanvix with Docker

> ℹ️ This builds Nanvix with the default parameters except for the toolchain directory.

```bash
# Build Nanvix with default parameters:
docker run \
  --rm -v"$(pwd):/mnt" \
  nanvix/toolchain \
  /bin/bash -l -c \
  "cd /mnt ; git config --global --add safe.directory /mnt ; make TOOLCHAIN_DIR=/opt all"
```

## Default Build Parameters

- `TOOLCHAIN_DIR=$PWD/toolchain`: Set the toolchain directory to the current working directory.
- `MACHINE=qemu-pc`: Set the target machine to `qemu-pc`.
- `TARGET=x86`: Set the target CPU architecture to `x86`.
- `LOG_LEVEL=warn`: Set the output log level to `warn`.
- `RELEASE=no`: Disable release build (implies debug build).
- `PROFILER=no`: Disable profiler for MicroVM.

## List of Optional Build Parameters

- `TOOLCHAIN_DIR=</path/to/toolchain>`: Set the toolchain directory.
- `LOG_LEVEL=<trace|info|warn|error>`: Set the output log level.
- `MACHINE=<hyperlight|microvm|qemu-pc>`: Set the target machine.
- `PROFILER=<yes|no>`: Enable/Disable profiler for MicroVM.
- `RELEASE=<yes|no>`: Enable/Disable release build.
- `TARGET=<architecture>`: Set the target CPU architecture.
- `TIMEOUT=<seconds>`: Set the execution timeout.
