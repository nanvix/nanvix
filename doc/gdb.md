# GDB Remote Debugging

> **Prerequisite:** Build Nanvix with the `gdb` Cargo feature before debugging.

The microvm machine type exposes a GDB Remote Serial Protocol (RSP) server over TCP for
interactively debugging the Nanvix guest kernel.

## Table of Contents

- [Building for Debug](#building-for-debug)
- [Quick Start](#quick-start)
- [Using the Provided `.gdbinit`](#using-the-provided-gdbinit)
- [Supported Features](#supported-features)
- [Limitations](#limitations)

## Building for Debug

Build Nanvix, enabling the `gdb` feature for the host component you use to launch the VM:

```bash
./z build -- all
```

You also need a GDB client with x86_64 target support (`gdb-multiarch` or `x86_64-elf-gdb`).

## Quick Start

**1. Launch nanvixd with the GDB server:**

```bash
RUST_LOG=info ./bin/nanvixd.elf \
    -gdb-port 1234 \
    -- ./bin/hello-rust-nostd.elf
```

The VM pauses until a GDB client connects.

You can also launch `uservm` directly:

```bash
RUST_LOG=info ./bin/uservm.elf \
    -kernel ./bin/kernel.elf \
    -initrd ./bin/hello-rust-nostd.elf \
    -gdb-port 1234
```

**2. Connect GDB (in a second terminal):**

```bash
gdb-multiarch ./bin/kernel.elf
(gdb) target remote :1234
(gdb) break _start
(gdb) continue
```

## Using the Provided `.gdbinit`

A ready-to-use [`.gdbinit`](../.gdbinit) is included at the repository root. When you launch GDB
from the project directory it is loaded automatically:

```bash
gdb-multiarch
```

## Supported Features

| Feature              | Description                                     |
| -------------------- | ----------------------------------------------- |
| Register read/write  | All x86_64 GPRs, RIP, RFLAGS, segment registers |
| Memory read/write    | Guest physical memory (identity-mapped)         |
| Software breakpoints | `INT3`-based, unlimited count                   |
| Single-stepping      | `stepi` executes one guest instruction          |
| Continue             | Resumes guest execution                         |
| Ctrl+C break-in      | Interrupts running guest from GDB (see note)    |

## Limitations

- **MicroVM only.** At the crate level, the `gdb` feature requires `microvm`.
- **Physical addresses only.** Memory access uses guest physical addresses. The kernel uses identity
  mapping, so virtual and physical addresses match for kernel code.
- **No hardware breakpoints.** Only software breakpoints (`INT3`) are supported.
- **Single vCPU.** The GDB server supports single-threaded debugging only.
- **Ctrl+C latency.** The break-in signal is processed at the next vCPU exit (e.g., port I/O), not
  asynchronously during pure guest computation. In practice this is near-instant because the Nanvix
  guest performs frequent I/O.
