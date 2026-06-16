# Nanvix

[![Join us on Slack!](https://img.shields.io/badge/chat-on%20Slack-e01563.svg)](https://join.slack.com/t/nanvix/shared_invite/zt-1yu30bs28-nsNmw8IwCyh6MBBV~B~X7w)
![GitHub commit activity](https://img.shields.io/github/commit-activity/m/nanvix/nanvix)
![GitHub last commit](https://img.shields.io/github/last-commit/nanvix/nanvix)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/nanvix/nanvix/ci.yml?branch=dev&label=tests)

Nanvix is a sandboxing technology for running untrusted applications in a hardware-isolated
environment with minimal overhead.

> **Note**: This repository contains the implementation of the sandbox. If you are looking to use
> Nanvix, refer to [nanvix-python](https://github.com/nanvix/nanvix-python), a Python distribution
> of Nanvix.

## Overview

Nanvix features a **unique co-designed architecture** that combines two key innovations:

- A **purpose-built Micro-VM** that provides hardware-enforced isolation with minimal overhead. This
  Micro-VM is designed to be as lightweight as possible, exposing only a virtual processor and
  memory to the guest and stripping away all device emulation and other features that add latency.

- A **multikernel OS** that enables flexible system component placement and sharing. This design
  splits the OS across two kernels: a guest-side Microkernel that runs inside the Micro-VM and
  provides essential OS features; and a host-side Macro-Kernel that runs on the host platform and
  provides network and storage software switches. System components run as special processes on top
  of either kernel, depending on the deployment configuration.

Read more about this architecture in our paper: [Nanvix: A Multikernel OS Design for High-Density
Serverless Deployments](https://arxiv.org/abs/2604.11669).

```plain
   ┌─────────────────────────────────────┐
   │               Micro-VM              │   Microkernel on Micro-VM runs:
   │              Microkernel            │     • The untrusted application.
   │                                     │     • Essential system services (scheduler, etc.).
   │   ┌─────────────┐ ┌──────────────┐  │     • Optional in-guest components (e.g. filesystem).
   │   │ Application │ │  Filesystem  │  │
   │   └─────────────┘ └──────────────┘  │
   └──────────────────┬──────────────────┘
                      │  selected I/O
                      │  (e.g. network)
                      ▼
   ┌─────────────────────────────────────┐
   │             Macro-Kernel            │   Macro-Kernel on host platform runs:
   │              (host OS)              │     • Network software switch.
   │                                     │     • Storage software switch.
   │           ┌──────────────┐          │     • Host-side system components (e.g. network stack).
   │           │   Network    │          │
   │           └──────────────┘          │
   └─────────────────────────────────────┘
```

### Key Features

- **Lightweight Virtual Machine**: Hardware isolation with minimal overhead — no device
  emulation, just a virtual processor and memory.
- **Custom Guest Microkernel**: A thin kernel exposing a rich feature set and POSIX API to support a
  wide range of applications (see our software [catalog](https://github.com/nanvix/catalog)).
- **Flexible Placement of System Components**: Components like filesystem and networking stack can
  run either on the guest-side or the host-side.
- **Cross-Platform Support**: Runs on both Linux and Windows hosts.
- **Fast Startup Times**: Application startup in the double-digit millisecond range, suitable for
  serverless and agentic workloads.
- **Low Memory Footprint**: Tens of megabytes of memory consumption per Micro-VM, friendly to
  resource-constrained environments and high-density deployments.

### Quick Comparison

| Feature                      | Nanvix | Unikraft | Firecracker | gVisor  | Docker | WebAssembly |
| ---------------------------- | ------ | -------- | ----------- | ------- | ------ | ----------- |
| Flexible Component Placement | ✅     | ❌       | ❌          | ❌      | ❌     | ❌          |
| Multiple Guest Processes     | ✅     | ❌       | ✅          | ✅      | ✅     | ❌          |
| Hardware Isolation           | ✅     | ✅       | ✅          | ❌      | ❌     | ❌          |
| Startup Time                 | ~30ms  | ~320ms   | ~70ms       | ~160ms  | ~1s    | ~1ms        |
| Memory Footprint             | ~10MB  | ~10MB    | 10–100MB    | 10–50MB | ~100MB | ~1MB        |
| Deployment Density           | ~10k   | ~1k      | ~1k         | ~1k     | ~100   | ~10k        |
| Full Linux Compatibility     | ❌     | ❌       | ✅          | ✅      | ✅     | ❌          |

> Startup time is the p50 latency to serve the first HTTP echo request; memory footprint is the
> per-instance contribution to host memory for the same workload, measured via `MemAvailable` in
> `/proc/meminfo`. Nanvix and Firecracker use snapshot-restore (the production serverless
> configuration); Unikraft and gVisor use cold start (no production snapshot path available). Ranges
> span cold boot and snapshot-restore variants where applicable. Docker and WebAssembly are
> approximate community values. See [arXiv:2604.11669](https://arxiv.org/abs/2604.11669) for more
> on the methodology.

### Trade-Offs

- **Linux Compatibility**: Nanvix provides a POSIX API with 150+ system calls, but it is not fully
  compatible with all Linux applications and may require some modifications to run certain
  applications.
- **Growing Software Ecosystem**: The catalog of ready-to-run applications is growing, so bringing
  up new workloads typically requires porting and cross-compiling them against the Nanvix toolchain.
  See the [catalog](https://github.com/nanvix/catalog) for a list of applications that have already
  been ported.

## Quick Start

### Linux

Requires Ubuntu 24.04 with sudo privileges and
[KVM](doc/setup-linux.md#4-setup-kvm) enabled.

```bash
# Run on Bash.

# Clone this source code.
git clone https://github.com/nanvix/nanvix.git && cd nanvix

# Setup the development environment.
./z setup

# Build Nanvix.
./z build -- all

# Run an example application.
./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/hello-rust-nostd.elf
```

### Windows

Requires Windows 11 with [GNU Make](doc/setup-windows.md#5-run-setup) on PATH, [Windows
Hypervisor Platform](doc/setup-windows.md#4-enable-windows-hypervisor-platform) enabled, [Developer
Mode](doc/setup-windows.md#2-enable-developer-mode) turned on, and a Rust toolchain installed via
[rustup](https://rustup.rs).

```powershell
# Run on PowerShell.

# Clone this source code (symlinks require Developer Mode).
git clone -c core.symlinks=true https://github.com/nanvix/nanvix.git; cd nanvix

# Setup the development environment.
.\z.ps1 setup

# Build Nanvix.
.\z.ps1 build -- all

# Run an example application.
.\bin\uservm.exe -kernel .\bin\kernel.elf -initrd .\bin\hello-rust-nostd.elf -standalone
```

> For more details, see the full [setup](doc/setup.md), [build](doc/build.md), and
> [run](doc/run.md) guides.

## Documentation

- [doc/setup.md](doc/setup.md) - Instructions for setting up your development environment.
- [doc/build.md](doc/build.md) - Instructions for building Nanvix.
- [doc/run.md](doc/run.md) - Instructions for running Nanvix.
- [doc/test.md](doc/test.md) - Instructions for testing Nanvix.
- [doc/benchmark.md](doc/benchmark.md) - Instructions for benchmarking Nanvix.

## Usage Statement

This project is a prototype. As such, we provide no guarantees that it will work and you are assuming any risks with using the code. We welcome comments and feedback. Please send any questions or comments to any of the following maintainers of the project:

- [Pedro Henrique Penna](https://github.com/ppenna) - [ppenna@microsoft.com](mailto:ppenna@microsoft.com)

> By sending feedback, you are consenting that it may be used in the further development of this project.

## License

This project is distributed under the [MIT License](LICENSE.txt).
