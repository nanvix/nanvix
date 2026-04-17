// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// x86 Bootstrap and Application-Processor Entry Points
//==================================================================================================

use super::constants;
use crate::hal::arch::x86::cpu::ContextInformation;
use ::arch::mem::PAGE_SIZE;
use ::core::arch::global_asm;

/// Whether to clear the BSS section during bootstrap (0 = skip, 1 = clear).
///
/// On microvm backends (KVM and WHP) the host has already zeroed guest memory before the
/// vCPU starts: `mmap(MAP_ANONYMOUS)` on Linux and `VirtualAlloc(MEM_COMMIT)` on Windows
/// both return zero-filled pages, and the ELF loader explicitly zeroes every BSS region
/// via `write_bytes()`. Skipping the guest-side `rep stosb` avoids touching every BSS
/// page from inside the VM, which would otherwise trigger expensive page/EPT-violation
/// faults for pages the kernel never actually reads during boot. In all other
/// configurations we must zero .bss here to ensure deterministic initialization of
/// static data.
const CLEAR_BSS: u32 = if cfg!(feature = "microvm") { 0 } else { 1 };

/// Whether to run the Hyperlight evolve phase before calling `kmain`.
///
/// On Hyperlight, the bootstrap code must halt the VM during `evolve()` before entering `kmain()`.
/// The host then calls `sandbox.call("kmain", ())` which re-enters the guest at `_nanvix_dispatch`,
/// restores the boot stack, and calls `kmain()` for real.
///
/// Evaluates to 0 on all other platforms; the assembler `.if` directive eliminates the
/// `call hyperlight_pre_kmain` instruction entirely at compile time.
///
const HYPERLIGHT_EVOLVE: u32 = if cfg!(feature = "hyperlight") { 1 } else { 0 };

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

    // Fill BSS section with zeros.
    "    movl %eax, %edx",

    // Fill BSS section with zeros (skipped on microvm backends; see CLEAR_BSS).
    ".if {CLEAR_BSS}",
    "    movl $__BSS_START, %edi",
    "    movl $__BSS_END, %ecx",
    "    subl %edi, %ecx",
    "    xorl %eax, %eax",
    "    rep stosb",
    ".endif",

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
    //
    // On Hyperlight, the evolve phase must halt the VM before kmain runs. `hyperlight_pre_kmain()`
    // registers the dispatch entry point and halts; it never returns. The host then calls
    // sandbox.call("kmain", ()) which enters `_nanvix_dispatch` → `kmain``.
    ".if {HYPERLIGHT_EVOLVE}",
    "    call hyperlight_pre_kmain",
    ".endif",
    "    push %esp",
    "    call kmain",
    "    addl $4, %esp",

    // Cleanup boot information.
    "    addl $8, %esp",

    // Halt execution.
    "1:  hlt",
    "    jmp 1b",

    CLEAR_BSS = const CLEAR_BSS,
    HYPERLIGHT_EVOLVE = const HYPERLIGHT_EVOLVE,
    PAGE_SIZE = const PAGE_SIZE,
    CONTEXT_HW_SIZE = const ContextInformation::CONTEXT_HW_SIZE,
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
