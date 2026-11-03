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
  - [Allowlist](#allowlist)
  - [Flagging Changes to Existing Tests](#flagging-changes-to-existing-tests)

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
| R3   | Other test-only code in mainline source (a "seam", for example a `#[cfg(test)]` fault-injection hook) is listed in the [allowlist](#allowlist). `use`, `extern crate` and `mod x;` items are always allowed. |
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

Rustdoc runs indented code blocks as well as Rust fences. Put illustrative indented examples inside
a `text` fence. Only plain `ignore` disables a doc-test everywhere; target-specific tags such as
`ignore-x86_64` still leave executable tests on other targets.

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

### Allowlist

The check is strict: every violation fails it. The only exception mechanism is
`scripts/test-layout-allowlist.txt`, which lists the seams (R3) that need private access to
mainline code and cannot move into a test file (for example a boot hook that the in-kernel test
runner calls). Only R3 can be allowlisted; a test or a test module in mainline source never can.

Each line has the form `R3 <path> <kind> <name> [xN]`, and `#` starts a comment. A line must match
exactly `N` occurrences (default 1) of the seam. Adding or removing occurrences fails the check:
lower the count after a partial removal, and delete the entry after a complete removal. Do
not write keys by hand. Print the lines for the seams that are not yet allowed with:

```bash
python3 scripts/check-test-layout.py --print-allowlist
```

Put a comment above every new entry that explains why the seam cannot live in a test file. Growing
the allowlist is flagged for [review](#flagging-changes-to-existing-tests).

### Flagging Changes to Existing Tests

Adding a test is routine. Changing a test that already exists can hide a regression (a weakened
assertion, a removed case, a test turned into a no-op), so those changes get extra scrutiny. The
`Test Change Review` workflow (`.github/workflows/test-change-review.yml`) runs
`scripts/flag-test-changes.py` on every pull request to `dev`. When it finds a change to an existing
test, it adds the `review::tests-modified` label and keeps a single comment up to date that lists
each change. The check is advisory: it never fails the pull request, and the label and comment
disappear when a later push no longer changes existing tests.

A change is flagged when it:

| Change                                                               | Flagged |
|----------------------------------------------------------------------|---------|
| Adds a test, a test file or a test manifest entry                    | No      |
| Edits the body, attributes or signature of an existing test          | Yes     |
| Inserts a statement into an existing test                            | Yes     |
| Edits a table, constant, helper, macro or manifest entry a test uses | Yes     |
| Removes a test, or deletes or renames a test file                    | Yes     |
| Grows the layout [allowlist](#allowlist)                             | Yes     |
| Moves a test, unchanged, to another file (such as `mod tests;`)      | No      |
| Reformats a test, reorders tests, or edits comments or docs          | No      |
| Changes mainline code outside a test                                 | No      |

The script compares the pull request with its merge base. It extracts every test item from the
tracked test code (Rust, Python and C/C++ test files, test modules and test-only seams in mainline
Rust, and the `test/*.toml` manifests), and fingerprints each item from its code with comments,
whitespace and known optional trailing commas ignored. Singleton tuple commas and unknown macro
token trees are preserved conservatively. Execution guards on containers also contribute to their
items' fingerprints, so disabling existing tests is flagged. An item that exists on both sides with
the same name but a different fingerprint is *modified*. An item that is gone from every changed
file is *removed*. An item that reappears unchanged in another changed file is a *move* and is not
flagged, which is why
moving tests into their own files produces no findings. A changed file that existed before and can no
longer be parsed is flagged as well, so that a syntax error never hides a change.

Run it locally against the branch that you target:

```bash
python3 scripts/flag-test-changes.py --base origin/dev           # list findings
python3 scripts/flag-test-changes.py --base origin/dev --json    # machine-readable report
```

Keep these limits in mind:

- The check is a heuristic safety net for reviewers, not a security boundary. For example, a test
  that is copied into a file that nothing compiles and then deleted from its original file looks
  like a move.
- Only the tracked languages and paths are analyzed. Test logic that lives in other files (shell
  scripts, build files) is not.
- The workflow runs on `pull_request_target` so that it can label pull requests from forks. It
  therefore only reads git objects and never checks out or runs code from the pull request, and the
  workflow and script are taken from `dev`, so a pull request cannot change how it is checked.
  For the same reason, the check only takes effect once it has been merged into `dev`.
- A pull request that adds `scripts/test-layout-allowlist.txt` itself reports every entry as new.
