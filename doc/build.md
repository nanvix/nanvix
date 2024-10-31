# Building Nanvix

This document instructs you on how to build Nanvix.

> The instructions in this document assume that you have your disposal a system with the development environment already set up. For more information on how to setup your development environment, please refer to the [Setting Up Your Development Environment](setup.md) document.

## Table of Contents

- [Build Nanvix with Default Parameters](#build-nanvix-with-default-parameters)
- [List of Optional Build Parameters](#list-of-optional-build-parameters)

## Build Nanvix with Default Parameters

```bash
# Builds Nanvix with default parameters:
# make MACHINE=qemu-pc TARGET=x86 LOG_LEVEL=warn RELEASE=no PROFILER=no all
make all
```

## List of Optional Build Parameters

- `LOG_LEVEL=<trace|info|warn|error>`: Set the output log level.
- `MACHINE=<qemu-pc|microvm>`: Set the target machine.
- `PROFILER=<yes|no>`: Enable/Disable profiler (`microvm` machine only).
- `RELEASE=<yes|no>`: Enable/Disable release build.
- `TARGET=x86`: Set target CPU architecture.
