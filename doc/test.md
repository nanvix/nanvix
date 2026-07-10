# Testing Nanvix

> ℹ️ The instructions in this document assume that you already know how to build
Nanvix. For more information on how to build Nanvix, please refer to the
[build.md](build.md) document.

This document guides you through testing Nanvix.

## Table of Contents

- [Table of Contents](#table-of-contents)
- [Running Full CI Pipeline](#running-full-ci-pipeline)
- [Running Unit Tests](#running-unit-tests)
- [Running System Integration Tests (MicroVM Only)](#running-system-integration-tests-microvm-only)
  - [Test Modes](#test-modes)
- [Running All Tests](#running-all-tests)

## Running Full CI Pipeline

```bash
./scripts/pipeline.sh
```

## Running Unit Tests

> ℹ️ This runs unit tests with the default build parameters. Check the
[build.md](build.md) document for more information on how to change default
build parameters.

```bash
./z build -- run-unit-tests
```

## Running System Integration Tests (MicroVM Only)

> ℹ️ System integration tests are available on `microvm`
machines on both Linux and Windows. On Windows, only standalone mode (`DEPLOYMENT_MODE=standalone`)
is supported.

The system integration tests can be run directly using:

```bash
# Linux
./z build -- run-nanvix-tests
```

```powershell
# Windows (standalone mode only)
.\z.ps1 build -- run-nanvix-tests
```

The appropriate test configuration is automatically selected based on the
deployment mode:

- **Standalone mode** (`DEPLOYMENT_MODE=standalone`): Uses `test/test-standalone.toml` on Linux and
  `test/test-standalone-windows.toml` on Windows
- **Single-process mode** (`DEPLOYMENT_MODE=single-process`): Uses `test/test-single_process.toml`

### Test Modes

The `nanvix-test.elf` utility supports two execution modes:

**HTTP Mode:**

- Programs are invoked via HTTP requests to nanvixd.
- Supports native executables (ELF binaries).

**Terminal Mode:**

- Programs are invoked directly by nanvixd with a terminal interface.
- Only supports native executables (ELF binaries).

## Running All Tests

> ℹ️ This target sequentially invokes each underlying test suite.

```bash
./z build -- test
```

`./z build -- test` runs both unit tests and system
integration tests.
