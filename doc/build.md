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
