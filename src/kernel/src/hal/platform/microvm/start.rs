// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// x86 Bootstrap and Application-Processor Entry Points (microvm)
//==================================================================================================

use crate::hal::arch::x86::cpu::ContextInformation;
use ::arch::mem::PAGE_SIZE;
use ::core::arch::global_asm;

//==================================================================================================
// Bootstrap Section — BSP Entry Point
//==================================================================================================

// _do_start2 (entered from 16-bit trampoline, already in protected mode).
global_asm!(
    r#".section .bootstrap,"ax",@progbits"#,

    ".align 4",
    ".globl _do_start2",
    "_do_start2:",
    "    mov $0x10, %dx",
    "    mov %dx, %ds",
    "    mov %dx, %es",
    "    mov %dx, %fs",
    "    mov %dx, %gs",
    "    mov %dx, %ss",

    // System V i386 ABI requires DF=0; set it once for all subsequent code.
    "    cld",

    // EAX and EBX registers store boot information.

    // BSS clearing is skipped on microvm backends: the host has already zeroed guest memory
    // before the vCPU starts (`mmap(MAP_ANONYMOUS)` on Linux, `VirtualAlloc(MEM_COMMIT)` on
    // Windows both return zero-filled pages, and the ELF loader explicitly zeroes every BSS
    // region via `write_bytes()`).

    // The boot stack guard page is pre-filled with the watermark pattern at link time (see the
    // `.data` block below), so no runtime initialization loop is required here.

    // Reset stack.
    "    movl $kstack, %esp",
    "    movl %esp, %ebp",

    // Initialize the dynamic stack overflow guard for the boot stack.
    "    movl $(kstack_guard + {CONTEXT_HW_SIZE}), EXCP_STACK_GUARD",

    // Save boot information on the stack.
    "    push %ebx",
    "    push %eax",

    // Clear all general purpose registers for deterministic startup.
    "    xorl %eax, %eax",
    "    xorl %ebx, %ebx",
    "    xorl %ecx, %ecx",
    "    xorl %edx, %edx",
    "    xorl %esi, %esi",
    "    xorl %edi, %edi",

    // Call kernel main function.
    "    push %esp",
    "    call kmain",
    "    addl $4, %esp",

    // Cleanup boot information.
    "    addl $8, %esp",

    // Halt execution.
    "1:  hlt",
    "    jmp 1b",

    CONTEXT_HW_SIZE = const ContextInformation::CONTEXT_HW_SIZE,
    options(att_syntax),
);

//==================================================================================================
// Bootstrap Section — AP Entry Point
//==================================================================================================

global_asm!(
    r#".section .bootstrap,"ax",@progbits"#,
    ".align 4",
    ".globl _do_ap_start",
    "_do_ap_start:",
    "    mov $0x10, %ax",
    "    mov %ax, %ds",
    "    mov %ax, %es",
    "    mov %ax, %fs",
    "    mov %ax, %gs",
    "    mov %ax, %ss",
    // Clear all general purpose registers for deterministic startup.
    "    xorl %eax, %eax",
    "    xorl %ebx, %ebx",
    "    xorl %ecx, %ecx",
    "    xorl %edx, %edx",
    "    xorl %esi, %esi",
    "    xorl %edi, %edi",
    // Setup stack.
    "    movl (kredzone), %esp",
    "    movl %esp, %ebp",
    // Get local APIC ID.
    "    movl  $1, %eax",
    "    cpuid",
    "    shrl  $24, %ebx",
    // Call kernel main function.
    "    pushl %ebx",
    "    call do_ap_start",
    "    addl $4, %esp",
    // Halt execution.
    "1:  hlt",
    "    jmp 1b",
    options(att_syntax),
);

//==================================================================================================
// Data Section — Boot Stack
//==================================================================================================

// The boot stack lives in `.data` rather than `.bss` so that its guard page can be initialized
// with the watermark pattern at link time via `.fill`. This eliminates the runtime fill loop in
// the BSP entry path at the cost of `KSTACK_SIZE` bytes added to the kernel ELF image.
global_asm!(
    ".section .data",

    // Boot stack guard page + usable stack.
    ".align {PAGE_SIZE}",
    ".globl kstack_guard",
    "kstack_guard:",
    // Guard page pre-filled with the watermark pattern (4-byte little-endian dwords).
    ".fill ({PAGE_SIZE} / 4), 4, {KSTACK_GUARD_PATTERN}",
    // Usable stack area, zero-initialized.
    ".fill ({KSTACK_SIZE} - {PAGE_SIZE}), 1, 0",
    ".globl kstack",
    "kstack:",

    PAGE_SIZE = const PAGE_SIZE,
    KSTACK_SIZE = const ::config::kernel::KSTACK_SIZE,
    KSTACK_GUARD_PATTERN = const ::config::kernel::KSTACK_GUARD_PATTERN,
    options(att_syntax),
);

//==================================================================================================
// BSS Section — Kernel Red Zone
//==================================================================================================

global_asm!(
    ".section .bss",

    // Kernel Red Zone.
    ".align {PAGE_SIZE}",
    ".globl kredzone",
    "kredzone:",
    ".space {KREDZONE_SIZE}",

    PAGE_SIZE = const PAGE_SIZE,
    KREDZONE_SIZE = const ::config::kernel::KREDZONE_SIZE,
    options(att_syntax),
);
