---
name: library-development
description: Guide for creating and modifying Nanvix libraries under src/libs, including guest no_std and host std crates. Use this when asked about library architecture or crate changes.
---

# Library Development

Use this skill when the user asks about developing, modifying, or adding system libraries in Nanvix.
All libraries live under `src/libs/` and are part of the Cargo workspace.

## Library Categories

### Core Libraries (Guest — `#![no_std]`)

These run inside the Nanvix guest (user-space) and are compiled with `cargo`:

| Library         | Path                      | Purpose            |
|-----------------|---------------------------|--------------------|
| `arch`          | `src/libs/arch/`          | Arch abstractions. |
| `config`        | `src/libs/config/`        | Config constants.  |
| `error`         | `src/libs/error/`         | Error types.       |
| `sys`           | `src/libs/sys/`           | System types.      |
| `syscall`       | `src/libs/syscall/`       | Syscall interface. |
| `sysapi`        | `src/libs/sysapi/`        | System API.        |
| `sysalloc`      | `src/libs/sysalloc/`      | Guest allocator.   |
| `syslog`        | `src/libs/syslog/`        | Guest logging.     |
| `syslog-macros` | `src/libs/syslog-macros/` | Log macros.        |
| `nvx`           | `src/libs/nvx/`           | High-level API.    |
| `posix`         | `src/libs/posix/`         | POSIX layer.       |
| `proc`          | `src/libs/proc/`          | Process types.     |
| `type-safe`     | `src/libs/type-safe/`     | Type-safe values.  |

### Utility Libraries (Guest — `#![no_std]`)

| Library         | Path                        | Purpose         |
|-----------------|-----------------------------|-----------------|
| `bitmap`        | `src/libs/bitmap/`          | Bitmap.         |
| `slab`          | `src/libs/slab/`            | Slab allocator. |
| `raw-array`     | `src/libs/raw-array/`       | Fixed arrays.   |
| `static_assert` | `src/libs/static_assert/`   | Compile checks. |
| `elf`           | `src/libs/elf/`             | ELF parser.     |
| `libc_stdlib`   | `src/libs/libc_stdlib/`     | C stdlib.       |
| `libc_string`   | `src/libs/libc_string/`     | C strings.      |
| `no_fail`       | `src/libs/no_fail/`         | No-fail alloc.  |

### Host Libraries (Host — `std` available)

These run on the host system and are compiled with the host Rust build command
(`HOST_CARGO_BUILD_CMD`, currently `cargo`):

| Library                | Path                             | Purpose      |
|------------------------|----------------------------------|--------------|
| `nanvix`               | `src/libs/nanvix/`               | Host API.    |
| `nanvix-http`          | `src/libs/nanvix-http/`          | HTTP.        |
| `nanvix-sandbox`       | `src/libs/nanvix-sandbox/`       | Sandbox.     |
| `nanvix-terminal`      | `src/libs/nanvix-terminal/`      | Terminal.    |
| `control-plane-api`    | `src/libs/control-plane-api/`    | Control API. |
| `hwloc`                | `src/libs/hwloc/`                | HW topology. |
| `profiler`             | `src/libs/profiler/`             | Profiling.   |
| `syscomm`              | `src/libs/syscomm/`              | Sockets.     |
| `user-vm-api`          | `src/libs/user-vm-api/`          | User VM API. |

### Build Utilities

| Library       | Path                    | Purpose        |
|---------------|-------------------------|----------------|
| `build-utils` | `src/libs/build-utils/` | Build helpers. |

## Creating a New Library

1. Create the directory under `src/libs/<name>/`.

2. Add a `Cargo.toml` with workspace package metadata:

   ```toml
   [package]
   name = "<name>"
   version.workspace = true
   license-file.workspace = true
   authors.workspace = true
   edition.workspace = true

   [dependencies]
   # Add dependencies here.

   [features]
   default = []
   std = []
   ```

3. Add the crate to the workspace `members` list in the root `Cargo.toml`.

4. Add the crate to the workspace `[workspace.dependencies]` section.

5. Add the library name to the appropriate list in the
   `Makefile`:
   - Guest: `ALL_GUEST_RUST_LIBS` (and optionally `ALL_GUEST_RUST_LIBS_TEST_LIST`).
   - Host: `ALL_HOST_RUST_LIBS`.

6. Add the copyright header to all source files.

7. Create `src/lib.rs` with proper module organization
   (see coding standards).

## Building Libraries

```bash
# Build all guest libraries.
./z build -- all

# Run unit tests for host libraries.
./z build -- run-unit-tests
```

Guest libraries use `GUEST_CARGO_BUILD_CMD` (cross-compiled with `cargo`). Host
libraries use `HOST_CARGO_BUILD_CMD`.

## Windows Compilation

On a Windows development host, guest libraries are cross-compiled using a local toolchain, so
they work identically to Linux. Host libraries that use `std` may need conditional compilation
(`#[cfg(target_os = "...")]`) for platform-specific code paths (e.g., KVM, libc).

> **Note:** Windows CI jobs run as part of the main CI pipeline. Platform-independent crates
> should still be manually verified on a Windows host when making changes that may affect
> cross-platform compatibility or Windows-specific behavior.

## Coding Rules (Library-Specific)

- Guest libraries must support `#![no_std]` with optional `std` feature gate.
- Use `default-features = false` in workspace dependency declarations.
- Use `c_size_t`, `c_ssize_t`, `c_int`, etc. for C interoperability.
- Keep struct fields private; provide getter/setter methods.
- All public items must have doc comments with
  `# Description`, `# Parameters`, `# Returns`, and
  `# Errors` sections as applicable.
- Unit tests go in `#[cfg(test)]` modules; `expect()` is preferred over `unwrap()`.
