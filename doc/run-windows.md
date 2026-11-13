# Running Nanvix (Windows)

> **Prerequisite:** You must build Nanvix before running it. See [build-windows.md](build-windows.md)
for instructions.

On Windows, `nanvixd` runs in **standalone single-tenant mode**. Both `nanvixd` and the UserVM
are built natively with the WHP (Windows Hypervisor Platform) backend. Standalone interactive
mode (stdio attached to the console) is the primary workflow; HTTP mode is also supported with
some limitations (see [HTTP Mode](#http-mode)).

## Quick Start

Run a guest application via `nanvixd` in standalone interactive mode:

```powershell
.\bin\nanvixd.exe -- .\bin\hello-rust-nostd.elf
```

`nanvixd` loads the guest kernel (`kernel.elf`) from the directory that contains the `nanvixd.exe`
executable, such as `bin\` in a build tree or in a sysroot, regardless of the current working
directory. Pass `-bin-dir <dir>` to load it from a different directory.

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
- [Configuration File](#configuration-file)
  - [Pinning nanvixd to CPUs](#pinning-nanvixd-to-cpus)
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

#### Filtering Host Egress

With host networking enabled, `-allow-host <ip|cidr>` and `-block-host <ip|cidr>` restrict which
IPv4 destinations the guest may reach with `connect()` and `sendto()`. Both flags are repeatable,
accept a single address (`a.b.c.d`) or a CIDR block (`a.b.c.d/n`), and can be combined. Every rule
is validated up front: a malformed rule rejects the whole command line instead of being ignored.
Denied destinations fail with `EACCES`.

Each destination is evaluated with fixed precedence, independent of the order of the flags:

1. A destination matching any `-block-host` rule is denied.
2. Otherwise, a destination matching any `-allow-host` rule is permitted.
3. Otherwise, when any `-allow-host` rule is given, connections to DNS (port 53) are permitted so
   that name resolution keeps working.
4. Otherwise, the default action applies: deny when any `-allow-host` rule is given, allow
   otherwise.

| Posture                | Flags                                                    |
| ---------------------- | -------------------------------------------------------- |
| Deny all               | omit `-allow-host-networking`                            |
| Allow all              | `-allow-host-networking`                                 |
| Allowlist              | `-allow-host-networking -allow-host ...`                 |
| Blocklist              | `-allow-host-networking -block-host ...`                 |
| Allowlist with denials | `-allow-host-networking -allow-host ... -block-host ...` |

For example, to permit `10.0.0.0/8` except `10.0.0.1`:

```powershell
.\bin\nanvixd.exe -allow-host-networking -allow-host 10.0.0.0/8 -block-host 10.0.0.1 -- .\bin\test-rust-network.elf
```

While any filter is active, non-IPv4 destinations (for example, `AF_UNIX` sockets) are denied.

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

## Configuration File

Instead of passing every option on the command line, you can set options in a TOML file and pass
it to `nanvixd` with `-config <file>`:

```powershell
.\bin\nanvixd.exe -config C:\nanvix\nanvixd.toml -- .\bin\hello-rust-nostd.elf
```

Each key sets the command-line option of the same name, without the leading dash and with dashes
replaced by underscores. Every key is optional:

| Key                     | Type             | Command-Line Option                                            |
| ----------------------- | ---------------- | -------------------------------------------------------------- |
| `http_addr`             | String           | `-http-addr <addr>:<port>`                                     |
| `bin_dir`               | String (path)    | `-bin-dir <dir>`                                               |
| `console_file`          | String (path)    | `-console-file <file>`                                         |
| `ramfs`                 | String (path)    | `-ramfs <file>`                                                |
| `log_dir`               | String (path)    | `-log-dir <dir>`                                               |
| `log_to_stdout`         | Boolean          | `-log-to-stdout`                                               |
| `snapshot`              | String (path)    | `-snapshot <path>`                                             |
| `mount`                 | String (path)    | `-mount <host-dir>`                                            |
| `kernel_args`           | String           | `-kernel-args <args>`                                          |
| `allow_host_networking` | Boolean          | `-allow-host-networking`                                       |
| `allow_hosts`           | Array of strings | `-allow-host`, once per entry                                  |
| `block_hosts`           | Array of strings | `-block-host`, once per entry                                  |
| `gateway_sockaddr`      | String           | `-gateway-sockaddr <path>`                                     |
| `hwloc`                 | String (path)    | None (see [Pinning nanvixd to CPUs](#pinning-nanvixd-to-cpus)) |

For example, the following file starts `nanvixd` in HTTP mode with an egress allowlist. Write
Windows paths as TOML literal strings, in single quotes, so that backslashes are not treated as
escape sequences:

```toml
http_addr = "127.0.0.1:8080"
log_dir = 'C:\nanvix\logs'
allow_host_networking = true
allow_hosts = ["10.0.0.0/8"]
block_hosts = ["10.0.0.1"]
```

`nanvixd` checks the whole file at startup. It fails with an error that names the file if the file
is not valid TOML, has an unknown key or a value of the wrong type, or sets an invalid option, such
as a snapshot that does not exist. Relative paths are resolved against the directory that contains
the configuration file, not against the current working directory. The exception is
`gateway_sockaddr`, which must be a named pipe path, such as `\\.\pipe\nanvix-gw`, and is used as
is.

Options given on the command line take precedence over the configuration file, so you can refine a
shared file for each run:

- The operation mode and the log destination each count as one option. For example,
  `-- <program>` runs the program in interactive mode even if the file sets `http_addr`, and
  `-log-to-stdout` overrides a `log_dir` that the file sets.
- Host rules accumulate: the `allow_hosts` and `block_hosts` rules of the file apply together with
  the `-allow-host` and `-block-host` rules given on the command line.
- Flags that the file turns on, such as `allow_host_networking`, cannot be turned off on the
  command line.

### Pinning nanvixd to CPUs

The `hwloc` key names a hardware locality file (`hwloc.json`), whose `nanvixd_core_str` entry lists
the CPUs to which `nanvixd` pins itself on Linux:

```json
{
    "nanvixd_core_str": "10-19"
}
```

CPU pinning is not supported on Windows. `nanvixd` still checks the file and fails to start if it
has no `nanvixd_core_str` entry or if its CPU mask is malformed, but otherwise it logs a warning
and runs unpinned. See the Linux guide ([run-linux.md](run-linux.md#pinning-nanvixd-to-cpus)) for
the format of the file.

## Logging

`nanvixd` uses the `RUST_LOG` environment variable to set the level of its daemon-level logs:

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

In interactive mode, the stdout of `nanvixd` also carries the output of the guest, so the records
are interleaved with it. In HTTP mode, the output of the guest goes through the gateway instead.

## Expert Mode: Direct UserVM

> **Warning:** This is an expert-level feature intended for low-level debugging and kernel
> development. Most users should use `nanvixd` instead (see [Running nanvixd Directly](#running-nanvixd-directly)).

For low-level debugging, you can bypass `nanvixd` and run the UserVM directly:

```powershell
.\bin\uservm.exe -kernel .\bin\kernel.elf -initrd .\bin\hello-rust-nostd.elf
```

To debug the UserVM in Visual Studio Code, use the debug configurations described in
[setup-windows.md](setup-windows.md#debugging).

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
