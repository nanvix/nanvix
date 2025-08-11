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
- **Runtimes:** Python 3.12.3, Libstdc++ v3, Newlib 4.4.0, WebAssembly
- **Build System:** Bash Scripts + Make + Cargo (Rust) with custom toolchain

## Repository Structure

- **`.githook/`** - GitHub configuration files
- **`.github/`** - GitHub Actions workflows and configuration
- **`bin/`** - Built binaries
- **`lib/`** - Built libraries
- **`logs/`** - Log files
- **`build/`** - Build system configuration files
- **`doc/`** - Documentation files
- **`scripts/`** - Utility scripts for setup, testing, and building
- **`src/`** - Source code
- **`sysroot-debug/`** - System root for debug builds
- **`sysroot-release/`** - System root for release builds
- **`target/`** - Build cache and artifacts

### Source Code Structure

- **`src/benchmarks/`** - Performance benchmarks
- **`src/daemons/`** - System services (linuxd, nanvixd, wasmd, memd, procd)
- **`src/kernel/`** - Microkernel implementation (Rust)
- **`src/libs/`** - System libraries (arch, config, syscall, etc.)
- **`src/microvm/`** - MicroVM implementation
- **`src/tests/`** - Integration tests
- **`src/user/`** - User-space applications
- **`src/utils/`** - Utility programs

## Build Commands

```bash
./z build --with-docker -- all
```

## Code Linting Commands

```bash
# Check for linting issues in the code.
./z build --with-docker -- lint-check
```

## Code Formatting Commands

```bash
# Check for code formatting issues.
./z build --with-docker -- format-check

# Fix code formatting issues.
./z build --with-docker -- format
```

## Testing Commands

### Unit Tests

```bash
./z build --with-docker -- run-unit-tests
```

### System tests

```bash
scripts/test-nanvixd.sh 127.0.0.1:8181 bin/hello-c.elf '' '[]' 'Hello, world from C!'
```

## Coding Standards

### Style & Formatting

- Always follow the existing code style.
- Code must pass formatting checks with `./z build --with-docker -- format-check`.
- Code must pass linting checks with `./z build --with-docker -- lint-check`.
- Constants must be defined at module/file scope; avoid magic numbers in code paths.
- Map errors to OS error codes consistently; prefer typed errors over ad-hoc strings.

#### Style & Formatting (Rust only)

- Do not use `panic!`, `unwrap()`, or `expect()`, instead return `Result<T, E>`.
- Avoid `unsafe` unless strictly necessary. When unavoidable, narrow its scope and document pre/post conditions.
- Always log errors with `error!` before returning an error.
- Use `warn!` log level for non-critical warnings that do not affect functionality.
- Use `info!` log level for informational messages that are not errors or warnings.
- Use `debug!` log level for debugging information that is for development purposes.
- Use `trace!` log level for tracing execution flow and detailed debugging information.

### Documentation & Comments

- Public modules, structures, classes, enumerations, types, functions, variables, constants must have doc comments.
- `TODO`/`FIXME` comments must link to GitHub issues (e.g., `TODO (#1234): rationale`).

## Coding Review Guidelines

- Ensure that coding standards are followed.
- Ensure that changes are minimal and focused.
- Ensure that new code is documented.
- Ensure that doc comments are updated when behavior changes.
- Ensure markdown files in the source tree and documentation in `doc/` are updated when behavior changes.
