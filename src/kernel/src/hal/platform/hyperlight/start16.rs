// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// x86 Trampoline Code (Hyperlight)
//==================================================================================================
//
// This module contains code placed in the `.entry` and `.trampoline` sections:
//
//   - `_entry`: Jump stub at PLATFORM_BASE_ADDR (0x1000). Hyperlight starts execution here.
//   - `_do_start`: BSP entry — 32-bit protected mode, jumps to `_do_start2`.
//   - `_ap_trampoline` (SMP only): AP startup (16-bit → protected mode → `_do_ap_start`).
//
// With `i686-guest`, the VMM starts the BSP in 32-bit protected mode with paging enabled.
// Segment registers are loaded by the VMM via `set_sregs()` with flat descriptors.  Page
// tables are built by the VMM in the snapshot region and **copied** to scratch before the
// guest starts; CR3 points to the scratch copy (RW).  No GDT load is needed on the BSP
// path — the VMM-provided segment descriptors are used as-is.
//

use core::arch::global_asm;

//==================================================================================================
// Entry Section — Jump stub at PLATFORM_BASE_ADDR
//==================================================================================================
//
// On Hyperlight, the VMM starts execution at BASE_ADDRESS (PLATFORM_BASE_ADDR = 0x1000),
// not at the ELF entry point. This stub is placed at the very first byte of the loaded image
// and immediately jumps to `_do_start` in the trampoline section.

global_asm!(
    r#".section .entry,"ax",@progbits"#,
    ".code32",
    ".globl _entry",
    "_entry:",
    "    jmp _do_start",
    options(att_syntax),
);

//==================================================================================================
// Trampoline Section — BSP Entry (32-bit protected mode stub)
//==================================================================================================

global_asm!(
    r#".section .trampoline,"ax",@progbits"#,
    ".code32",
    ".align 4",
    ".globl _do_start",
    "_do_start:",
    // With Hyperlight stable (i686-guest), the host starts the vCPU directly
    // in 32-bit protected mode with paging enabled.  The VMM already loaded
    // valid flat segment descriptors via set_sregs() and CR3 points to the
    // host-built page tables (copied to scratch, RW).  No GDT load is needed.
    ".extern _do_start2",
    "    jmp   _do_start2",
    options(att_syntax),
);

//==================================================================================================
// Trampoline Section — AP Trampoline and GDT (SMP only)
//==================================================================================================
//
// The AP trampoline and its supporting GDT are only needed for multi-core startup.
// SMP boot on Hyperlight is out of scope for the current integration.

#[cfg(feature = "smp")]
global_asm!(
    r#".section .trampoline,"ax",@progbits"#,
    ".code16",
    ".align 4",
    ".globl _ap_trampoline",
    "_ap_trampoline:",
    "    cli",
    // Zero data segment registers.
    "    xorw    %ax,%ax",
    "    movw    %ax,%ds",
    "    movw    %ax,%es",
    "    movw    %ax,%fs",
    "    movw    %ax,%gs",
    "    movw    %ax,%ss",
    "    lgdt   gdtptr",
    "    movl    %cr0, %eax",
    "    orl     $1, %eax",
    "    mov    %eax, %cr0",
    "    jmpl $0x8, $_do_ap_start",
    options(att_syntax),
);

#[cfg(feature = "smp")]
global_asm!(
    r#".section .trampoline,"ax",@progbits"#,
    ".p2align 2",
    "gdt:",
    // Null segment descriptor.
    "    .word 0, 0",
    "    .byte 0, 0, 0, 0",
    // Code segment: base=0, limit=0xFFFFFFFF, type=Execute+Read.
    "    .word 0xffff, 0x0000",
    "    .byte 0x00, 0x9a, 0xcf, 0x00",
    // Data segment: base=0, limit=0xFFFFFFFF, type=Write.
    "    .word 0xffff, 0x0000",
    "    .byte 0x00, 0x92, 0xcf, 0x00",
    "gdtptr:",
    "    .word   (gdtptr - gdt - 1)",
    "    .long   gdt",
    options(att_syntax),
);
