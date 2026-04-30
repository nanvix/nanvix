// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// x86 Bootstrap and Application-Processor Entry Points (Hyperlight)
//==================================================================================================

use ::core::arch::global_asm;
use ::hyperlight_common::outb::VmAction;

//==================================================================================================
// Trampoline Section — BSP Entry Point
//==================================================================================================
//
// _do_start: the VMM starts execution here (placed at PLATFORM_BASE_ADDR by the linker script).
// Already in 32-bit protected mode with paging enabled.
global_asm!(
    r#".section .trampoline,"ax",@progbits"#,

    ".align 4",
    ".globl _do_start",
    "_do_start:",

    // With Hyperlight stable (i686-guest), the host starts the vCPU in 32-bit
    // protected mode with paging enabled and CR3 pointing to a host-built page
    // directory.  Snapshot pages are mapped read-only (CoW via the KVM memory
    // slot) and the scratch region is mapped read-write. Hyperlight may use
    // distinct GPA and GVA ranges elsewhere, with explicit GPA↔GVA
    // translation; this bootstrap path relies only on the host-installed
    // mappings already making the scratch-backed boot stack accessible.
    //
    // The host-built page tables are kept active:
    //  - Do NOT disable paging.
    //  - Do NOT build a PSE identity map.
    //  - Do NOT reload segment registers (snapshot pages are read-only until
    //    eager_prefault_cow_pages() resolves them in hyperlight_pre_kmain()).
    //
    // The cached segment descriptors set by the VMM are valid.

    // System V i386 ABI requires DF=0; set it once for all subsequent code.
    "    cld",


    // Set ESP directly to the scratch-backed boot stack.  BOOT_STACK_TOP is a
    // compile-time constant (top of the scratch region minus the two reserved
    // pages).
    "    movl ${BOOT_STACK_TOP}, %esp",
    "    movl %esp, %ebp",

    // Save boot information (EAX=magic, EBX=info) on the scratch-backed stack.
    // These values persist through all function calls and are later read by
    // _nanvix_dispatch as KernelArguments.
    "    push %ebx",
    "    push %eax",

    // NOTE: BSS is zeroed by the Hyperlight VMM before the guest starts;
    // no explicit clear is needed here.

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
// Bootstrap Section — AP Entry Point (SMP only)
//==================================================================================================

#[cfg(feature = "smp")]
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
