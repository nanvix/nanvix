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
./z build --with-cached-options -- run-unit-tests
```

Libraries with unit tests are listed in
`ALL_GUEST_RUST_LIBS_TEST_LIST`: `arch`, `bitmap`, `config`,
`elf`, `error`, `type-safe`, `proc`, `raw-array`, `slab`,
`static_assert`, `libc_string`, `syslog-macros`, `syslog`.

### Integration Tests (Guest — Rust)

| Test          | Path                      | Purpose            |
|---------------|---------------------------|--------------------|
| `testd`       | `src/tests/testd/`        | Guest test daemon  |
| `arch-rust`   | `src/tests/arch-rust/`    | Arch tests         |
| `file-rust`   | `src/tests/file-rust/`    | File tests         |
| `thread-rust` | `src/tests/thread-rust/`  | Thread tests       |
| `stress-rust` | `src/tests/stress-rust/`  | Stress tests       |
| `test-kernel` | `src/tests/test-kernel/`  | Kernel tests       |
| `linux-app`   | `src/tests/linux-app/`    | Linux app tests    |

### Integration Tests (Guest — C/C++)

| Test         | Path                     | Purpose            |
|--------------|--------------------------|--------------------|
| `file-c`     | `src/tests/file-c/`      | File tests         |
| `memory-c`   | `src/tests/memory-c/`    | Memory tests       |
| `network-c`  | `src/tests/network-c/`   | Network tests      |
| `thread-c`   | `src/tests/thread-c/`    | Thread tests       |
| `misc-c`     | `src/tests/misc-c/`      | Misc tests         |
| `dlfcn-c`    | `src/tests/dlfcn-c/`     | Dlfcn tests        |
| `c-bindings` | `src/tests/c-bindings/`  | C bindings tests   |

### System Integration Tests

System tests run the full Nanvix stack (`nanvixd` + kernel + guest) and are available only on
`microvm` and `hyperlight` machines.

```bash
./z build --with-cached-options -- run-nanvix-tests
```

Test configurations:

- `test/test-single_process.toml` — Single-process mode.
- `test/test-multi_process.toml` — Multi-process mode.
- `test/test-l2.toml` — L2 VM mode.

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

1. Create directory at `src/tests/<name>/`.
2. Structure similar to guest daemons
   (`#![no_std]`, `#![no_main]`).
3. Add to `ALL_GUEST_TESTS` in the `Makefile`.
4. Add to workspace `members` in root `Cargo.toml`.

## Writing C/C++ Tests

C/C++ tests use the Nanvix C toolchain (`i686-nanvix-gcc`/`i686-nanvix-g++`). They link against
`libposix.a` and the Newlib C library.  Tests follow a standalone Makefile pattern under
`src/tests/<name>/`.

## Running All Tests

```bash
# Unit tests + system tests (if applicable).
./z build --with-cached-options -- test
# Full CI pipeline (includes all test types).
./scripts/pipeline.sh
```
