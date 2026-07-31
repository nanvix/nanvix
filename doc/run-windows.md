# Running Nanvix (Windows)

> **Prerequisite:** You must build Nanvix before running it. See [build-windows.md](build-windows.md)
for instructions.

On Windows, `nanvixd` runs in **standalone single-tenant mode**. Both `nanvixd` and the UserVM
are built natively with the WHP (Windows Hypervisor Platform) backend. Standalone interactive
mode (stdio attached to the console) is the primary workflow; HTTP mode is also supported with
some limitations (see [HTTP Mode](#http-mode)).

Windows X64 runs X64 host binaries and x86/x86_64 guests. Windows ARM64 runs native ARM64 host
binaries and AArch64 guests. WHP does not provide cross-instruction-set emulation, so use
`TARGET=aarch64` on ARM64; `z.ps1` selects it automatically when no target is specified.

## Quick Start

Run a guest application via `nanvixd` in standalone interactive mode:

```powershell
.\bin\nanvixd.exe -- .\bin\hello-rust-nostd.elf
```

To make the architecture explicit on ARM64:

```powershell
.\z.ps1 build -- all TARGET=aarch64
.\bin\nanvixd.exe -- .\bin\hello-rust-nostd.elf
```

## Table of Contents

- [Quick Start](#quick-start)
- [Creating Multibinary Images](#creating-multibinary-images)
- [Using `z.ps1 run`](#using-zps1-run)
- [Running nanvixd Directly](#running-nanvixd-directly)
  - [Enabling Host Networking](#enabling-host-networking)
  - [Mounting a Host Directory](#mounting-a-host-directory)
  - [Passing Kernel Arguments](#passing-kernel-arguments)
- [HTTP Mode](#http-mode)
  - [Limitations vs. Linux HTTP Mode](#limitations-vs-linux-http-mode)
- [Logging](#logging)
- [Expert Mode: Direct UserVM](#expert-mode-direct-uservm)
  - [Recognised Kernel Arguments](#recognised-kernel-arguments)
- [Benchmarking](#benchmarking)
  - [Quick Start](#quick-start-1)

---

## Creating Multibinary Images

Guest applications run alongside system daemons (`procd`, `memd`, and `vfsd`) inside a single VM.
These components must be bundled into a **multibinary image** with `mkimage` before launch.

The `mkimage` tool takes an output path and a list of `<path>;<name>` pairs, where `<path>` is
the path to the ELF binary and `<name>` is the logical name the kernel uses to identify it at
boot:

```powershell
.\bin\mkimage.exe -o my-app.img `
    .\bin\procd.elf`;procd `
    .\bin\memd.elf`;memd `
    .\bin\vfsd.elf`;vfsd `
    .\bin\my-app.elf`;my-app
```

The three daemon binaries (`procd.elf`, `memd.elf`, `vfsd.elf`) are shipped in the release
archive under `bin/`. Your application binary must be compiled and linked against `libc.a`
using the `user.ld` linker script (both also in the release archive).

Once the image is created, pass it to `nanvixd` as the program argument:

```powershell
.\bin\nanvixd.exe -- .\my-app.img
```

> **Important:** The daemon order in the `mkimage` command line matters. Daemons are started in
> the order they appear, and `procd` must be listed first because other daemons depend on it.

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

| Option            | Default                    | Description               |
| ----------------- | -------------------------- | ------------------------- |
| `-program <path>` | `bin/hello-rust-nostd.elf` | Path to the guest binary. |

## Running nanvixd Directly

You can also invoke `nanvixd.exe` directly:

```powershell
.\bin\nanvixd.exe -- .\bin\hello-rust-nostd.elf
```

### Enabling Host Networking

By default, networking system calls from the guest are blocked. To allow the guest to access the
host network stack, pass `-allow-host-networking`:

```powershell
.\bin\nanvixd.exe -allow-host-networking -- .\bin\test-rust-network.elf
```

### Mounting a Host Directory

To make a host directory accessible to the guest at `/mnt`, use the `-mount` flag:

```powershell
.\bin\nanvixd.exe -mount C:\path\to\shared\dir -- .\bin\test-rust-file.elf
```

The guest can then read and write files under `/mnt/` which map to the host directory.
See [host-mount.md](host-mount.md) for the design and protocol details.

### Passing Kernel Arguments

To pass kernel arguments (written to guest control registers), use the `-kernel-args` flag:

```powershell
.\bin\nanvixd.exe -kernel-args snapshot -- .\bin\snapshot-rust-nostd.elf
```

See [Recognised Kernel Arguments](#recognised-kernel-arguments) below for available tokens.

Everything after `--` is forwarded to the application as arguments:

```powershell
.\bin\nanvixd.exe -- .\bin\echo-rust-nostd.elf arg1 arg2
```

Arguments and environment variables are packed into a single string separated
by `;`. The format is `<app args>;<env vars>`:

- Everything before the first unescaped `;` becomes command-line arguments.
- Everything after the first unescaped `;` becomes environment variables as
  space-separated `KEY=VALUE` pairs.

Use an empty string when neither is needed. To pass only environment variables, start the string
with `;`:

```powershell
# Arguments and environment variables.
.\bin\nanvixd.exe -- .\bin\echo-rust-nostd.elf "arg1 arg2;VAR1=foo VAR2=bar"

# Environment variables only.
.\bin\nanvixd.exe -- .\bin\echo-rust-nostd.elf ";VAR1=foo"
```

To include a literal `;` in any section, escape it as `\;`:

```powershell
# Argument containing a literal semicolon.
.\bin\nanvixd.exe -- .\bin\echo-rust-nostd.elf "arg1 with\;semicolon arg2;VAR1=foo"
# args: ["arg1", "with;semicolon", "arg2"]   env: ["VAR1=foo"]
```

> **Note:** Kernel arguments can be passed via `-kernel-args` on `nanvixd` (see
> [Passing Kernel Arguments](#passing-kernel-arguments)) or directly on the UserVM (see
> [Expert Mode: Direct UserVM](#expert-mode-direct-uservm)). They are not embedded in
> the initrd arguments string.

## HTTP Mode

`nanvixd` can also expose its REST control plane over HTTP on Windows. Launch it with
`-http-addr <addr>:<port>`:

```powershell
.\bin\nanvixd.exe -http-addr 127.0.0.1:8080
```

Clients then create, drive, and tear down User VMs through the REST API (this is the mode the
`nanvix-test` HTTP executor uses). Guest stdio is bridged through a per-process **named pipe**
gateway (`\\.\pipe\nanvix-standalone-gw-<pid>`) rather than the Unix-domain socket used on Linux.

### Limitations vs. Linux HTTP Mode

HTTP mode on Windows is intended for **single-tenant standalone** deployments. It differs from the
Linux implementation in the following ways:

- **Named-pipe gateway.** The guest stdio gateway is a Windows named pipe instead of a
  Unix-domain socket.
- **Emulated stdin half-close.** Unix-domain sockets half-close the write direction to signal
  end-of-input (EOF) to the guest's stdin while keeping the output direction open. Windows named
  pipes have no half-close primitive, so the gateway emulates one with a small in-band framing on
  the input (consumer → guest stdin) direction: each record is a little-endian `u32` length
  followed by that many payload bytes, and a zero-length record marks EOF. The daemon-side bridge
  closes the guest's stdin when it reads the EOF record, while the pipe stays open for guest
  output. Because the EOF record shares the pipe's FIFO ordering with the preceding data, stdin
  bytes always reach the guest first, so stdin-driven workloads (e.g. `echo`) behave the same as on
  Linux. The framing is internal to the gateway transport; the guest stdout direction remains a raw
  byte stream.

## Logging

`nanvixd` uses the `RUST_LOG` environment variable for daemon-level logging (printed to stderr):

```powershell
$env:RUST_LOG = "debug"
.\bin\nanvixd.exe -- .\bin\hello-rust-nostd.elf
```

By default, `nanvixd`'s own structured (`logrus`) records are written to an auto-named file
(`nanvixd_<timestamp>.log`) inside the log directory (overridable via `-log-dir <dir>`). Pass
`-log-to-stdout` to route those records to stdout instead:

```powershell
.\bin\nanvixd.exe -log-to-stdout -- .\bin\hello-rust-nostd.elf
```

This is useful when a parent process captures `nanvixd`'s stdout and forwards it to its own
log sink. `-log-to-stdout` and `-log-dir` are mutually exclusive.

## Expert Mode: Direct UserVM

> **Warning:** This is an expert-level feature intended for low-level debugging and kernel
> development. Most users should use `nanvixd` instead (see [Running nanvixd Directly](#running-nanvixd-directly)).

For low-level debugging, you can bypass `nanvixd` and run the UserVM directly:

```powershell
.\bin\uservm.exe -kernel .\bin\kernel.elf -initrd .\bin\hello-rust-nostd.elf
```

Optional flags:

| Flag                  | Description                                                      |
| --------------------- | ---------------------------------------------------------------- |
| `-stderr <file>`      | Redirect guest stderr to a file instead of host stderr.          |
| `-initrd_args <args>` | Arguments forwarded to the initrd payload.                       |
| `-kernel-args <args>` | Kernel arguments written to guest control registers (see below). |
| `-ramfs <file>`       | Path to a RAM filesystem image exposed to the guest.             |
| `-user-vm-id <id>`    | VM identifier (defaults to `0`).                                 |
| `-log-to-file`        | Write logs to files instead of stdout.                           |
| `-log-dir <dir>`      | Directory for log files (used with `-log-to-file`).              |

### Recognised Kernel Arguments

The `-kernel-args` flag accepts a space-separated list of tokens:

| Token      | Description                                                                     |
| ---------- | ------------------------------------------------------------------------------- |
| `snapshot` | Allow the guest to take exactly one VM snapshot via the `snapshot` kernel call. |

Example:

```powershell
.\bin\uservm.exe -kernel .\bin\kernel.elf `
  -initrd .\bin\snapshot-rust-nostd.elf `
    -kernel-args snapshot
```

Enable verbose logging with `RUST_LOG`:

```powershell
$env:RUST_LOG="trace"; .\bin\uservm.exe -kernel .\bin\kernel.elf -initrd .\bin\hello-rust-nostd.elf
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
