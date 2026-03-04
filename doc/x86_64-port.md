# Nanvix x86_64 Port

This document describes the port of Nanvix from x86 (32-bit) to x86_64 (64-bit long mode),
covering the design decisions, architecture, memory layout, and boot flow.

## Table of Contents

- [Overview](#overview)
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
make TARGET=x86_64 MACHINE=microvm all-kernel

# Build a 64-bit user binary.
make TARGET=x86_64 MACHINE=microvm all-guest-binaries-hello-rust-nostd

# Run it.
./bin/nanvixd.elf -- ./bin/hello-rust-nostd.elf
```

The 32-bit build remains the default and is unaffected:

```bash
make TARGET=x86 MACHINE=microvm all-kernel
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

The VMM's `reset_64bit()` function configures the guest to start directly in 64-bit long
mode, bypassing real mode and protected mode entirely.

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

The kernel detects whether an ELF binary is 32-bit or 64-bit by inspecting `e_ident[EI_CLASS]`:

- **ELFCLASS32** (1): Dispatched to the existing `do_elf32_load()`.
- **ELFCLASS64** (2): Dispatched to the new `do_elf64_load()`.

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

- **IDT**: Added `Idte` (16-byte 64-bit interrupt gate descriptor) with `offset_high` for
  the upper 32 bits of the handler address.
- **IDTR**: Added 64-bit `Idtr` with `u64` base address.
- **TSS**: Added 64-bit `Tss` structure (104 bytes) with RSP0-RSP2, IST1-IST7, and I/O
  map base.
- **GDTR**: Added 64-bit `Gdtr` with `u64` base address.

### System Library (`sys`)

- Added `src/sys/kcall/arch/x86_64.rs` with kernel call stubs using `int 0x80` and the
  x86_64 register convention (RAX=number, RDI/RSI/RDX/R10/R8=arguments).
- Added 64-bit variants of `ExitStatus`, process management calls, and type definitions.

### NVX Runtime Library (`nvx`)

- Added x86_64 `_do_start` entry point that receives arguments in RDI (argp) and RSI
  (envp) per the SysV ABI, aligns the stack to 16 bytes, and calls `_start()`.

### Slab Allocator Fix

The kernel heap allocator's size-to-slab mapping was changed from `4096 => Slab4096` to
`513..=4096 => Slab4096`. Without this fix, allocations between 513 and 4095 bytes failed
because they fell through to `Err(AllocError)`. This is particularly critical on x86_64
where pointer sizes are 8 bytes — a `Vec<UserFrame>` with 128 entries requires 1024 bytes,
which could not be served by any slab class.

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
