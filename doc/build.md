# Building Nanvix

> ℹ️ The instructions in this document assume that you have a system with the development
environment already set up. For more information on how to set up your development environment,
please refer to the [setup](setup.md) guide.

This document guides you through building Nanvix. Choose the guide that matches your development
environment:

- **[Linux Build](build-linux.md)** — Full build workflow on Linux Ubuntu 24.04. All components are
  built natively or via Docker. Uses `./z` as the build utility.

- **[Windows Build](build-windows.md)** — Full build workflow on Windows 11. Host components are
  built natively; guest components are cross-compiled using a local toolchain. Uses `z.ps1` as the
  build utility.

## Rust Thread-Local Storage

The static `x86-user` and `x86_64-user` targets support compiler-level TLS in `no_std` binaries.
Enable the nightly `thread_local` feature and annotate statically initialized variables with
`#[thread_local]`. Both initialized and zero-filled TLS are copied into each thread's data area.
The guest linker scripts collect the compiler's `.tdata`, `.tbss`, and named subsections into
that initialization template.

Use the normal `nvx-crt0` startup path for main-thread initialization and
`syscall::pthread::pthread_create()` for workers. Raw kernel threads created without a thread
data area must not access compiler TLS. The `#[thread_local]` attribute does not provide automatic
destructor registration. TLS in dynamically loaded libraries is not supported; the separate
`x86_64-user-pic` target continues to leave TLS support disabled.

The `test-rust-thread` integration suite checks initialization, per-thread isolation, preservation
across scheduling, and initialization of replacement threads.

## Switching Build Configurations

The shared `bin/` directory tracks the last image build configuration in `.last-build-config`:
`TARGET`, the Cargo build mode (including release profiling), `LOG_LEVEL`, and `MEMORY_SIZE`.
Changing any of these invalidates standalone `.initrd` images, POSIX C test executables, and
POSIX RAMFS images so that timestamp-preserving staging cannot reuse images from another
configuration. Returning to a previously built configuration also rebuilds these outputs;
repeating the same configuration preserves incremental image builds.

The first build after upgrading from the old target-only guard invalidates these outputs once.
The obsolete `.last-build-target` file is ignored. Do not run different configurations concurrently
in the same worktree, because staging paths are shared.
