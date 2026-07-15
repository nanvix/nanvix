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

## System Integration Tests (microvm only)

```bash
./z build -- run-nanvix-tests
```

Test configurations are selected by host platform:

- Linux: `test/test-standalone.toml`
- Windows: `test/test-standalone-windows.toml`

## All Tests

```bash
./z build -- test
```

## Windows

On Windows, unit tests can be run natively through `z.ps1`:

```powershell
.\z.ps1 build -- run-unit-tests
```

System integration tests are also available on Windows on `microvm` machines:

```powershell
.\z.ps1 build -- run-nanvix-tests
```

## Troubleshooting Test Failures

- Ensure the project builds successfully before running tests (see the `build` skill).
- Use `LOG_LEVEL=trace` or `LOG_LEVEL=debug` for more verbose output when diagnosing failures.
- See the `troubleshooting` skill for deeper diagnosis of runtime and test failures.
