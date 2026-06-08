---
name: kernel-development
description: Guide for modifying and debugging the Nanvix kernel architecture and kernel-call paths. Use this when asked about kernel internals or kernel implementation changes.
---

# Kernel Development

Use this skill when the user asks about developing, modifying, or debugging the Nanvix microkernel.
The kernel is a `#![no_std]` Rust binary targeting x86 (32-bit) and lives in `src/kernel/`.

## Architecture Overview

The kernel is a freestanding binary (`#![no_std]`,
`#![no_main]`) that runs in ring 0 on x86. Entry point is
`src/kernel/src/kmain.rs`.

### Key Modules

| Module   | Path                      | Purpose               |
|----------|---------------------------|-----------------------|
| `hal`    | `src/kernel/src/hal/`     | Hardware Abstraction. |
| `mm`     | `src/kernel/src/mm/`      | Memory Management.    |
| `pm`     | `src/kernel/src/pm/`      | Process Management.   |
| `ipc`    | `src/kernel/src/ipc/`     | IPC (mailboxes).      |
| `event`  | `src/kernel/src/event/`   | Event subsystem.      |
| `io`     | `src/kernel/src/io/`      | I/O subsystem.        |
| `kcall`  | `src/kernel/src/kcall/`   | Kernel call dispatch. |
| `kargs`  | `src/kernel/src/kargs.rs` | Boot args parsing.    |
| `kimage` | `src/kernel/src/kimage.rs`| Kernel image mgmt.    |
| `klog`   | `src/kernel/src/klog.rs`  | Kernel logging.       |
| `kpanic` | `src/kernel/src/kpanic.rs`| Panic handler.        |
| `uart`   | `src/kernel/src/uart.rs`  | UART serial driver.   |

### Machine Configurations

The kernel supports multiple machine types via Cargo
feature flags:

- `microvm` (default) — Lightweight VM for KVM.

### Virtual Memory Layout

- `KERNEL_BASE_RAW` to `KPOOL_BASE_RAW` — Kernel binary.
- `KPOOL_BASE_RAW` to `USER_BASE_RAW` — Kernel pool.
- `USER_BASE_RAW` to `USER_MMAP_BASE_RAW` — User binary.
- `USER_MMAP_BASE_RAW` to `USER_LIBS_BASE_RAW` — Mapped.
- `USER_LIBS_BASE_RAW` to `USER_HEAP_BASE_RAW` — Libs.
- `USER_HEAP_BASE_RAW` to `USER_HEAP_END_RAW` — Heap.
- `USER_STACK_TOP_RAW` to `USER_STACK_BASE_RAW` — Stack.

## Building the Kernel

```bash
./z build -- kernel
```

The kernel uses a custom Cargo target
(`build/targets/x86-kernel.json`) and custom linker scripts
(`build/kernel/`). Kernel builds disable sccache to avoid
issues with `compiler_builtins`.

## Key Dependencies

The kernel depends on workspace libraries:

- `arch` — Architecture abstractions.
- `bitmap` — Bitmap data structure.
- `config` — Compile-time configuration.
- `type-safe` — Type-safe wrappers.
- `raw-array` — Fixed-size arrays.
- `slab` — Slab allocator.
- `sys` — System-level types (kcall, IPC, MM, PM).

## Coding Rules (Kernel-Specific)

- All kernel code is `#![no_std]` — only `core` and `alloc`
  are available.
- No `panic!`, `unwrap()`, or `expect()` in production
  kernel code.
- Use explicit type annotations everywhere.
- Prefix imports with `::`.
- Minimize `unsafe` blocks and document safety invariants.
- Log errors with `error!` macro before returning `Err`.
- Use kernel-specific Cargo commands:
  `KERNEL_CARGO_BUILD_CMD`.
- Kernel configuration is in `build/kernel_config.toml`.

## Kernel Calls (System Calls)

Kernel calls are dispatched through
`src/kernel/src/kcall/dispatcher.rs`. Each subsystem (PM,
MM, IPC, Event, IO) registers its own kernel call handlers
in its respective `kcall/` subdirectory.
