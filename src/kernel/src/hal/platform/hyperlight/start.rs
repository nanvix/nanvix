// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// x86 Bootstrap and Application-Processor Entry Points (Hyperlight)
//==================================================================================================

use ::core::arch::global_asm;
use ::hyperlight_common::outb::VmAction;

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

    // Save boot info (EAX) before it is clobbered by the BSS clear below.
    "    movl %eax, %edx",

    // Fill BSS section with zeros.
    "    movl $__BSS_START, %edi",
    "    movl $__BSS_END, %ecx",
    "    subl %edi, %ecx",
    "    xorl %eax, %eax",
    "    rep stosb",

    "    movl %edx, %eax",

    // Set ESP directly to the scratch-backed boot stack.  BOOT_STACK_TOP is a
    // compile-time constant (top of the scratch region minus the two reserved
    // pages), so no BSS-based temporary stack is needed.
    "    movl ${BOOT_STACK_TOP}, %esp",
    "    movl %esp, %ebp",

    // Save boot information on the scratch stack.  These values persist
    // through init_scratch_kstack() and hyperlight_pre_kmain() and are
    // later read by _nanvix_dispatch as KernelArguments.
    "    push %ebx",
    "    push %eax",

    // Patch PEB GVA→GPA, fill the scratch guard page with the watermark
    // pattern, and set EXCP_STACK_GUARD.
    "    call init_scratch_kstack",

    // Clear all general purpose registers for deterministic startup.
    "    xorl %eax, %eax",
    "    xorl %ebx, %ebx",
    "    xorl %ecx, %ecx",
    "    xorl %edx, %edx",
    "    xorl %esi, %esi",
    "    xorl %edi, %edi",

    // Evolve phase: initialise heap and GuestHandle, then halt.
    //
    // `hyperlight_pre_kmain()` initialises the kernel heap (backed by scratch memory) and the
    // GuestHandle.  On return the assembly switches ESP to the scratch-backed stack, loads the
    // `_nanvix_dispatch` entry point into EAX, and halts the VM so that `evolve()` returns on
    // the host.  The host then calls `sandbox.call("kmain", ())` which enters
    // `_nanvix_dispatch` → `kmain`.
    "    call hyperlight_pre_kmain",
    ".extern _nanvix_dispatch",
    "    movl ${BOOT_STACK_TOP}, %esp",
    "    andl $0xFFFFFFF0, %esp",
    "    movl $_nanvix_dispatch, %eax",
    "    mov ${HALT_PORT}, %dx",
    "    outb %al, %dx",
    "    cli",
    "1:  hlt",
    "    jmp 1b",

    BOOT_STACK_TOP = const ::config::memory_layout::HYPERLIGHT_BOOT_STACK_TOP,
    HALT_PORT = const VmAction::Halt as u16,
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

    // Kernel Red Zone.
    ".globl kredzone",
    "kredzone:",
    ".space {KREDZONE_SIZE}",

    KREDZONE_SIZE = const ::config::kernel::KREDZONE_SIZE,
    options(att_syntax),
);
