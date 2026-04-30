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

// _do_start2 (entered from trampoline stub, already in 32-bit protected mode with paging).
global_asm!(
    r#".section .bootstrap,"ax",@progbits"#,

    ".align 4",
    ".globl _do_start2",
    "_do_start2:",

    // With Hyperlight stable (i686-guest), the host starts the vCPU in 32-bit
    // protected mode with paging enabled and CR3 pointing to a host-built page
    // directory.  Snapshot pages are mapped read-only (CoW via the KVM memory
    // slot) and the scratch region is mapped read-write.  On i686,
    // MAX_GVA == MAX_GPA, so scratch GVA == GPA — no address translation
    // divergence.
    //
    // The host-built page tables are kept active:
    //  - Do NOT disable paging.
    //  - Do NOT build a PSE identity map.
    //  - Do NOT reload segment registers (the CPU writes the Accessed bit into
    //    the GDT entry on each load, and the GDT page is in the read-only
    //    snapshot region — a write would fault before the early IDT is
    //    installed).
    //
    // The cached segment descriptors set by the VMM are valid.

    // System V i386 ABI requires DF=0; set it once for all subsequent code.
    "    cld",


    // Set ESP directly to the scratch-backed boot stack.  BOOT_STACK_TOP is a
    // compile-time constant (top of the scratch region minus the two reserved
    // pages).  The stack must be established before the early IDT installation
    // (below) which requires a valid stack pointer.
    "    movl ${BOOT_STACK_TOP}, %esp",
    "    movl %esp, %ebp",

    // Save boot information (EAX=magic, EBX=info) on the scratch-backed stack.
    // These values persist through all function calls and are later read by
    // _nanvix_dispatch as KernelArguments.
    "    push %ebx",
    "    push %eax",

    // Install a minimal early IDT with only the page-fault vector (14) wired to
    // the CoW handler stub.  Any write to a snapshot (read-only) page triggers a
    // CoW page fault which the handler resolves by allocating a scratch frame,
    // copying the page, and remapping the PTE as writable.
    "    call install_early_idt",

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
