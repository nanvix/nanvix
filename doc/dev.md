# Developing Nanvix

> ℹ️ The instructions in this document assume that you already know how to built
Nanvix. For more information on how to build Nanvix, please refer to the
[build.md](build.md) document.

This document guides you through testing Nanvix.

## Table of Contents

- [Table of Contents](#table-of-contents)
- [Running Unit Tests](#running-unit-tests)
- [Running System-Level Tests (MicroVM and Hyperlight Machines Only)](#running-system-level-tests-microvm-and-hyperlight-machines-only)

## Running Full CI Pipeline

```bash
make ./scripts/pipeline.sh
```

## Running Unit Tests

> ℹ️ This runs unit tests with the default build parameters. Check the
[build.md](build.md) document for more information on how to change default
build parameters.

```bash
make run-unit-tests
```

## Running System-Level Tests (MicroVM and Hyperlight Machines Only)

> ℹ️ This runs system-level tests with the default build parameters.  Check the
[build.md](build.md) document for more information on how to change default
build parameters.

```bash
make run-nanvixd-tests
```
