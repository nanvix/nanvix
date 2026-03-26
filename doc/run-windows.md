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

- [Using `z.ps1 run`](#using-zps1-run)
- [Running nanvixd Directly](#running-nanvixd-directly)
- [Logging](#logging)
- [Expert Mode: Standalone UserVM](#expert-mode-standalone-uservm)

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

Everything after `--` is forwarded to the application as arguments:

```powershell
.\bin\nanvixd.exe -- .\bin\echo-rust-nostd.elf arg1 arg2
```

Arguments and environment variables are packed into a single string separated by `;`. Everything
before the semicolon becomes command-line arguments; everything after it becomes environment
variables as space-separated `KEY=VALUE` pairs (e.g., `"arg1 arg2;VAR1=foo VAR2=bar"`). Use an empty
string when neither is needed. To pass only environment variables, start the string with `;`:

```powershell
# Arguments and environment variables.
.\bin\nanvixd.exe -- .\bin\echo-rust-nostd.elf "arg1 arg2;VAR1=foo VAR2=bar"

# Environment variables only.
.\bin\nanvixd.exe -- .\bin\echo-rust-nostd.elf ";VAR1=foo"
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
