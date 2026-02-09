# Testing Nanvix

> ℹ️ The instructions in this document assume that you already know how to built
Nanvix. For more information on how to build Nanvix, please refer to the
[build.md](build.md) document.

This document guides you through testing Nanvix.

## Table of Contents

- [Running Full CI Pipeline](#running-full-ci-pipeline)
- [Running Unit Tests](#running-unit-tests)
- [Running System Integration Tests (MicroVM and Hyperlight Only)](#running-system-integration-tests-microvm-and-hyperlight-only)
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
make run-unit-tests
```

## Running System Integration Tests (MicroVM and Hyperlight Only)

> ℹ️ System integration tests are only available on `microvm` and `hyperlight`
machines. These tests use the `nanvix-test.elf` utility to run programs through
the Nanvix Daemon.

The system integration tests can be run directly using:

```bash
make run-nanvix-tests
```

The appropriate test configuration is automatically selected based on the
deployment mode:

- **Single-process mode** (`SINGLE_PROCESS=yes`): Uses `test/test-single_process.toml`
- **L2 VM mode** (`L2_VM=yes`): Uses `test/test-l2.toml`
- **Multi-process mode** (default): Uses `test/test-multi_process.toml`

### Test Modes

The `nanvix-test.elf` utility supports two execution modes:

**HTTP Mode:**

- Programs are invoked via HTTP requests to nanvixd.
- Supports all program types: native executables, WASM modules, and interpreter-based programs.

**Terminal Mode:**

- Programs are invoked directly by nanvixd with a terminal interface.
- Only supports native executables (ELF binaries).
- Not available for L2 VM deployments (`L2_VM=yes`); L2 configurations always use the HTTP executor.

## Running All Tests

> ℹ️ This target sequentially invokes each underlying test suite using independent `make` calls.

```bash
make test
```

On `microvm` and `hyperlight` machines, `make test` runs both unit tests and
system integration tests. On other machines, only unit tests are executed.
