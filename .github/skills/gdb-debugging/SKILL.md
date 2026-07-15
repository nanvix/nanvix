---
name: gdb-debugging
description: Guide for debugging Nanvix guests using GDB remote debugging over the microvm machine type. Use this when asked about attaching GDB, setting breakpoints, or stepping through kernel code.
---

# GDB Debugging Nanvix

Use this skill when the user asks about debugging the Nanvix kernel or guest applications using GDB.
Refer to `doc/gdb.md` for the user-facing documentation.

## Prerequisites

- Nanvix built with the `gdb` Cargo feature. At the crate level, `gdb` requires `microvm`.
- A GDB client with x86_64 target support (`gdb-multiarch` or `x86_64-elf-gdb`).

## Build

```bash
./z build -- all
```

## Launch

Through nanvixd (preferred):

```bash
RUST_LOG=info ./bin/nanvixd.elf -gdb-port 1234 -- ./bin/hello-rust-nostd.elf
```

Through uservm directly:

```bash
RUST_LOG=info ./bin/uservm.elf \
    -kernel ./bin/kernel.elf \
    -initrd ./bin/hello-rust-nostd.elf \
    -gdb-port 1234
```

The guest halts at its first instruction until a GDB client connects.

## Connect

```bash
gdb-multiarch ./bin/kernel.elf
(gdb) target remote :1234
(gdb) break _start
(gdb) continue
```

A ready-to-use `.gdbinit` exists at the repository root and is loaded automatically when GDB is
launched from the project directory.

## GDB Command Reference

| Operation           | Command               | Notes                                |
| ------------------- | --------------------- | ------------------------------------ |
| Read registers      | `info registers`      | All GPRs, RIP, RFLAGS, segments      |
| Write registers     | `set $rax = 0x42`     | Any general-purpose register         |
| Read memory         | `x/16xb 0x1000`       | Guest physical memory                |
| Write memory        | `set {int}0x1000 = 1` | Guest physical memory                |
| Software breakpoint | `break *0x1000`       | INT3-based                           |
| Remove breakpoint   | `delete 1`            | Restores original instruction byte   |
| Single step         | `stepi`               | One instruction                      |
| Continue            | `continue`            | Resume until next event              |
| Backtrace           | `bt`                  | Requires symbols                     |
| Disassemble         | `disas`               | At current RIP                       |

## Common Workflows

### Kernel crash

```gdb
(gdb) continue
# ... guest crashes ...
(gdb) bt
(gdb) info registers
(gdb) x/10i $rip
```

### Stepping through boot

```gdb
(gdb) stepi
(gdb) info registers
```

### Breaking on a function

```gdb
(gdb) break kmain
(gdb) continue
```

## Troubleshooting

| Problem                        | Cause / Fix                                              |
| ------------------------------ | -------------------------------------------------------- |
| GDB cannot connect             | Port in use (`ss -tlnp \| grep 1234`), or VM not started |
|                                 | with `-gdb-port`.                                        |
| "Remote connection closed"      | Guest exited before GDB connected. Check VM stderr.      |
| Breakpoints not hit             | Address mismatch — kernel uses identity mapping           |
|                                 | (GVA == GPA). Verify symbol file: `file ./bin/kernel.elf`.|
| Memory access returns zeros     | Address outside guest memory region                       |
|                                 | (`config::kernel::MEMORY_SIZE`).                         |

## Limitations

- **Linux only.** GDB debugging requires the `microvm` machine type with KVM, which is not
  available on Windows. The WHP backend does not expose a GDB server.
- **Physical addresses only.** The kernel uses identity mapping (GVA == GPA).
- **No hardware breakpoints.** Only software breakpoints (`INT3`).
- **Single vCPU.** Single-threaded debugging model.
- **Ctrl+C latency.** The break-in signal is processed at the next vCPU exit (e.g., port I/O), not
  asynchronously during pure guest computation. In practice this is near-instant because the Nanvix
  guest performs frequent I/O.

## Architecture Notes

- GDB server uses the `gdbstub` crate (v0.7), feature-gated behind `gdb`.
- Server runs synchronously in the vCPU thread (KVM register ioctls are thread-specific).
- Software breakpoints patch guest memory with `INT3` (0xCC); KVM delivers `VcpuExit::Debug`.
- At the crate level, `gdb` requires `microvm` through its feature dependency.
