---
name: test-development
description: Guide for writing, running, and debugging Nanvix unit, integration, and system tests in Rust and C/C++. Use this when asked about test implementation or failures.
---

# Test Development

Use this skill when the user asks about writing, running, or debugging tests in Nanvix. Nanvix has
unit tests, integration tests, and system tests written in Rust and C/C++.

## Test Categories

### Unit Tests

Unit tests are test-only modules (`#[cfg(test)] mod tests;`) of library crates, kept in their own
files rather than inline in mainline source. They run on the host system using `cargo test`.

```bash
./z build -- run-unit-tests
```

Libraries with unit tests are listed in
`ALL_GUEST_RUST_LIBS_TEST_LIST`: `arch`, `bitmap`, `config`,
`elf`, `error`, `type-safe`, `proc`, `raw-array`, `slab`,
`static_assert`, `libc_string`, `syslog-macros`, `syslog`.

### Integration Tests (Guest — Rust)

| Test          | Path                                  | Purpose            |
|---------------|---------------------------------------|--------------------|
| `testd`       | `src/tests/integration/testd/`        | Guest test daemon  |
| `arch-rust`   | `src/tests/integration/arch-rust/`    | Arch tests         |
| `file-rust`   | `src/tests/integration/file-rust/`    | File tests         |
| `thread-rust` | `src/tests/integration/thread-rust/`  | Thread tests       |
| `stress-rust` | `src/tests/stress/stress-rust/`       | Stress tests       |
| `test-kernel` | `src/tests/integration/test-kernel/`  | Kernel tests       |
| `linux-app`   | `src/tests/integration/linux-app/`    | Linux app tests    |

### System Integration Tests

System tests are available on `microvm` machines on Linux and Windows.

```bash
# Linux
./z build -- run-nanvix-tests
```

```powershell
# Windows
.\z.ps1 build -- run-nanvix-tests
```

Test configurations:

- `test/test-standalone.toml` — Linux.
- `test/test-standalone-windows.toml` — Windows.

The `nanvix-test` utility (`src/utils/nanvix-test/`) drives
these tests in two execution modes:

- **HTTP Mode**: Programs invoked via HTTP requests to
  `nanvixd`.
- **Terminal Mode**: Programs invoked directly by `nanvixd`
  (native ELF only).

## Writing Unit Tests (Rust)

Tests never go inline in mainline source. Declare the module out of line next to the code under
test, and put the tests in the file that the compiler resolves for it (`src/foo/tests.rs` for
`src/foo.rs`; `src/tests.rs` for `lib.rs`, `main.rs` and `mod.rs` files):

```rust
// src/foo.rs
#[cfg(test)]
mod tests;
```

```rust
// src/foo/tests.rs
use super::*;

#[test]
fn test_example() {
    let result: u32 = some_function();
    assert_eq!(result, 42, "expected 42");
}
```

`use super::*` gives the test module access to the private items of its parent, so seams
(test-only code in mainline source) are rarely needed. A module whose name does not mark a test file
(for example `benchmarks`) needs `#[path = "<dir>/tests/<name>.rs"]` so that it lives in a `tests`
directory.

`./z build -- test-layout-check` enforces this layout (rules R1-R6 in `doc/test.md`). It fails on
inline `#[test]` functions or `mod tests { ... }` bodies, on test-only code that is not in a test
file, on executable doc-tests, and on misplaced Python/C tests and e2e manifests. Existing
violations are recorded in `scripts/test-layout-baseline.txt`; never add to it. To move legacy inline
modules, use `scripts/migrate-inline-tests.py` (see `doc/test.md`, "Migrating Inline Tests").

Changing an existing test (editing, removing or renaming it, or growing the layout baseline or
allowlist) is flagged on pull requests: the `Test Change Review` workflow adds the
`review::tests-modified` label and posts a comment, and `scripts/flag-test-changes.py --base
origin/dev` previews it locally. Adding tests and moving tests unchanged is not flagged. Do not
weaken or delete a test to make a change pass; when a change to an existing test is justified,
explain why in the pull request (see `doc/test.md`, "Flagging Changes to Existing Tests").

Rules for tests:

- `expect()` is preferred over `unwrap()` for better failure diagnostics.
- Use `#[allow(clippy::unwrap_used)]` or `#[allow(clippy::expect_used)]` at the module level when
  needed (the crate uses `#![deny(...)]`).
- Provide descriptive assertion messages.
- Keep tests focused on a single behavior.

## Writing Guest Integration Tests (Rust)

Guest integration tests are `#![no_std]` binaries that run inside Nanvix:

1. Create directory at `src/tests/integration/<name>/` (or
   `src/tests/stress/<name>/` for stress tests).
2. Structure similar to guest daemons
   (`#![no_std]`, `#![no_main]`).
3. Add to `ALL_GUEST_TESTS` in the `Makefile`.
4. Add to workspace `members` in root `Cargo.toml`.

## Running All Tests

```bash
# Unit tests + system tests (if applicable).
./z build -- test
# Full CI pipeline (includes all test types).
./scripts/pipeline.sh
```
