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
./z build --with-cached-options -- run-unit-tests
```

## System Integration Tests (microvm and hyperlight only)

```bash
./z build --with-cached-options -- run-nanvix-tests
```

Test configurations are auto-selected based on deployment mode:

- Single-process: `test/test-single_process.toml`
- L2 VM: `test/test-l2.toml`
- Multi-process: `test/test-multi_process.toml`

## All Tests

```bash
./z build --with-cached-options -- test
```

## Troubleshooting Test Failures

- Ensure the project builds successfully before running tests (see the `build` skill).
- Use `LOG_LEVEL=trace` or `LOG_LEVEL=debug` for more verbose output when diagnosing failures.
- Hyperlight does not support ramfs, so standalone integration tests must be excluded for hyperlight.
- See the `troubleshooting` skill for deeper diagnosis of runtime and test failures.
