# Copilot Instructions for Nanvix

This document provides essential guidelines for the Copilot coding agent to
effectively work with the Nanvix repository.

## Project Overview

Nanvix is a microkernel-based research operating system written primarily in
Rust and C/C++. It's a complex systems project that targets multiple machine
types and supports various runtimes.

### Key Technologies

- **Languages:** Rust (kernel, daemons, libraries), C/C++ (tests), Python and Shell (tooling)
- **Target Architecture:** x86 (32-bit)
- **Target Machines:** microvm (default), hyperlight, qemu-pc, qemu-isapc, qemu-baremetal
- **Runtimes:** Python 3.12.3, Libstdc++ v3, Newlib 4.4.0, WebAssembly (via QuickJS and wasmd)
- **Build System:** Bash Scripts + Make + Cargo (Rust) with custom toolchain
- **Development Tools:** Binutils v2.40, GCC v12.4.0, G++ v12.4.0, GFortran v12.4.0, Rustc v1.87.0
- **Supported Libraries:** OpenBLAS v0.3.29, OpenSSL v3.5.0, SQLite v3.49.0, Zlib v1.3.1

## Repository Structure

- **`.githook/`** - GitHub hooks
- **`.github/`** - GitHub Actions workflows and Copilot configuration
- **`bin/`** - Built binaries (output directory)
- **`lib/`** - Built libraries (output directory)
- **`logs/`** - Runtime log files
- **`build/`** - Build system configuration (linker scripts, Makefiles, TOML configs)
- **`doc/`** - Documentation files (setup, build, run, test, benchmark)
- **`scripts/`** - Utility scripts for setup, testing, building, and CI/CD
- **`src/`** - Source code (see detailed structure below)
- **`sysroot-debug/`** - System root for debug builds
- **`sysroot-release/`** - System root for release builds
- **`target/`** - Cargo build artifacts and intermediate files
- **`toolchain/`** - Custom cross-compilation toolchain (when built locally)

### Source Code Structure

- **`src/benchmarks/`** - Performance benchmarks (echo, noop variants in C, C++, Rust, WASM)
- **`src/daemons/`** - System services
  - `linuxd/` - Linux daemon for L2 VM deployment
  - `memd/` - Memory management daemon
  - `procd/` - Process management daemon
  - `wasmd/` - WebAssembly runtime daemon
- **`src/kernel/`** - Microkernel implementation (Rust, x86 architecture)
- **`src/libs/`** - System libraries
  - Core libraries: `arch/`, `config/`, `error/`, `sys/`, `syscall/`
  - Utilities: `bitmap/`, `slab/`, `raw-array/`, `profiler/`
  - High-level: `nanvix/`, `posix/`, `nvx/`
  - Specialized: `elf/`, `hwloc/`, `nanvix-http/`, `nanvix-registry/`, `nanvix-sandbox/`
- **`src/uservm/`** - UserVM implementation (orchestrates user process execution)
- **`src/tests/`** - Integration and system tests
  - Rust tests: `arch-rust/`, `file-rust/`, `testd/`
  - C tests: `file-c/`, `memory-c/`, `network-c/`, `thread-c/`, `misc-c/`, `dlfcn-c/`
  - Runtime tests: `quickjs/`, `linux-app/`
- **`src/user/`** - User-space example applications (hello-c, hello-cpp, hello-rust-nostd, etc.)
- **`src/utils/`** - Utility programs (nanvixd, nanvix-bench, echo-client)

## Building, Formatting, Linting, and Testing

Nanvix uses the `z` utility script to streamline building, formatting, linting, and testing.
All operations can be performed using Docker (recommended for consistency) or a local toolchain.

### Getting Started

To get started with the `z` utility, run:

```bash
./z help
```

### Build Commands

```bash
# Build with previous build parameters/options (recommended)
./z build --with-cached-options -- all

# Build with Docker
./z build --with-docker -- all

# Build with local toolchain
./z build -- all

# Build specific targets
./z build -- kernel          # Build kernel only
./z build -- all-nanvixd     # Build nanvixd only
./z build -- all-uservm      # Build uservm only
```

### Build Parameters

Key build parameters can be set as environment variables or via the `z` utility:

- `MACHINE` - Target machine: `microvm` (default), `hyperlight`, `qemu-pc`, `qemu-isapc`, `qemu-baremetal`
- `TARGET` - Target architecture: `x86` (only supported value)
- `RELEASE` - Release build: `yes` or `no` (default)
- `LOG_LEVEL` - Logging level: `trace`, `debug`, `info`, `warn` (default), `error`, `panic`
- `PROFILER` - Enable profiler: `yes` or `no` (default)
- `SINGLE_PROCESS` - Single-process deployment: `yes` (default) or `no`
- `L2_VM` - L2 VM deployment: `yes` or `no` (default)
- `BUILD_OPT` - Build optional software: `yes` (default) or `no`

Example:

```bash
./z build -- all MACHINE=hyperlight RELEASE=yes LOG_LEVEL=error
```

### Code Linting Commands

```bash
# Check for linting issues in the code
./z build --with-cached-options -- lint-check

# Fix code linting issues
./z build --with-cached-options -- lint
```

### Code Formatting Commands

```bash
# Check for code formatting issues
./z build --with-cached-options -- format-check

# Fix code formatting issues
./z build --with-cached-options -- format
```

### Spell Check Commands

```bash
# Check for spelling errors in source code and documentation
./z build --with-cached-options -- spellcheck

# Fix spelling errors in source code and documentation
./z build --with-cached-options -- spellcheck-fix
```

### Testing Commands

#### Unit Tests

```bash
# Run all unit tests
./z build --with-cached-options -- run-unit-tests
```

#### System Tests

System tests run against a live Nanvix system and come in two modes:

**HTTP Mode Tests** (supports all program types including WASM and interpreters):

```bash
./z build --with-cached-options -- run-nanvixd-http-tests
```

**Terminal Mode Tests** (native executables only):

```bash
./z build --with-cached-options -- run-nanvixd-terminal-tests
```

**Run specific test suites:**

```bash
# Run all system tests
make run-nanvixd-tests

# Run specific test categories
make run-nanvixd-http-tests-c-bindings
make run-nanvixd-http-tests-file
make run-nanvixd-http-tests-quickjs
```

#### Manual System Test

For manual testing or debugging:

```bash
scripts/test-nanvixd.sh 127.0.0.1:8181 bin/hello-c.elf '' '[]' 'Hello, world from C!'
```

#### Benchmarking

```bash
# Compile for benchmarking (requires specific build flags)
make all RELEASE=yes LOG_LEVEL=panic

# For echo-breakdown benchmark
make all RELEASE=yes LOG_LEVEL=panic TIMESTAMP_MSG=yes

# Run benchmarks
./bin/nanvix-bench.elf -benchmark cold-start
./bin/nanvix-bench.elf -benchmark warm-start
./bin/nanvix-bench.elf -benchmark warm-start-vmm
./bin/nanvix-bench.elf -benchmark echo-breakdown
```

### CI/CD Pipeline

To run the full CI pipeline locally:

```bash
./scripts/pipeline.sh
```

This runs formatting checks, linting, spell checking, building, and all tests.

## Coding Standards

### Style & Formatting

- Always follow the existing code style.
- Code must pass formatting checks with `./z build --with-cached-options -- format-check`.
- Code must pass linting checks with `./z build --with-cached-options -- lint-check`.
- Constants must be defined at module/file scope; avoid magic numbers in code paths.
- Map errors to OS error codes consistently; prefer typed errors over ad-hoc strings.
- Error messages should contain relevant information about the error and the context in which it occurred.
- Keep line width to 100 characters maximum (configured in `rustfmt.toml`).
- All files must include copyright header: `// Copyright(c) The Maintainers of Nanvix.\n// Licensed under the MIT License.`

#### Style & Formatting (Rust only)

- Do not use `panic!`, `unwrap()`, or `expect()`, instead return `Result<T, E>`.
- Avoid `unsafe` unless strictly necessary. When unavoidable, narrow its scope and document pre/post conditions.
- Always add explicit type annotation when defining variables and constants, even if type can be inferred (e.g., `let x: u32 = 42;`).
- Prefix all import statements with `::` (e.g., use `::std::fs` instead of `std::fs`).
- Always log errors with `error!` before returning an error.
- Use `warn!` log level for non-critical warnings that do not affect functionality.
- Use `info!` log level for informational messages that are not errors or warnings.
- Use `debug!` log level for debugging information that is for development purposes.
- Use `trace!` log level for tracing execution flow and detailed debugging information.
- Logs must be single-line, concise, and machine-parsable when feasible.
  - Do not use multiline logs or explicit newlines (e.g., `\n`) in messages.
  - Do not use pretty-printed debug formatting (e.g., `{:#?}`).
- Module organization:
  - Group imports at the top after the copyright header and configuration section.
  - Order sections: Configuration (`#![...]`), Imports, Modules, Constants, Structures, Enumerations, Trait Implementations, Standalone Functions, Tests.
  - Use section header comments with `//==================================================================================================`.

#### Style & Formatting (C/C++ only)

- Use consistent indentation and formatting as defined in the existing codebase.
- Include proper header guards in `.h` files.
- Follow Linux kernel style for C code where applicable.
- All function declarations must have parameter names in headers.
- Organize C files with section comments matching Rust style.

#### Style & Formatting (Python only)

- Follow PEP 8 style guidelines.
- Use type hints for function parameters and return values.
- Maximum line length: 100 characters.
- Use docstrings for all public functions and classes.

#### Style & Formatting (Shell scripts only)

- Use `#!/bin/bash` shebang for all shell scripts.
- Quote all variable expansions unless intentionally splitting words.
- Use `set -e` to exit on error where appropriate.
- Add descriptive comments for non-obvious operations.
- Keep functions small and focused.

### Documentation & Comments

- Public modules, structures, classes, enumerations, types, functions, variables, constants must have doc comments.
- Use triple-slash `///` for Rust doc comments.
- Doc comments should start with `# Description` section.
- Include `# Parameters` section for functions with parameters.
- Include `# Returns` section for functions that return values.
- Include `# Errors` section for functions that can fail.
- Include `# Safety` section for unsafe functions explaining invariants.
- `TODO`/`FIXME` comments must link to GitHub issues (e.g., `TODO (#1234): rationale`).
- Terminate all comments with a period.
- Avoid redundant comments that simply restate the code.
- Focus comments on *why* rather than *what* (code shows what, comments explain why).

## Coding Review Guidelines

- Ensure that coding standards are followed.
- Ensure that changes are minimal and focused.
- Ensure that new code is documented.
- Ensure that doc comments are updated when behavior changes.
- Ensure markdown files in the source tree and documentation in `doc/` are updated when behavior changes.
- Check for typos in comments and documentation.
- Check for arithmetic overflows.
- Check for potential resource leaks (e.g., file handles, memory).
- Check for potential deadlocks.
- Verify error handling is comprehensive and errors are logged appropriately.
- Check that all public APIs have documentation.
- Ensure tests are added or updated for new functionality.
- Verify that changes work across all supported target machines when applicable.
- Check that no hardcoded paths or machine-specific constants are introduced.
- Ensure copyright headers are present in all new files.

### Coding Review Guidelines (Rust only)

- Ensure `c_size_t` is used instead of `usize` for C interoperability.
- Ensure `c_ssize_t` is used instead of `isize` for C interoperability.
- Ensure `c_int`, `c_uint`, `c_long`, `c_ulong`, `c_short`, and `c_ushort` are used instead of their Rust counterparts for C interoperability.
- Member fields in `struct`s must be private and accessed via getter/setter methods.
- Verify that all imports use the `::` prefix convention.
- Check that error paths log before returning.
- Ensure no `unwrap()`, `expect()`, or `panic!()` are used.
- Verify unsafe blocks are minimized and properly documented.
- Check that explicit type annotations are used for variable declarations.

### Coding Review Guidelines (C/C++ only)

- Ensure proper memory management (no leaks, double-frees, or use-after-free).
- Check for buffer overflows and bounds checking.
- Verify that all pointers are validated before dereferencing.
- Ensure consistent error handling patterns.
- Check for proper initialization of variables.

### Coding Review Guidelines (Python only)

- Verify type hints are present and correct.
- Check for proper exception handling.
- Ensure resource cleanup (files, sockets) using context managers.
- Verify adherence to PEP 8.

### Coding Review Guidelines (Shell scripts only)

- Check for proper quoting of variables.
- Verify error handling with appropriate exit codes.
- Ensure scripts are idempotent where applicable.
- Check for shellcheck compliance.
