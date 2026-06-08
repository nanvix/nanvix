// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// x86 16-bit Trampoline Code (microvm)
//==================================================================================================
//
// This module contains 16-bit real-mode code placed in the `.trampoline` section at physical
// address 0x8000. It provides:
//   - `_do_start`: non-multiboot BSP entry (16-bit → protected mode → `_do_start2`)
//   - `_ap_trampoline`: AP startup (16-bit → protected mode → `_do_ap_start`)
//   - A minimal GDT (null + code + data) used during the real→protected mode transition.
//

use core::arch::global_asm;

//==================================================================================================
// Trampoline Section — BSP Entry
//==================================================================================================

global_asm!(
    r#".section .trampoline,"ax",@progbits"#,
    ".code16",
    ".align 4",
    ".globl _do_start",
    "_do_start:",
    // Zero data segment registers.
    "    xorw  %dx,%dx",
    "    movw  %dx,%ds",
    "    movw  %dx,%es",
    "    movw  %dx,%fs",
    "    movw  %dx,%gs",
    "    movw  %dx,%ss",
    "    lgdt  gdtptr",
    "    movl  %cr0, %edx",
    "    orl   $1, %edx",
    "    mov   %edx, %cr0",
    ".extern _do_start2",
    "    jmpl $0x8, $_do_start2",
    options(att_syntax),
);

//==================================================================================================
// Trampoline Section — AP Trampoline
//==================================================================================================

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

//==================================================================================================
// Trampoline Section — GDT
//==================================================================================================

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
