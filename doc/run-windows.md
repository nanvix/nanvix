# Running Nanvix (Windows)

> **Prerequisite:** You must build Nanvix before running it. See [build-windows.md](build-windows.md)
for instructions.

On Windows, `nanvixd` supports **standalone interactive mode** only (no HTTP mode). Both
`nanvixd` and the UserVM are built natively with the WHP (Windows Hypervisor Platform) backend.

## Quick Start

Run a guest application via `nanvixd` in standalone interactive mode:

```powershell
.\bin\nanvixd.exe -- .\bin\hello-rust-nostd.elf
```

## Table of Contents

- [Quick Start](#quick-start)
- [Using `z.ps1 run`](#using-zps1-run)
- [Running nanvixd Directly](#running-nanvixd-directly)
- [Logging](#logging)
- [Expert Mode: Standalone UserVM](#expert-mode-standalone-uservm)
- [Benchmarking](#benchmarking)
  - [Quick Start](#quick-start-1)

---

## Using `z.ps1 run`

The simplest way to run on Windows:

```powershell
.\z.ps1 run
```

This launches `nanvixd.exe` with the default guest binary (`bin/hello-rust-nostd.elf`). You can
override it:

```powershell
.\z.ps1 run -- -program bin\hello-rust-nostd.elf
```

| Option              | Default                       | Description                |
| ------------------- | ----------------------------- | -------------------------- |
| `-program <path>`   | `bin/hello-rust-nostd.elf`    | Path to the guest binary.  |

## Running nanvixd Directly

You can also invoke `nanvixd.exe` directly:

```powershell
.\bin\nanvixd.exe -- .\bin\hello-rust-nostd.elf
```

### Enabling Host Networking

By default, networking system calls from the guest are blocked. To allow the guest to access the
host network stack, pass `-allow-host-networking`:

```powershell
.\bin\nanvixd.exe -allow-host-networking -- .\bin\network-rust.elf
```

Everything after `--` is forwarded to the application as arguments:

```powershell
.\bin\nanvixd.exe -- .\bin\echo-rust-nostd.elf arg1 arg2
```

Arguments, environment variables, and kernel arguments are packed into a single string separated
by `;`. The format is `<app args>;<env vars>;<kernel args>`:

- Everything before the first unescaped `;` becomes command-line arguments.
- Everything between the first and second unescaped `;` becomes environment variables as
  space-separated `KEY=VALUE` pairs.
- Everything after the second unescaped `;` becomes kernel arguments — a space-separated string
  that the kernel uses to enable/disable internal features.

Use an empty string when neither is needed. To pass only environment variables, start the string
with `;`. To pass only kernel arguments, use `;;`:

```powershell
# Arguments and environment variables.
.\bin\nanvixd.exe -- .\bin\echo-rust-nostd.elf "arg1 arg2;VAR1=foo VAR2=bar"

# Environment variables only.
.\bin\nanvixd.exe -- .\bin\echo-rust-nostd.elf ";VAR1=foo"

# All three components.
.\bin\nanvixd.exe -- .\bin\echo-rust-nostd.elf "arg1 arg2;VAR1=foo;feature1 feature2"

# Kernel arguments only.
.\bin\nanvixd.exe -- .\bin\echo-rust-nostd.elf ";;feature1 feature2"
```

To include a literal `;` in any section, escape it as `\;`:

```powershell
# Argument containing a literal semicolon.
.\bin\nanvixd.exe -- .\bin\echo-rust-nostd.elf "arg1 with\;semicolon arg2;VAR1=foo"
# args: ["arg1", "with;semicolon", "arg2"]   env: ["VAR1=foo"]   kernel_args: []
```

## Logging

`nanvixd` uses the `RUST_LOG` environment variable for daemon-level logging (printed to stderr):

```powershell
$env:RUST_LOG = "debug"
.\bin\nanvixd.exe -- .\bin\hello-rust-nostd.elf
```

## Expert Mode: Standalone UserVM

> **Warning:** This is an expert-level feature intended for low-level debugging and kernel
> development. Most users should use `nanvixd` instead (see [Running nanvixd Directly](#running-nanvixd-directly)).

For low-level debugging, you can bypass `nanvixd` and run the UserVM directly:

```powershell
.\bin\uservm.exe -kernel .\bin\kernel.elf -initrd .\bin\hello-rust-nostd.elf -standalone
```

## Benchmarking

Nanvix ships with `nanvix-bench`, a benchmarking tool that measures system performance. On Windows,
it supports standalone-mode benchmarks. See [benchmark.md](benchmark.md) for full details.

### Quick Start

```powershell
# Build with release settings.
.\z.ps1 build -- all RELEASE=yes LOG_LEVEL=panic

# Run the cold-start benchmark.
.\z.ps1 bench -- -benchmark cold-start -iterations 10

# Run the boot-time benchmark.
.\z.ps1 bench -- -benchmark boot-time -iterations 100

# See all options.
.\bin\nanvix-bench.exe -help
```
