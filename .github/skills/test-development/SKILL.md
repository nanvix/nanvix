---
name: test-development
description: Guide for writing, running, and debugging Nanvix unit, integration, and system tests in Rust and C/C++. Use this when asked about test implementation or failures.
---

# Test Development

Use this skill when the user asks about writing, running, or debugging tests in Nanvix. Nanvix has
unit tests, integration tests, and system tests written in Rust and C/C++.

## Test Categories

### Unit Tests

Unit tests are `#[cfg(test)]` modules within library crates. They run on the host system using
`cargo test`.

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

System tests are available on `microvm` machines. On Linux, all deployment
modes are supported (`nanvixd` + kernel + guest). On Windows, only standalone mode is supported.

```bash
# Linux
./z build -- run-nanvix-tests
```

```powershell
# Windows (standalone mode only)
.\z.ps1 build -- run-nanvix-tests
```

Test configurations:

- `test/test-standalone.toml` — Standalone mode (Linux).
- `test/test-standalone-windows.toml` — Standalone mode (Windows).
- `test/test-single_process.toml` — Single-process mode.
- `test/test-multi_process.toml` — Multi-process mode.

The `nanvix-test` utility (`src/utils/nanvix-test/`) drives
these tests in two execution modes:

- **HTTP Mode**: Programs invoked via HTTP requests to
  `nanvixd`.
- **Terminal Mode**: Programs invoked directly by `nanvixd`
  (native ELF only).

## Writing Unit Tests (Rust)

Add `#[cfg(test)]` modules to library crates:

```rust
//============================================================
// Tests
//============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example() {
        let result: u32 = some_function();
        assert_eq!(result, 42, "expected 42");
    }
}
```

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
