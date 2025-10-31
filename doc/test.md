# Testing Nanvix

> ℹ️ The instructions in this document assume that you already know how to built
Nanvix. For more information on how to build Nanvix, please refer to the
[build.md](build.md) document.

This document guides you through testing Nanvix.

## Table of Contents

- [Running Full CI Pipeline](#running-full-ci-pipeline)
- [Running Unit Tests](#running-unit-tests)
- [Running System-Level Tests (MicroVM and Hyperlight Machines Only)](#running-system-level-tests-microvm-and-hyperlight-machines-only)
  - [HTTP Mode Tests](#http-mode-tests)
  - [Terminal Mode Tests](#terminal-mode-tests)
  - [Understanding Test Modes](#understanding-test-modes)

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

## Running System-Level Tests (MicroVM and Hyperlight Machines Only)

> ℹ️ This runs system-level tests with the default build parameters.  Check the
[build.md](build.md) document for more information on how to change default
build parameters.

System-level tests for Nanvix can be run in two modes: HTTP mode and terminal mode.

### HTTP Mode Tests

HTTP mode tests run programs through nanvixd's HTTP API. This is the default mode and supports all test types including WASM and interpreter-based programs (Python, QuickJS).

```bash
make run-nanvixd-http-tests
```

### Terminal Mode Tests

Terminal mode tests run programs directly through nanvixd's terminal interface. This mode provides a more direct execution path but does not support WASM or interpreter-based programs.

```bash
make run-nanvixd-terminal-tests
```

### Understanding Test Modes

**HTTP Mode:**

- Programs are invoked via HTTP requests to nanvixd
- Program arguments and input are passed as JSON payloads
- Supports all program types: native executables, WASM modules, and interpreter-based programs
- Uses `run-nanvixd.sh` script to communicate with nanvixd server

**Terminal Mode:**

- Programs are invoked directly by nanvixd with a terminal interface
- Input is provided directly via stdin (not as JSON)
- Only supports native executables (ELF binaries)
- Does not support WASM modules or interpreter-based programs (Python, QuickJS)
- Does not support L2 VM deployment

To run all tests (both HTTP and terminal modes):

```bash
make run-nanvixd-tests
```
