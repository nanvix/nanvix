// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// x86 Bootstrap and Application-Processor Entry Points
//==================================================================================================

use super::constants;
use core::arch::global_asm;

//==================================================================================================
// Constants
//==================================================================================================

const PAGE_SIZE: u32 = 4096;

/// Hardware-saved execution context size (in bytes).
const CONTEXT_HW_SIZE: u32 = 24;

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

    // EAX and EBX registers store boot information.

    // Fill BSS section with zeros.
    "    movl %eax, %edx",
    "    movl $__BSS_START, %edi",
    "    movl $__BSS_END, %ecx",
    "    subl %edi, %ecx",
    "    xorl %eax, %eax",
    "    cld",
    "    rep stosb",

    // Fill the boot stack guard page with a watermark pattern.
    "    movl $kstack_guard, %edi",
    "    movl $({PAGE_SIZE} / 4), %ecx",
    "    movl ${KSTACK_GUARD_PATTERN}, %eax",
    "    rep stosl",

    "    movl %edx, %eax",

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

    PAGE_SIZE = const PAGE_SIZE,
    CONTEXT_HW_SIZE = const CONTEXT_HW_SIZE,
    KSTACK_GUARD_PATTERN = const constants::KSTACK_GUARD_PATTERN,
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
// BSS Section — Boot Stack and Kernel Red Zone
//==================================================================================================

global_asm!(
    ".section .bss",

    // Boot stack guard page + usable stack.
    ".align {PAGE_SIZE}",
    ".globl kstack_guard",
    "kstack_guard:",
    ".space {KSTACK_SIZE}",
    ".globl kstack",
    "kstack:",

    // Kernel Red Zone.
    ".globl kredzone",
    "kredzone:",
    ".space {KREDZONE_SIZE}",

    PAGE_SIZE = const PAGE_SIZE,
    KSTACK_SIZE = const constants::KSTACK_SIZE,
    KREDZONE_SIZE = const constants::KREDZONE_SIZE,
    options(att_syntax),
);
