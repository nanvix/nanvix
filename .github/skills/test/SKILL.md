---
name: test
description: Guide for running Nanvix tests with z. Use this when asked to run unit tests, integration tests, or the full test suite.
---

# Test Nanvix

Use this skill when the user asks to run tests on Nanvix. This covers unit tests, system
integration tests, and the combined test suite exposed through the `z` utility.

## Prerequisites

- Development environment set up per `doc/setup.md`.
- A successful build of the components under test (see the `build` skill).

## Unit Tests

```bash
./z build -- run-unit-tests
```

## System Integration Tests (microvm and hyperlight only)

```bash
./z build -- run-nanvix-tests
```

Test configurations are auto-selected based on deployment mode:

- Single-process: `test/test-single_process.toml`
- L2 VM: `test/test-l2.toml`
- Multi-process: `test/test-multi_process.toml`

## All Tests

```bash
./z build -- test
```

## Windows

On Windows, unit tests can be run via Docker through `z.ps1`:

```powershell
.\z.ps1 build -- run-unit-tests
```

System integration tests (`run-nanvix-tests`) and `nanvixd`-based tests are **Linux-only**.
The standalone UserVM can be launched on Windows for manual verification, but the automated
test harness (`nanvix-test`) requires `nanvixd`, which is not available on Windows.

## Troubleshooting Test Failures

- Ensure the project builds successfully before running tests (see the `build` skill).
- Use `LOG_LEVEL=trace` or `LOG_LEVEL=debug` for more verbose output when diagnosing failures.
- Hyperlight does not support ramfs, so standalone integration tests must be excluded for hyperlight.
- See the `troubleshooting` skill for deeper diagnosis of runtime and test failures.
