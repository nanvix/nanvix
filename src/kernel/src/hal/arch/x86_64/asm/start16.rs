// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// x86_64 16-bit → 32-bit → 64-bit Trampoline Code
//==================================================================================================
//
// This module contains the real-mode entry point placed in the `.trampoline` section at physical
// address 0x8000. It transitions through 32-bit protected mode to 64-bit long mode by:
//
// 1. Loading a flat 32-bit GDT and enabling protected mode (CR0.PE).
// 2. Building identity-mapped page tables (2 MiB pages covering 0–2 GiB) from a page-aligned
//    BSS area reserved in this module.
// 3. Enabling PAE (CR4.PAE), loading CR3, setting EFER.LME, and enabling paging (CR0.PG).
// 4. Loading a 64-bit GDT (L=1 code segment) and far-jumping to the 64-bit kernel entry.
//
// After the kernel finishes initializing its own 4 KiB page tables (via `virt::init()` and
// `PageMap::new_boot()`), the boot page tables built here are abandoned — the kernel switches
// CR3 to its own PML4 and these pages are never referenced again.
//

use core::arch::global_asm;

//==================================================================================================
// Addresses of page-aligned BSS regions used by the 32-bit trampoline code.
//
// These symbols are defined in the BSS section at the bottom of this file.
// The 32-bit code references them by name to build the boot page tables.
//==================================================================================================

//==================================================================================================
// Trampoline Section — BSP Entry (16-bit → 32-bit protected mode)
//==================================================================================================

global_asm!(
    r#".section .trampoline,"ax",@progbits"#,
    ".code16",
    ".align 4",
    ".globl _do_start",
    "_do_start:",
    // Zero data segment registers.
    "    xorw  %dx, %dx",
    "    movw  %dx, %ds",
    "    movw  %dx, %es",
    "    movw  %dx, %fs",
    "    movw  %dx, %gs",
    "    movw  %dx, %ss",
    // Load the 32-bit flat GDT and enable protected mode.
    "    lgdt  gdt32ptr",
    "    movl  %cr0, %edx",
    "    orl   $1, %edx",
    "    movl  %edx, %cr0",
    // Far-jump to 32-bit code (selector 0x08 = flat code segment).
    ".extern _do_start32",
    "    jmpl  $0x08, $_do_start32",
    options(att_syntax),
);

//==================================================================================================
// Trampoline Section — 32-bit GDT (used for the real → protected mode transition)
//==================================================================================================

global_asm!(
    r#".section .trampoline,"ax",@progbits"#,
    ".p2align 2",
    "gdt32:",
    // Null segment descriptor.
    "    .word 0, 0",
    "    .byte 0, 0, 0, 0",
    // 32-bit code segment: base=0, limit=0xFFFFFFFF, type=Execute+Read.
    "    .word 0xffff, 0x0000",
    "    .byte 0x00, 0x9a, 0xcf, 0x00",
    // 32-bit data segment: base=0, limit=0xFFFFFFFF, type=Read+Write.
    "    .word 0xffff, 0x0000",
    "    .byte 0x00, 0x92, 0xcf, 0x00",
    "gdt32ptr:",
    "    .word (gdt32ptr - gdt32 - 1)",
    "    .long gdt32",
    options(att_syntax),
);

//==================================================================================================
// Bootstrap Section — 32-bit → 64-bit transition
//==================================================================================================

global_asm!(
    r#".section .bootstrap,"ax",@progbits"#,
    ".code32",
    ".align 4",
    ".globl _do_start32",
    "_do_start32:",
    // Set 32-bit data segments (selector 0x10 = flat data segment from gdt32).
    "    movw  $0x10, %dx",
    "    movw  %dx, %ds",
    "    movw  %dx, %es",
    "    movw  %dx, %fs",
    "    movw  %dx, %gs",
    "    movw  %dx, %ss",
    // Save boot info (EAX=magic, EBX=initrd info) to EBP and EBX.
    // EBP and EBX are NOT clobbered by the page table construction below.
    // We restore them to ESI/EDI right before jumping to 64-bit code.
    "    movl  %eax, %ebp",
    "    movl  %ebx, %ebx",
    // ---------------------------------------------------------------
    // Build identity-mapped page tables (2 MiB pages, 0–2 GiB).
    //
    // Layout (BSS symbols, each 4 KiB page-aligned):
    //   boot_pml4  — PML4 (one entry: [0] → boot_pdpt)
    //   boot_pdpt  — PDPT (two entries: [0] → boot_pd0, [1] → boot_pd1)
    //   boot_pd0   — PD for [0, 1 GiB), supervisor-only, 2 MiB pages
    //   boot_pd1   — PD for [1 GiB, 2 GiB), user-accessible, 2 MiB pages
    // ---------------------------------------------------------------

    // Zero all four pages (4 × 4096 = 16384 bytes).
    "    movl  $boot_pml4, %edi",
    "    xorl  %eax, %eax",
    "    movl  $(4 * 4096 / 4), %ecx",
    "    cld",
    "    rep stosl",
    // PML4[0] → boot_pdpt | present(0x1) + writable(0x2) + user(0x4).
    "    movl  $boot_pdpt, %eax",
    "    orl   $0x07, %eax",
    "    movl  %eax, boot_pml4",
    // PDPT[0] → boot_pd0 | present(0x1) + writable(0x2) (supervisor).
    "    movl  $boot_pd0, %eax",
    "    orl   $0x03, %eax",
    "    movl  %eax, boot_pdpt",
    // PDPT[1] → boot_pd1 | present(0x1) + writable(0x2) + user(0x4).
    "    movl  $boot_pd1, %eax",
    "    orl   $0x07, %eax",
    "    movl  %eax, (boot_pdpt + 8)",
    // Fill PD0: 512 entries, each 2 MiB, supervisor-only.
    // Flags: present(0x1) + writable(0x2) + PS(0x80) = 0x83.
    "    movl  $boot_pd0, %edi",
    "    movl  $0x83, %eax", // first entry: physical 0x0 | flags
    "    xorl  %edx, %edx",  // high 32 bits = 0
    "    movl  $512, %ecx",
    "1:",
    "    movl  %eax, (%edi)",
    "    movl  %edx, 4(%edi)",
    "    addl  $0x200000, %eax", // next 2 MiB
    "    adcl  $0, %edx",
    "    addl  $8, %edi",
    "    decl  %ecx",
    "    jnz   1b",
    // Fill PD1: 512 entries, each 2 MiB, user-accessible.
    // Flags: present(0x1) + writable(0x2) + user(0x4) + PS(0x80) = 0x87.
    // Base physical address: 0x40000000 (1 GiB).
    "    movl  $boot_pd1, %edi",
    "    movl  $0x40000087, %eax", // 0x40000000 | 0x87
    "    xorl  %edx, %edx",
    "    movl  $512, %ecx",
    "2:",
    "    movl  %eax, (%edi)",
    "    movl  %edx, 4(%edi)",
    "    addl  $0x200000, %eax",
    "    adcl  $0, %edx",
    "    addl  $8, %edi",
    "    decl  %ecx",
    "    jnz   2b",
    // ---------------------------------------------------------------
    // Enable PAE.
    // ---------------------------------------------------------------
    "    movl  %cr4, %eax",
    "    orl   $(1 << 5), %eax", // CR4.PAE
    "    movl  %eax, %cr4",
    // ---------------------------------------------------------------
    // Load CR3 with boot PML4.
    // ---------------------------------------------------------------
    "    movl  $boot_pml4, %eax",
    "    movl  %eax, %cr3",
    // ---------------------------------------------------------------
    // Enable long mode via EFER MSR.
    // ---------------------------------------------------------------
    "    movl  $0xC0000080, %ecx", // IA32_EFER MSR
    "    rdmsr",
    "    orl   $(1 << 8), %eax", // EFER.LME
    "    wrmsr",
    // ---------------------------------------------------------------
    // Enable paging (enters long mode compatibility sub-mode).
    // ---------------------------------------------------------------
    "    movl  %cr0, %eax",
    "    orl   $(1 << 31), %eax", // CR0.PG
    "    orl   $(1 << 5), %eax",  // CR0.NE (numeric error)
    "    movl  %eax, %cr0",
    // ---------------------------------------------------------------
    // Restore boot info to ESI/EDI for the 64-bit entry point.
    // EBP=magic (was EAX), EBX=initrd info (was EBX).
    // ---------------------------------------------------------------
    "    movl  %ebp, %esi",
    "    movl  %ebx, %edi",
    // ---------------------------------------------------------------
    // Load 64-bit GDT and far-jump to 64-bit code.
    // ---------------------------------------------------------------
    "    lgdt  gdt64ptr",
    "    ljmpl $0x08, $_do_start64",
    options(att_syntax),
);

//==================================================================================================
// Bootstrap Section — 64-bit GDT (used after enabling long mode)
//==================================================================================================

global_asm!(
    r#".section .bootstrap,"ax",@progbits"#,
    ".p2align 3",
    "gdt64:",
    // Entry 0: null descriptor.
    "    .quad 0x0000000000000000",
    // Entry 1 (selector 0x08): 64-bit kernel code (L=1, D=0, P=1, DPL=0).
    "    .quad 0x00AF9A000000FFFF",
    // Entry 2 (selector 0x10): kernel data (P=1, DPL=0, writable).
    "    .quad 0x00CF92000000FFFF",
    "gdt64ptr:",
    "    .word (gdt64ptr - gdt64 - 1)",
    "    .long gdt64",
    options(att_syntax),
);

//==================================================================================================
// BSS Section — Boot Page Tables (abandoned after kernel switches to its own PML4)
//==================================================================================================

global_asm!(
    ".section .bss",
    // Boot PML4 (4 KiB, page-aligned).
    ".align 4096",
    ".globl boot_pml4",
    "boot_pml4:",
    ".space 4096",
    // Boot PDPT (4 KiB, page-aligned).
    ".align 4096",
    ".globl boot_pdpt",
    "boot_pdpt:",
    ".space 4096",
    // Boot PD0 — maps [0, 1 GiB) supervisor (4 KiB, page-aligned).
    ".align 4096",
    ".globl boot_pd0",
    "boot_pd0:",
    ".space 4096",
    // Boot PD1 — maps [1 GiB, 2 GiB) user (4 KiB, page-aligned).
    ".align 4096",
    ".globl boot_pd1",
    "boot_pd1:",
    ".space 4096",
    options(att_syntax),
);
