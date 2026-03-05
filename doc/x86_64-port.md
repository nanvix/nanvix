# Nanvix x86_64 Port

This document describes the port of Nanvix from x86 (32-bit) to x86_64 (64-bit long mode),
covering the design decisions, architecture, memory layout, and boot flow.

## Table of Contents

- [Overview](#overview)
- [Code Organization](#code-organization)
- [Build Infrastructure](#build-infrastructure)
- [Boot Flow](#boot-flow)
- [VMM Guest Setup](#vmm-guest-setup)
- [Kernel Architecture](#kernel-architecture)
  - [Global Descriptor Table](#global-descriptor-table)
  - [Interrupt Descriptor Table](#interrupt-descriptor-table)
  - [Task State Segment](#task-state-segment)
  - [Context Save and Restore](#context-save-and-restore)
  - [Paging](#paging)
  - [Kernel Calls](#kernel-calls)
  - [User-Mode Transition](#user-mode-transition)
- [Memory Layout](#memory-layout)
- [ELF64 Loading](#elf64-loading)
- [Library Changes](#library-changes)
- [Integration Testing](#integration-testing)
- [Clippy and Lint Fixes](#clippy-and-lint-fixes)
- [CI/CD Pipeline](#cicd-pipeline)
- [Known Limitations](#known-limitations)

## Overview

Nanvix runs as a guest under KVM on the microvm machine. The VMM (`uservm`) sets up a
virtual machine, loads the kernel and user binaries into guest physical memory, and boots
the guest directly in 64-bit long mode. The kernel initializes hardware, loads ELF64 user
programs, and transitions to user mode via `iretq`.

The x86_64 port reuses the existing x86 module namespace via a path alias:

```rust
#[cfg(target_arch = "x86_64")]
#[path = "x86_64/mod.rs"]
pub mod x86;
```

All existing `arch::x86::` references continue to work without modification. This avoids
a large-scale rename and keeps 32-bit and 64-bit code paths coexisting cleanly.

## Code Organization

The port minimizes modifications to existing x86 source files. New x86_64-specific code
lives in dedicated files, selected at compile time via `#[cfg(target_arch)]` and `#[path]`
attributes. This keeps the original x86 files unchanged and isolates all 64-bit additions.

### Isolation Pattern

Wherever an x86 type or function has a different x86_64 implementation, the pattern is:

```rust
// Original module compiles only on x86.
#[cfg(target_arch = "x86")]
pub mod idt;
// New file compiles only on x86_64.
#[cfg(target_arch = "x86_64")]
#[path = "idt_x86_64.rs"]
pub mod idt;
```

Both expose the same public API (e.g., `idt::Idte`), so downstream code works on both
architectures without `cfg` annotations.

### File Layout

New files introduced by the port (excluding the `hal/arch/x86_64/` kernel module, which
is entirely new):

| New File | Paired With | Contents |
|----------|-------------|----------|
| `src/libs/arch/src/x86/cpu/idt_x86_64.rs` | `idt.rs` | 16-byte IDT entry (`Idte`) for 64-bit gates |
| `src/libs/arch/src/x86/cpu/idtr_x86_64.rs` | `idtr.rs` | 10-byte IDTR with `u64` base, `lidt (%rax)` |
| `src/libs/arch/src/x86/cpu/tss_x86_64.rs` | `tss.rs` | 104-byte TSS with RSP0–RSP2, IST1–IST7 |
| `src/libs/arch/src/x86/mem/gdtr_x86_64.rs` | `gdtr.rs` | 10-byte GDTR with `u64` base, `lgdt (%rax)` |
| `src/libs/nvx/src/crt0_x86_64.rs` | inline asm in `lib.rs` | x86_64 `_do_start` process entry point |
| `src/libs/sys/src/sys/kcall/pm_x86_64.rs` | `pm.rs` | x86_64 `__start_thread` assembly stub |
| `src/uservm/src/elf64.rs` | `elf.rs` | `Elf64Fhdr`, `Elf64Phdr`, `memory_footprint_64()`, `load_64()` |
| `src/uservm/src/vmm/microvm/kvm/vcpu/reset64.rs` | `vcpu/mod.rs` | `reset_64bit()` for long-mode guest init |
| `src/kernel/src/mm/elf64.rs` | `mm/elf.rs` | `Elf64Fhdr`, `Elf64Phdr`, `do_elf64_load()`, `elf64_load()` |
| `src/kernel/src/hal/arch/x86_64/` (directory) | `hal/arch/x86/` | Full 64-bit kernel arch: GDT, IDT, TSS, MMU, context, hooks |

### Minimal Changes to Existing Files

The original x86 files (`idt.rs`, `idtr.rs`, `tss.rs`, `gdtr.rs`) are **unchanged**. Other
existing files have small, isolated additions:

- **Module routers** (`cpu/mod.rs`, `mem/mod.rs`): `#[cfg]` + `#[path]` lines to select
  the correct module variant per architecture.
- **ELF loaders** (`elf.rs` in kernel and uservm): `#[path] mod elf64` declaration, an
  `ElfClass` enum, and a thin dispatcher that checks `e_ident[EI_CLASS]` and delegates to
  the 32-bit or 64-bit loader.
- **Process manager** (`pm/process/manager/mod.rs`): `ElfClass` parameter in
  `create_process()` to route 32-bit vs 64-bit ELF loading.
- **vCPU module** (`vcpu/mod.rs`): `mod reset64` declaration and a `reset()` dispatcher
  that calls either `reset_32bit()` or `reset_64bit()` based on the target architecture.

## Build Infrastructure

### Target Specifications

Two custom target specs define the freestanding x86_64 environments:

| Target                    | Purpose                  | Key Settings                              |
|---------------------------|--------------------------|-------------------------------------------|
| `x86_64-kernel.json`     | Kernel (Ring 0)          | No SSE/AVX, soft-float, static linking    |
| `x86_64-user.json`       | User programs (Ring 3)   | No SSE/AVX, soft-float, static linking    |

Both use the `x86_64-unknown-none` base with `"os": "none"`, `"panic-strategy": "abort"`,
and disabled SIMD features to avoid FPU state management in the kernel.

### Building

```bash
# Build the 64-bit kernel.
./z build -- all-kernel TARGET=x86_64 MACHINE=microvm

# Build a 64-bit user binary.
./z build -- all-guest-binaries-hello-rust-nostd TARGET=x86_64 MACHINE=microvm

# Run it.
./bin/nanvixd.elf -- ./bin/hello-rust-nostd.elf
```

The 32-bit build remains the default and is unaffected:

```bash
./z build -- all-kernel TARGET=x86 MACHINE=microvm
```

### Linker Scripts

- **Kernel** (`build/kernel/linker/x86_64/kernel.ld.in`): Places `.text` at 0x100000 (1 MiB)
  with 2 MiB alignment for the kernel image. A template processed by `build.rs` to inject
  heap padding.
- **User** (`build/user/linker/x86_64/user.ld`): Places `.crt0` and `.text` at 0x40000000
  (1 GiB), which is the `USER_BASE` address.

## Boot Flow

```
nanvixd (host)
  └─ uservm (host, KVM VMM)
       ├─ Loads kernel.elf into guest RAM at 0x100000
       ├─ Loads hello-rust-nostd.elf as initrd at 0xC00000
       ├─ Calls reset_64bit():
       │    ├─ Writes GDT, PML4, PDPT, PD0, PD1 into low guest memory
       │    ├─ Configures EFER.LME, CR0.PG, CR4.PAE, CR3=PML4
       │    ├─ Sets CS=0x08 (kernel code), RIP=kernel entry
       │    └─ RAX=0x0c00ffee (magic), RBX=initrd info
       └─ KVM_RUN → guest enters long mode at kernel entry

kernel (guest, Ring 0)
  ├─ start.S: zero BSS, set up stack, call kmain()
  ├─ kmain(): parse boot args, init HAL, init MM, init PM
  ├─ spawn_servers(): load ELF64 from initrd, create process
  └─ kcall::handler(): schedule user process → iretq to Ring 3

hello-rust-nostd (guest, Ring 3)
  ├─ _do_start: align stack, call _start(argp, envp)
  ├─ _start(): init runtime, parse args, call main()
  └─ main(): prints "Hello, world from Rust!" via IKC
```

## VMM Guest Setup

The VMM's `reset_64bit()` function (in `vcpu/reset64.rs`) configures the guest to start
directly in 64-bit long mode, bypassing real mode and protected mode entirely. The main
`vcpu/mod.rs` contains only a `reset()` dispatcher that calls either `reset_32bit()` or
`reset_64bit()` based on the target architecture.

### Boot Structures

Fixed physical addresses in the first 40 KiB of guest memory:

| Structure | Address | Size    | Description                          |
|-----------|---------|---------|--------------------------------------|
| GDT       | 0x5000  | 40 B    | 5-entry Global Descriptor Table      |
| PML4      | 0x6000  | 4 KiB   | Page Map Level 4 (single entry)      |
| PDPT      | 0x7000  | 4 KiB   | Page Directory Pointer Table         |
| PD0       | 0x8000  | 4 KiB   | Page Directory for 0–1 GiB           |
| PD1       | 0x9000  | 4 KiB   | Page Directory for 1–2 GiB           |

> ℹ️ These pages are reserved in the kernel frame allocator as "vmm-boot-structures"
> (0x2000–0x9FFF) to prevent them from being allocated and overwritten during runtime.

### Initial Page Tables

The VMM identity-maps the first 2 GiB using 2 MiB pages:

- **PML4[0] → PDPT** with flags `0x07` (Present + Writable + User).
- **PDPT[0] → PD0** with flags `0x03` (Present + Writable, supervisor-only).
- **PDPT[1] → PD1** with flags `0x07` (Present + Writable + User).
- **PD0[0..511]**: 2 MiB pages covering 0–1 GiB, flags `0x83` (supervisor-only).
- **PD1[0..511]**: 2 MiB pages covering 1–2 GiB, flags `0x87` (user-accessible).

The first 1 GiB (kernel space) is supervisor-only. The second 1 GiB (user space starting
at 0x40000000) is user-accessible. The U/S bit must be set at every level of the page table
hierarchy for user-mode access to succeed.

### Control Registers

| Register | Value      | Description                                    |
|----------|------------|------------------------------------------------|
| CR0      | 0x80000021 | PE (protected) + NE (numeric error) + PG (paging) |
| CR3      | 0x6000     | PML4 physical address                          |
| CR4      | 0x20       | PAE (physical address extension)               |
| EFER     | 0x500      | LME (long mode enable) + LMA (long mode active) |
| RFLAGS   | 0x2        | Reserved bit set, IF=0 (interrupts disabled)   |

## Kernel Architecture

### Global Descriptor Table

The kernel installs its own GDT during `Hal::init()` with 7 entries:

| Index | Selector | Name        | DPL | Description                        |
|-------|----------|-------------|-----|------------------------------------|
| 0     | 0x00     | Null        | —   | Required null descriptor           |
| 1     | 0x08     | KernelCode  | 0   | 64-bit code segment (L=1, D=0)    |
| 2     | 0x10     | KernelData  | 0   | Data segment                       |
| 3     | 0x1B     | UserCode    | 3   | 64-bit code segment (L=1, D=0)    |
| 4     | 0x23     | UserData    | 3   | Data segment                       |
| 5–6   | 0x28     | TSS         | 0   | 64-bit TSS (16 bytes, two slots)   |

In long mode, code segments must have L=1 (long mode) and D=0. Data segments ignore L and D.
The TSS descriptor occupies two GDT slots because it is 16 bytes in 64-bit mode.

### Interrupt Descriptor Table

The IDT has 256 entries using 16-byte 64-bit gate descriptors:

| Range   | Type               | DPL | Description                        |
|---------|--------------------|-----|------------------------------------|
| 0–20    | Interrupt Gate     | 0   | CPU exceptions (DE, DB, NMI, ...)  |
| 30      | Interrupt Gate     | 0   | Security Exception                 |
| 32–47   | Interrupt Gate     | 0   | Hardware interrupts (IRQ 0–15)     |
| 128     | Interrupt Gate     | 3   | System call (`int 0x80`)           |

Exception handlers are installed with DPL=0 (kernel-only). The system call gate uses DPL=3
so user-mode code can invoke `int 0x80`.

### Task State Segment

The 64-bit TSS provides:

- **RSP0**: Ring 0 stack pointer for privilege-level transitions. Updated on every context
  switch to point to the current thread's kernel stack top.
- **IST entries**: Not currently used (reserved for future NMI/double-fault stacks).
- **I/O Map Base**: Set to the TSS size (no I/O bitmap).

### Context Save and Restore

The context frame is 176 bytes, laid out as follows:

```
Offset  Register    Saved by
──────  ──────────  ──────────────────
  0     RSP0        Software (kernel stack top for privilege transitions)
  8     CR3         Software (page table root)
 16     R15         context_save macro
 24     R14         context_save macro
 32     R13         context_save macro
 40     R12         context_save macro
 48     R11         context_save macro
 56     R10         context_save macro
 64     R9          context_save macro
 72     R8          context_save macro
 80     RBP         context_save macro
 88     RSI         context_save macro
 96     RDI         context_save macro
104     RDX         context_save macro
112     RCX         context_save macro
120     RBX         context_save macro
128     RAX         context_save macro
136     Error code  CPU (for exceptions with error codes)
144     RIP         CPU (interrupt/exception frame)
152     CS          CPU (interrupt/exception frame)
160     RFLAGS      CPU (interrupt/exception frame)
168     RSP         CPU (interrupt/exception frame, on privilege change)
176     SS          CPU (interrupt/exception frame, on privilege change)
```

The `context_save` and `context_restore` macros push and pop all 15 general-purpose
registers (excluding RSP which is implicit). The hardware pushes RIP, CS, RFLAGS, RSP,
and SS automatically on interrupt or exception entry.

### Paging

The kernel uses a two-layer paging approach:

1. **Hardware page tables** (managed by `hwpt.rs`): The actual 4-level page tables
   (PML4 → PDPT → PD → PT) that the CPU walks for address translation. These are
   the VMM-provided identity-mapped tables, extended at runtime.

2. **Software bookkeeping tables** (the existing x86 `PageDirectory` and `PageTable`):
   32-bit page directory/table structures used purely for tracking which virtual pages
   are mapped to which physical frames. These are never loaded into CR3 on x86_64.

When a user page is mapped via `vmem.map()`:
1. The software bookkeeping tables are updated (tracking the mapping).
2. `hwpt::map_user()` updates the hardware page tables:
   - Walks the PML4 → PDPT → PD hierarchy, creating intermediate tables as needed.
   - If the target PD entry is a 2 MiB page, splits it into 512 × 4 KiB entries.
   - Installs the 4 KiB PTE with the correct physical address and flags.
   - Flushes the TLB entry with `invlpg`.

When kernel page permissions are changed via `vmem.kctrl()` (e.g., making an MMIO region
user-accessible), the software bookkeeping tables are updated as on x86, but on x86_64 the
hardware page tables must also be updated. `kctrl()` calls `hwpt::map()` for identity-mapped
kernel regions, which splits the enclosing 2 MiB supervisor-only PD entry into 4 KiB entries
and sets the User bit on the target page. The `ensure_table()` helper propagates the User bit
upward through all intermediate page table levels (PML4, PDPT, PD), since x86_64 requires the
U/S bit to be set at every level for user-mode access to succeed.

The hardware page table manager uses a static pool of 64 page-table pages allocated from
BSS. Each page is 4 KiB (512 × 8-byte entries). This pool is sufficient for the current
workload but may need expansion for applications with large address spaces.

### Kernel Calls

Kernel calls use `int 0x80`, matching the x86 convention. The system call number is
passed in RAX, with up to five arguments in RDI, RSI, RDX, R10, and R8 (following the
Linux x86_64 syscall convention adapted for the `int` instruction).

The `_do_kcall` handler in `hooks.S` saves context, extracts the call number and
arguments, calls the Rust `do_kcall()` dispatcher, restores context, and returns via
`iretq`.

### User-Mode Transition

When the kernel creates a new process, it forges a kernel stack that mimics an interrupt
return frame:

```
Kernel Stack (forge_user_stack)
──────────────────────────────
  arg0            ← popped into RDI by __leave_kernel_to_user_mode
  User SS (0x23)  ← iretq frame: stack segment
  User RSP        ← iretq frame: user stack pointer
  RFLAGS          ← iretq frame: flags (IF set if interrupts enabled)
  User CS (0x1B)  ← iretq frame: code segment
  User RIP        ← iretq frame: entry point (_do_start)
  arg0            ← SysV ABI first argument (argp)
  arg1            ← SysV ABI second argument (envp)
  kernel_func     ← __leave_kernel_to_user_mode address
```

The scheduler picks up the new thread, restores its context, and `ret`s to
`__leave_kernel_to_user_mode`, which:

1. Pops `arg1` into RSI (envp) and `arg0` into RDI (argp).
2. Clears all volatile registers to prevent kernel state leakage.
3. Executes `iretq`, which pops RIP, CS, RFLAGS, RSP, and SS from the stack,
   transitioning the CPU to Ring 3 at the user entry point.

### Thread Creation

When `create_thread()` is called, the same `forge_user_stack` mechanism is used with
`user_fn = _do_start_thread` and `arg0 = thread_func`, `arg1 = thread_arg`. The
`__leave_kernel_to_user_mode` stub pops these into **RDI** (thread_func) and **RSI**
(thread_arg), then `iretq`s to `_do_start_thread`.

The `_do_start_thread` assembly stub (in `pm_x86_64.rs`) must therefore read the function
pointer from RDI and the argument from RSI — **not** RDX/RCX — because
`__leave_kernel_to_user_mode` zeroes all volatile registers except RDI and RSI before
`iretq`.

## Memory Layout

### Guest Physical Memory (128 MiB)

```
 Address        Size     Description
─────────────  ───────  ────────────────────────────────────────
 0x0000_0000    4 KiB   microvm control registers (MMIO)
 0x0000_1000    4 KiB   pvclock page (MMIO)
 0x0000_2000    4 KiB   Reserved (unused)
 0x0000_3000    4 KiB   Reserved (unused)
 0x0000_4000    4 KiB   Reserved (unused)
 0x0000_5000    4 KiB   VMM boot GDT
 0x0000_6000    4 KiB   VMM boot PML4
 0x0000_7000    4 KiB   VMM boot PDPT
 0x0000_8000    4 KiB   VMM boot PD0 (0–1 GiB mapping)
 0x0000_9000    4 KiB   VMM boot PD1 (1–2 GiB mapping)
 0x000A_0000   384 KiB  Available frames (user page pool)
 0x0010_0000    ~3 MiB  Kernel image (.text, .rodata, .data, .bss)
 0x0040_0000    ~2 MiB  Kernel BSS (includes slab heap, stacks)
 0x0080_0000    4 MiB   Kernel page pool (kpool)
 0x00C0_0000    ~256 KiB Initrd (user ELF binary)
 0x00C4_0000   ~124 MiB Available frames (user page pool)
 0x0800_0000           End of 128 MiB guest memory
```

### Virtual Address Space

```
 Virtual Address     Description
──────────────────  ──────────────────────────────
 0x0000_0000_0000    Kernel space start
 0x0000_0000_0000    Identity-mapped (VA == PA for first 1 GiB)
 0x3FFF_FFFF_FFFF    Kernel space end
 0x4000_0000_0000    User space start (USER_BASE)
 0x4000_0000_0000    User code/data (ELF segments loaded here)
 0xEFC0_0000_0000    User stack (512 KiB, grows down from USER_END)
 0xF000_0000_0000    User space end (USER_END)
```

> ℹ️ The current implementation uses identity mapping throughout. User virtual addresses
> (e.g., 0x40000000) map to the same physical addresses via the VMM's 2 MiB page table
> entries, which are split into 4 KiB entries on demand by the `hwpt` module.

## ELF64 Loading

ELF64 loading code lives in dedicated `elf64.rs` files, keeping the original `elf.rs`
loaders largely unchanged. Both the kernel and uservm have this split:

- **`elf.rs`**: Original 32-bit loader, plus a thin dispatcher that inspects
  `e_ident[EI_CLASS]` and delegates to either the 32-bit or 64-bit path.
- **`elf64.rs`**: `Elf64Fhdr`, `Elf64Phdr` structures and the 64-bit load functions.
  Loaded via `#[path = "elf64.rs"] mod elf64` (since `elf.rs` is a flat file, not a
  directory module).

The kernel's `elf.rs` also defines an `ElfClass` enum and a `detect_elf_class()` function
used by `kmain.rs` to determine the binary class before calling `create_process()`.

The ELF64 loader performs a two-pass approach:

1. **Dry run**: Validates all segments (alignment, address ranges, permissions) without
   allocating memory. This ensures the binary is well-formed before committing resources.
2. **Real run**: Allocates user frames, maps them into the virtual address space (both
   software bookkeeping and hardware page tables), and copies segment data from the initrd.

For each `PT_LOAD` segment, the loader:
- Allocates one user frame per 4 KiB page in the segment's virtual address range.
- Copies data from the initrd physical address to the user frame via `copy_to_user`.
- Zeroes any BSS portion (memory size exceeding file size).

## Library Changes

### Architecture Library (`arch`)

64-bit type variants live in dedicated files selected by `#[cfg(target_arch)]` in the
module routers (`cpu/mod.rs`, `mem/mod.rs`). The original 32-bit files are unmodified:

- **`idt_x86_64.rs`**: 16-byte `Idte` (64-bit interrupt gate descriptor) with `offset_high`
  for the upper 32 bits of the handler address. Includes its own copies of `GateType` and
  `Flags` since the entire module is conditionally compiled.
- **`idtr_x86_64.rs`**: 10-byte `Idtr` with `u64` base address and `lidt (%rax)` assembly.
- **`tss_x86_64.rs`**: 104-byte `Tss` with RSP0–RSP2, IST1–IST7, and I/O map base.
- **`gdtr_x86_64.rs`**: 10-byte `Gdtr` with `u64` base address and `lgdt (%rax)` assembly.

### System Library (`sys`)

- **`src/sys/kcall/arch/x86_64.rs`** (new file): Kernel call stubs using `int 0x80` and
  the x86_64 register convention (RAX=number, RDI/RSI/RDX/R10/R8=arguments).
- **`src/sys/kcall/pm_x86_64.rs`** (new file): `_do_start_thread` assembly stub for x86_64
  thread entry, loaded via `#[path]` when targeting x86_64. Receives the thread function
  pointer in RDI and the thread argument in RSI (from `__leave_kernel_to_user_mode`).
- Added 64-bit variants of `ExitStatus`, process management calls, and type definitions
  in existing files with `#[cfg(target_arch = "x86_64")]` guards.

### NVX Runtime Library (`nvx`)

- **`src/crt0_x86_64.rs`** (new file): x86_64 `_do_start` entry point that receives
  arguments in RDI (argp) and RSI (envp) per the SysV ABI, aligns the stack to 16 bytes,
  and calls `_start()`. Loaded as `mod crt0_x86_64` in `lib.rs`.

### Slab Allocator Fix

The kernel heap allocator's size-to-slab mapping was changed from `4096 => Slab4096` to
`513..=4096 => Slab4096`. Without this fix, allocations between 513 and 4095 bytes failed
because they fell through to `Err(AllocError)`. This is particularly critical on x86_64
where pointer sizes are 8 bytes — a `Vec<UserFrame>` with 128 entries requires 1024 bytes,
which could not be served by any slab class.

## Integration Testing

All integration tests pass on both x86 and x86_64. The x86 test suite includes 40 tests
(Rust and C/C++). The x86_64 test suite includes 38 tests — `dlfcn-c` is excluded because
the Nanvix ELF dynamic loader only handles `R_386_*` relocations (see
[Known Limitations](#known-limitations)). Six issues were identified and fixed during the
porting process.

### TLS Inline Assembly (`test-kernel`)

The TLS stress tests in `test-kernel` use inline assembly to read the Thread Data Area (TDA)
via a segment register. On x86, the TDA is accessed through `%gs` (configured via a GDT
descriptor). On x86_64, the TDA is accessed through `%fs` (configured via the FS_BASE MSR,
written by `WRMSR` during context switch). The `read_gs_offset_0()` and `read_gs_at()`
helpers now use `#[cfg(target_arch)]` guards to emit the correct segment prefix.

### MMIO RAMFS Page Fault

The kernel's `kctrl()` function in `vmem.rs` changes page permissions for MMIO regions (e.g.,
the RAMFS image mapped by the VMM near the top of guest physical memory). On x86, the software
page tables are the hardware page tables, so `kctrl()` works directly. On x86_64, the software
bookkeeping tables are separate from the VMM-provided 4-level hardware page tables, so
`kctrl()` alone left the hardware page tables unchanged. User-mode access to the RAMFS page
faulted with error code 5 (user-mode read of a supervisor-only page).

The fix adds a `hwpt::map()` call in `kctrl()` for x86_64, which splits the enclosing 2 MiB
supervisor-only PD entry into 4 KiB entries and propagates the User bit through all
intermediate page table levels (PML4, PDPT, PD). See the [Paging](#paging) section for
details.

### Worker Thread RIP=0 Crash

The `_do_start_thread` assembly stub (in `pm_x86_64.rs`) originally read the thread function
pointer from RDX and the thread argument from RCX. However, `__leave_kernel_to_user_mode`
passes arguments via RDI and RSI (and zeroes all other volatile registers). This caused the
worker thread to call through a null function pointer (RDX=0), crashing at RIP=0. The fix
changes the stub to read from RDI and RSI. See the [Thread Creation](#thread-creation)
section.

### Mutex/Condvar Timeout Sentinel

The user-space `lock_mutex()` and `wait_cond()` wrappers use `(u32::MAX, u32::MAX)` as a
sentinel to indicate "no timeout" (infinite wait). The kernel handlers compared these values
against `usize::MAX`. On x86, `u32::MAX == usize::MAX` (both are `0xFFFFFFFF`), so the check
worked. On x86_64, `usize::MAX` is `0xFFFFFFFFFFFFFFFF`, which does not match the
zero-extended `u32::MAX` value `0x00000000FFFFFFFF` received from user space. The kernel
rejected the timeout as invalid. The fix changes the sentinel checks to use
`u32::MAX as usize`, which works on both architectures.

### FFI Type Definitions (`sysapi`)

The `sysapi` crate's `ffi.rs` module originally defined all C type aliases in a single
`mod bits32` with `c_long = i32` and `c_ulong = u32`. This matches the x86 ILP32 data model
but is incorrect for x86_64 LP64 where C `long` is 8 bytes. The fix splits the type
definitions into a conditional module:

```rust
#[cfg(target_pointer_width = "32")]
mod bits { pub type c_long = i32; pub type c_ulong = u32; }

#[cfg(target_pointer_width = "64")]
mod bits { pub type c_long = i64; pub type c_ulong = u64; }
```

This propagates to `timespec` (where `tv_nsec` is `c_long`) and `stat` (where `st_blksize`
and `st_blkcnt` are `c_long`), causing these structs to have different native sizes on x86
(84 and 12 bytes) versus x86_64 (104 and 16 bytes).

### Struct Layout (`repr(C)` vs `repr(C, packed)`)

Several structs (`stat`, `dirent`, `posix_dent`, `pthread_attr_t`, `pthread_condattr_t`)
used `#[repr(C, packed)]`. On x86, packed layout matches natural C alignment because
8-byte types (like `c_long` on x86_64) have 4-byte alignment. On x86_64, `repr(C, packed)`
suppresses the required 8-byte alignment for `c_long` fields, producing layouts that don't
match the C compiler's output. The fix changes these structs to `#[repr(C)]`, which
produces correct layouts on both architectures.

### IPC Wire Format

The host daemon (linuxd) always runs as a native x86_64 process. The guest can be x86 or
x86_64. IPC messages serialize structs to bytes and transmit them between guest and host.
With `c_long = i64` on the host and `c_long = i32` on the x86 guest, the wire format must
be architecture-independent. The solution uses a fixed 8-byte (i64) representation for all
`c_long`-based fields in the wire format:

| Struct      | Field(s)         | Native x86 | Native x86_64 | Wire Format |
|-------------|------------------|------------|---------------|-------------|
| `timespec`  | `tv_nsec`        | 4 bytes    | 8 bytes       | 8 bytes     |
| `stat`      | `st_blksize`     | 4 bytes    | 8 bytes       | 8 bytes     |
| `stat`      | `st_blkcnt`      | 4 bytes    | 8 bytes       | 8 bytes     |
| `stat`      | `st_atim/mtim/ctim` | 12 bytes each | 16 bytes each | 16 bytes each |

The `timespec::to_bytes()` and `timespec::try_from_bytes()` methods always write/read
`tv_nsec` as `i64`, casting to/from the native `c_long` type. Similarly, `stat::to_bytes()`
and `stat::try_from_bytes()` serialize `st_blksize` and `st_blkcnt` as `i64`.

The `futimens` message was converted from `mem::transmute`-based serialization (which
assumed identical struct layouts on both sides) to explicit byte serialization using
`timespec::to_bytes()`/`try_from_bytes()`. Other `transmute`-based messages (`fstat`,
`fchmod`, `mkdirat`, etc.) only contain fixed-size fields (`i32`, `u32`) and are unaffected.

### C Test Portability

The C integration tests were updated for LP64 compatibility:

- **`c-bindings`**: Static assertions for type sizes now account for x86_64 sizes (e.g.,
  `sizeof(size_t) == 8`, `sizeof(long) == 8`).
- **`file-c`**: Added padding defines (`STAT_MODE_PADDING`, `POSIX_DENT_TAIL_PADDING`) for
  struct field offsets that differ due to alignment. Fixed `off_t`/`size_t` sign comparison.
- **`thread-c`**: Added padding defines for `pthread_attr_t` and `pthread_condattr_t`.
  Fixed `int` → `void*` cast via `(uintptr_t)`.
- **`memory-c`**: Changed `aligned_alloc` alignment from `4u` to `sizeof(void *)`.
- **`dlfcn-c`**: Added x86_64 inline assembly variant for the shared library.

## Clippy and Lint Fixes

Several Clippy lints that pass on x86 trigger errors on x86_64 due to differences in pointer
width, type sizes, and stack frame budgets. The kernel enforces strict lint policies via
`#![deny(clippy::all)]` and `#![forbid(clippy::large_stack_frames)]`.

### Operator Precedence (`hwpt.rs`)

In `split_2m_entry()`, the expression `base_2m + (i as u64 * 4096) | flags_4k` was flagged by
`clippy::precedence` because bitwise OR binds more tightly than intended. The fix adds explicit
parentheses: `(base_2m + (i as u64 * 4096)) | flags_4k`.

### Large Stack Frames (`event/manager.rs`)

The `EventManagerInner` struct used `usize::BITS as usize` to size the interrupt and exception
ownership/pending arrays. On x86, `usize::BITS = 32` which happens to match the hardware event
count, so the arrays are 32 elements. On x86_64, `usize::BITS = 64`, doubling array sizes and
pushing `init()` to 36,695 bytes of stack — exceeding the kernel's 32,768-byte threshold
enforced by `#![forbid(clippy::large_stack_frames)]`.

The fix replaces `usize::BITS as usize` with the correct constants:

| Array                    | Old Size          | New Size                       |
|--------------------------|-------------------|--------------------------------|
| `interrupt_ownership`    | `usize::BITS` (64)| `InterruptEvent::NUMBER_EVENTS` (32) |
| `pending_interrupts`     | `usize::BITS` (64)| `InterruptEvent::NUMBER_EVENTS` (32) |
| `exception_ownership`    | `usize::BITS` (64)| `ExceptionEvent::NUMBER_EVENTS` (32) |
| `pending_exceptions`     | `usize::BITS` (64)| `ExceptionEvent::NUMBER_EVENTS` (32) |
| `scheduling_ownership`   | unchanged         | `SchedulingEvent::NUMBER_EVENTS` (3) |
| `pending_scheduling`     | unchanged         | `SchedulingEvent::NUMBER_EVENTS` (3) |

All loop bounds that iterated `0..usize::BITS` over these arrays were updated to use the
corresponding `NUMBER_EVENTS` constant. This prevents out-of-bounds access on x86_64 and
reduces stack usage on both architectures.

### Cast Truncation (`libc_stdlib`)

The `malloc_usable_size()` function casts `usize` to `c_size_t`, which is `u32` in Nanvix
(defined as `c_uint` in `sysapi`). On x86, this cast is a no-op (both are 32-bit). On x86_64,
it is a truncating cast. The crate forbids `clippy::cast_possible_truncation` on 64-bit targets,
so `#[allow(...)]` annotations conflict with the crate-level `#[forbid(...)]`.

The fix replaces the direct cast on 64-bit targets with a safe conversion:

```rust
match c_size_t::try_from(size) {
    Ok(v) => v,
    Err(_) => c_size_t::MAX,
}
```

## CI/CD Pipeline

A dedicated GitHub Actions workflow (`.github/workflows/ci-x86_64.yml`) provides continuous
integration for x86_64 builds. It runs on GitHub-hosted `ubuntu-24.04` runners with KVM
enabled for integration tests.

### Workflow Structure

The workflow has three stages:

1. **Lint** — Runs Clippy and format checks via Docker. Matrix: `{single-process,
   multi-process}`.
2. **Build** — Compiles all Nanvix components via Docker. Matrix: `{debug, release} ×
   {single-process, multi-process}`. Build artifacts are uploaded for the test stage.
3. **Test** — Downloads build artifacts and runs unit tests (via `make run-unit-tests`) and
   integration tests (via `nanvix-test.elf`) on KVM-enabled runners. Matrix matches the build
   stage.

### Triggers

The workflow triggers on:
- Pushes to `dev` and `feature-kernel-x64` branches.
- Pull requests targeting `dev`.
- Manual dispatch (`workflow_dispatch`).

### Action Parameterization

The existing CI actions (`docker-check`, `docker-build`, `build`, `lint`, `test`) were extended
with a `target-arch` input (default: `x86`). When `target-arch=x86_64`, the actions pass
`TARGET=x86_64` to the build system, and the Docker build action includes the architecture in
the sccache cache key to avoid cross-architecture cache collisions.

### C/C++ Cross-Compiler Toolchain

The `x86_64-nanvix` cross-compiler toolchain is built from the Nanvix forks of binutils,
GCC, and newlib. The build order is: binutils → GCC stage 0 (C only, no libc) → newlib →
GCC stage 1 (C, C++, Fortran with libstdc++). Key configuration flags:

```bash
# Binutils
../src/binutils/configure --target=x86_64-nanvix --prefix=$PREFIX --disable-nls

# GCC stage 0
../src/gcc/configure --target=x86_64-nanvix --prefix=$PREFIX --without-headers \
    --with-newlib --disable-multilib --enable-languages=c --disable-nls

# Newlib
../src/newlib/configure --target=x86_64-nanvix --prefix=$PREFIX

# GCC stage 1
../src/gcc/configure --target=x86_64-nanvix --prefix=$PREFIX --with-newlib \
    --disable-multilib --enable-languages=c,c++,fortran --disable-nls
```

The newlib `crt0.S` was extended with an `#ifdef __x86_64__` block that aligns the stack
to 16 bytes (`andq $-16, %rsp`) and follows the SysV ABI register convention (`argp` in
`%rdi`, `envp` in `%rsi`). Custom `crti.S` and `crtn.S` files were added to provide
proper `.init`/`.fini` section prologues and epilogues — GCC generates empty `crti.o` and
`crtn.o` for `--with-newlib` targets, which causes SSE `movaps` instructions to fault on
misaligned stacks in C++ programs. The build system gate that previously excluded C/C++
compilation for `TARGET=x86_64` (`ifneq ($(TARGET),x86_64)` in
`build/make/generic-guest-binaries.mk`) was removed.

### Test Configurations

Separate test configuration files account for x86_64-specific exclusions:

| Deployment      | x86 Config                      | x86_64 Config                            |
|-----------------|---------------------------------|------------------------------------------|
| single-process  | `test/test-single_process.toml` | `test/test-single_process-x86_64.toml`   |
| multi-process   | `test/test-multi_process.toml`  | `test/test-multi_process-x86_64.toml`    |

The x86_64 configs include all Rust and C/C++ tests except `dlfcn-c` (which requires
`R_X86_64_*` ELF relocations not yet supported by the Nanvix dynamic loader). The Makefile
selects the correct config file based on `TARGET` and `SINGLE_PROCESS` variables.

## Known Limitations

- **Single address space**: The kernel does not load its own page tables into CR3. All
  processes share the VMM-provided identity-mapped page tables. Process isolation via
  separate address spaces is not yet implemented.
- **No SMP**: The port targets a single-processor configuration.
- **No interrupt controller**: PIC/IOAPIC initialization is skipped on x86_64 microvm.
  The kernel runs with interrupts disabled and uses cooperative scheduling.
- **No FPU/SSE in user mode**: The target specs disable SIMD features. User programs
  must not use floating-point or SIMD instructions.
- **Static page table pool**: The hardware page table manager allocates from a fixed pool
  of 64 pages (256 KiB). This limits the number of distinct 4 KiB mappings.
- **`__context_switch` does not reload CR3**: Since all processes share the same address
  space, the context switch skips the CR3 load. This must be revisited when per-process
  page tables are implemented.
- **No dynamic linking on x86_64**: The Nanvix ELF dynamic loader only handles `R_386_*`
  relocation types. The `dlfcn-c` test is excluded from x86_64 test configurations until
  `R_X86_64_*` relocations are implemented.
- **`faccessat()` under root**: The `faccessat()` and `access()` tests are disabled on
  both x86 and x86_64 because linuxd runs as root (via `sudo` for KVM access) and root
  bypasses DAC permission checks, causing `W_OK` checks on read-only files to succeed.
- **`c_size_t` is 32-bit on x86_64**: The `c_size_t` and `c_ssize_t` types in `sysapi`
  remain defined as `u32`/`i32` even on x86_64 to preserve IPC wire format compatibility
  for `read`/`write`/`getdents` messages. This is a known compromise.
