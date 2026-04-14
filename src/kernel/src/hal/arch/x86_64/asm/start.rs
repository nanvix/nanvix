// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// x86_64 Bootstrap Entry Point
//==================================================================================================

use super::constants;
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

//==================================================================================================
// Bootstrap Section — BSP Entry Point
//==================================================================================================

// _do_start64 (entered from the trampoline after the 32-bit → 64-bit transition).
global_asm!(
    r#".section .bootstrap,"ax",@progbits"#,

    ".code64",
    ".align 8",
    ".globl _do_start64",
    "_do_start64:",

    // Reload data segment registers with the 64-bit GDT data selector (0x10).
    "    movw  $0x10, %ax",
    "    movw  %ax, %ds",
    "    movw  %ax, %es",
    "    movw  $0x00, %ax",
    "    movw  %ax, %fs",
    "    movw  %ax, %gs",
    "    movw  $0x10, %ax",
    "    movw  %ax, %ss",

    // Boot information was saved to ESI (=RAX) and EDI (=RBX) by the 32-bit
    // trampoline code. Move them to callee-saved R12/R13 before BSS is cleared.
    "    movq %rsi, %r12",
    "    movq %rdi, %r13",

    // Fill BSS section with zeros (skipped on microvm backends; see CLEAR_BSS).
    ".if {CLEAR_BSS}",
    "    movq $__BSS_START, %rdi",
    "    movq $__BSS_END, %rcx",
    "    subq %rdi, %rcx",
    "    xorq %rax, %rax",
    "    cld",
    "    rep stosb",
    ".endif",

    // Fill the boot stack guard page with a watermark pattern.
    "    movq $kstack_guard, %rdi",
    "    movq $({PAGE_SIZE} / 4), %rcx",
    "    movl ${KSTACK_GUARD_PATTERN}, %eax",
    "    rep stosl",

    // Reset stack.
    "    movq $kstack, %rsp",
    "    movq %rsp, %rbp",

    // Clear volatile general purpose registers for deterministic startup.
    // R12 and R13 are intentionally preserved: they hold boot information
    // (RAX=boot magic, RBX=initrd info) saved above.
    "    xorq %rax, %rax",
    "    xorq %rbx, %rbx",
    "    xorq %rcx, %rcx",
    "    xorq %rdx, %rdx",
    "    xorq %rsi, %rsi",
    "    xorq %rdi, %rdi",
    "    xorq %r8, %r8",
    "    xorq %r9, %r9",
    "    xorq %r10, %r10",
    "    xorq %r11, %r11",
    "    xorq %r14, %r14",
    "    xorq %r15, %r15",

    // Call kernel main function.
    // Push boot info onto the stack and pass RSP as first argument
    // (x86_64 SysV ABI: first arg in RDI).
    "    pushq %r13",       // RBX value (initrd info).
    "    pushq %r12",       // RAX value (boot magic).
    "    movq %rsp, %rdi",

    "    call kmain",
    "    addq $16, %rsp",

    // Halt execution.
    "1:  hlt",
    "    jmp 1b",

    CLEAR_BSS = const CLEAR_BSS,
    PAGE_SIZE = const PAGE_SIZE,
    KSTACK_GUARD_PATTERN = const constants::KSTACK_GUARD_PATTERN,
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
