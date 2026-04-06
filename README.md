# Nanvix

[![Join us on Slack!](https://img.shields.io/badge/chat-on%20Slack-e01563.svg)](https://join.slack.com/t/nanvix/shared_invite/zt-1yu30bs28-nsNmw8IwCyh6MBBV~B~X7w)
![GitHub commit activity](https://img.shields.io/github/commit-activity/m/nanvix/nanvix)
![GitHub last commit](https://img.shields.io/github/last-commit/nanvix/nanvix)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/nanvix/nanvix/ci.yml?branch=dev&label=tests)

Nanvix is a microkernel-based research operating system.

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
