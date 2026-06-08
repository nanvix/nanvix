// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// x86_64 Bootstrap Entry Point
//==================================================================================================

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

// _do_start (entered directly in 64-bit long mode from the VMM).
global_asm!(
    r#".section .bootstrap,"ax",@progbits"#,

    ".align 8",
    ".globl _do_start",
    "_do_start:",

    // RAX and RBX registers store boot information. Save them before
    // they are overwritten.
    "    movq %rax, %r12",
    "    movq %rbx, %r13",

    // Fill BSS section with zeros (skipped on microvm backends; see CLEAR_BSS).
    ".if {CLEAR_BSS}",
    "    movq $__BSS_START, %rdi",
    "    movq $__BSS_END, %rcx",
    "    subq %rdi, %rcx",
    "    xorq %rax, %rax",
    "    cld",
    "    rep stosb",
    ".endif",

    // The boot stack guard page is pre-filled with the watermark pattern at link time (see
    // the `.data` block below), so no runtime initialization loop is required here.

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
