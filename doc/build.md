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
