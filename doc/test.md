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
- [Test Layout](#test-layout)
  - [Rules](#rules)
  - [Writing Unit Tests](#writing-unit-tests)
  - [Checking the Layout](#checking-the-layout)
  - [Baseline and Allowlist](#baseline-and-allowlist)
  - [Migrating Inline Tests](#migrating-inline-tests)

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

> ℹ️ System integration tests are available on `microvm` machines on both Linux and Windows.

The system integration tests can be run directly using:

```bash
# Linux
./z build -- run-nanvix-tests
```

```powershell
# Windows
.\z.ps1 build -- run-nanvix-tests
```

The runner uses `test/test-standalone.toml` on Linux and
`test/test-standalone-windows.toml` on Windows.

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

## Test Layout

Tests are decoupled from mainline source code: every test (unit, integration, end-to-end) lives in
its own source file, never next to the code that it exercises. This keeps production sources small
and makes it possible to tell, from a diff alone, whether a change touches tests or production code.

### Rules

| Rule | Requirement                                                                                    |
|------|------------------------------------------------------------------------------------------------|
| R1   | `#[test]`-style attributes (`#[test]`, `#[tokio::test]`, `#[bench]`, ...) only appear in test files. |
| R2   | A test-only module (`#[cfg(test)]`, `#[cfg(feature = "test")]`, ...) is declared out of line (`mod tests;`) and resolves to a test file. |
| R3   | Other test-only code in mainline source (a "seam", for example a `#[cfg(test)]` fault-injection hook) is listed in the [allowlist](#baseline-and-allowlist). `use`, `extern crate` and `mod x;` items are always allowed. |
| R4   | Doc-tests do not run code: mark the fence `text` or `ignore`, or write a real test.              |
| R5   | Python tests live under `tests/`. C/C++ tests live under `src/tests/`.                          |
| R6   | End-to-end test manifests (`test-*.toml`) live under `test/`.                                   |
| R0   | A source file that the checker cannot parse (fix the file, or report a checker bug).            |

A Rust file is a *test file* when it lives in a `test`/`tests` directory (or under `src/tests/`), or
when its name is `test`, `tests`, `<name>_test`, `<name>_tests` or `test_<name>`.

### Writing Unit Tests

Declare the test module out of line next to the code it tests:

```rust
// src/parser.rs
#[cfg(test)]
mod tests;
```

and put the tests in the file that the compiler resolves for that module (`src/parser/tests.rs` for
`src/parser.rs`, `src/tests.rs` for `src/lib.rs`, `src/main.rs` and `mod.rs` files):

```rust
// src/parser/tests.rs
use super::*;

#[test]
fn parses_empty_input() {
    assert!(parse("").is_ok(), "empty input must be accepted");
}
```

A child module can reach the private items of its parent through `use super::*`, so tests rarely
need a seam. If the module has a name that does not mark a test file (for example `benchmarks`),
declare it with `#[path = "<dir>/tests/<name>.rs"]` so that it lives in a `tests` directory.

Tests of a crate that are not unit tests go in the crate's `tests/` directory (Cargo integration
tests) or under `src/tests/` (guest integration and stress tests). Python tests go under `tests/`,
and end-to-end manifests under `test/`.

### Checking the Layout

```bash
./z build -- test-layout-check      # Linux
```

```powershell
.\z.ps1 build -- test-layout-check  # Windows
```

The check uses only the Python standard library. It also runs from the `pre-commit` hooks, from
`./scripts/pipeline.sh` (step `test-layout`) and from the CI `checks` job. Pass `--format github` to
`scripts/check-test-layout.py` to emit GitHub Actions annotations.

### Baseline and Allowlist

Code that predates the policy is recorded in `scripts/test-layout-baseline.txt` (a ratchet). The
check fails on new violations, on baselined violations that grew, and on baseline entries that no
longer apply, so the baseline can only shrink. Never add an entry: put the test in its own file.
After fixing code, drop the entries that no longer apply with:

```bash
python3 scripts/check-test-layout.py --update-baseline
```

`scripts/test-layout-allowlist.txt` lists the seams (R3) that a reviewer explicitly approved. Each
entry needs a justification in the PR, and a stale entry fails the check.

### Migrating Inline Tests

`scripts/migrate-inline-tests.py` moves inline `#[cfg(test)] mod tests { ... }` modules into their
own files without changing module names, so test IDs stay the same:

```bash
python3 scripts/migrate-inline-tests.py --dry-run src/libs/<crate>   # preview
python3 scripts/migrate-inline-tests.py src/libs/<crate>             # migrate, format, shrink baseline
python3 scripts/migrate-inline-tests.py --verify origin/dev          # prove a pure migration
```

Modules that cannot be moved mechanically (they use `include_str!`, `file!()`, `#[path]`, are nested
in another inline module, or collide with an existing file) are reported and left in place; move
them by hand. `--verify REV` re-plans every changed file at `REV` and compares the code token by
token, so a reviewer can trust that a migration PR only moves code.
