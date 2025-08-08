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

## Linting Commands

```bash
./z build --with-docker -- clippy
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
