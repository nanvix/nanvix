// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use core::arch::global_asm;

const SCRATCH_SIZE_GVA: u32 = 0xFFFF_FFF8;
const ALLOCATOR_GVA: u32 = 0xFFFF_FFF0;

const PTE_RW: u32 = 1 << 1;
const PTE_COW: u32 = 1 << 9;
const PTE_ADDR_MASK: u32 = 0xFFFFF000;

global_asm!(
    r#".section .bootstrap,"ax",@progbits"#,
    ".code32",
    ".align 4",
    ".globl _hyperlight_entry",
    "_hyperlight_entry:",

    // Copy GDT to a temporary scratch page so the CPU can set the
    // accessed bit in segment descriptors without faulting. The
    // snapshot is read-only; a segment load would trigger an EPT
    // violation before the CoW handler is installed. Hal::init()
    // later relocates the GDT to its permanent scratch-reserved slot.
    "    .extern gdt",
    "    movl $gdt, %esi",
    "    movl $0xFFFDF000, %edi",
    "    movl $6, %ecx",
    "    cld",
    "    rep movsl",
    "    subl $8, %esp",
    "    movw $23, (%esp)",
    "    movl $0xFFFDF000, 2(%esp)",
    "    lgdt (%esp)",
    "    addl $8, %esp",
    "    ljmp $0x08, $.Lgdt_reloaded",
    ".Lgdt_reloaded:",
    "    movl $0x10, %eax",
    "    movl %eax, %ds",
    "    movl %eax, %es",
    "    movl %eax, %ss",
    "    movl %eax, %fs",
    "    movl %eax, %gs",

    // If scratch_size is zero, no CoW — skip IDT setup.
    "    movl ${SCRATCH_SIZE_GVA}, %eax",
    "    movl (%eax), %ecx",
    "    testl %ecx, %ecx",
    "    jz _do_start2",

    // Set up a temporary stack in the scratch I/O page for the
    // IDT construction code below. The 32-byte gap avoids the
    // scratch_size and allocator values at the top of the page.
    "    movl ${SCRATCH_SIZE_GVA}, %esp",
    "    subl $0x20, %esp",

    // Build IDT at 0xFFFE0000 with only entry #14 (#PF).
    "    movl $0xFFFE0000, %esi",
    "    movl %esi, %edi",
    "    xorl %eax, %eax",
    "    movl $512, %ecx",
    "    cld",
    "    rep stosl",
    "    movl $_cow_pf_handler, %eax",
    "    movw %ax, 112(%esi)",
    "    movw $0x08, 114(%esi)",
    "    movb $0, 116(%esi)",
    "    movb $0x8E, 117(%esi)",
    "    shrl $16, %eax",
    "    movw %ax, 118(%esi)",
    "    subl $8, %esp",
    "    movw $2047, (%esp)",
    "    movl %esi, 2(%esp)",
    "    lidt (%esp)",
    "    addl $8, %esp",

    // Pre-fault all kstack pages (top-down, stop before guard).
    "    .extern kstack",
    "    .extern kstack_guard",
    "    movl $kstack, %eax",
    "    movl $kstack_guard, %ecx",
    "    movl $0, -4(%eax)",
    ".Lprefault_loop:",
    "    subl $4096, %eax",
    "    cmpl %ecx, %eax",
    "    jb .Lprefault_done",
    "    movl $0, (%eax)",
    "    jmp .Lprefault_loop",
    ".Lprefault_done:",

    // Pre-fault CoW pages and fill in missing PTEs in PD entries 0-2.
    // Page tables are writable during evolve (first boot), so direct PTE
    // writes are safe — no CoW handler needed for the page table pages.
    "    movl %cr3, %esi",
    "    andl $0xFFFFF000, %esi",
    "    xorl %ebx, %ebx",
    ".Lpf_pd_loop:",
    "    cmpl $4, %ebx",
    "    jge .Lpf_pd_done",
    "    movl (%esi, %ebx, 4), %eax",
    "    testl $1, %eax",
    "    jz .Lpf_pd_next",
    "    andl $0xFFFFF000, %eax",
    "    xorl %ecx, %ecx",
    ".Lpf_pt_loop:",
    "    cmpl $1024, %ecx",
    "    jge .Lpf_pd_next",
    "    movl (%eax, %ecx, 4), %edx",
    "    testl $1, %edx",
    "    jz .Lpf_fill_missing",
    "    testl ${PTE_COW}, %edx",
    "    jz .Lpf_pt_next",
    // CoW page: read-then-write to trigger CoW resolution.
    "    pushl %eax",
    "    pushl %ecx",
    "    movl %ebx, %edi",
    "    shll $22, %edi",
    "    movl %ecx, %edx",
    "    shll $12, %edx",
    "    orl %edx, %edi",
    "    movl (%edi), %edx",
    "    movl %edx, (%edi)",
    "    popl %ecx",
    "    popl %eax",
    "    jmp .Lpf_pt_next",
    // Non-present PTE: create identity mapping (present + read-only).
    ".Lpf_fill_missing:",
    "    movl %ebx, %edx",
    "    shll $22, %edx",
    "    movl %ecx, %edi",
    "    shll $12, %edi",
    "    orl %edi, %edx",
    "    orl $1, %edx",
    "    movl %edx, (%eax, %ecx, 4)",
    ".Lpf_pt_next:",
    "    incl %ecx",
    "    jmp .Lpf_pt_loop",
    ".Lpf_pd_next:",
    "    incl %ebx",
    "    jmp .Lpf_pd_loop",
    ".Lpf_pd_done:",
    "    movl %cr3, %eax",
    "    movl %eax, %cr3",

    "    jmp _do_start2",

    // CoW Page Fault Handler for i686 2-level page tables.
    ".code32",
    ".align 4",
    ".globl _cow_pf_handler",
    "_cow_pf_handler:",
    "    pushl %eax",
    "    pushl %ecx",
    "    pushl %edx",
    "    pushl %ebx",
    "    pushl %esi",
    "    pushl %edi",

    "    movl %cr2, %edi",
    "    andl $0xFFFFF000, %edi",

    "    movl 24(%esp), %eax",
    "    testl $1, %eax",
    "    jz .Lcow_unhandled",
    "    testl $2, %eax",
    "    jz .Lcow_unhandled",

    "    movl %cr3, %eax",
    "    andl $0xFFFFF000, %eax",

    "    movl %edi, %ebx",
    "    shrl $22, %ebx",
    "    andl $0x3FF, %ebx",
    "    movl (%eax, %ebx, 4), %eax",
    "    testl $1, %eax",
    "    jz .Lcow_unhandled",
    "    andl ${PTE_ADDR_MASK}, %eax",

    "    movl %edi, %ebx",
    "    shrl $12, %ebx",
    "    andl $0x3FF, %ebx",
    "    leal (%eax, %ebx, 4), %esi",
    "    movl (%esi), %eax",

    "    testl ${PTE_COW}, %eax",
    "    jnz .Lcow_resolve",
    "    testl ${PTE_RW}, %eax",
    "    jz .Lcow_unhandled",
    ".Lcow_resolve:",

    "    movl %eax, %edx",

    "    movl ${ALLOCATOR_GVA}, %ebx",
    "    movl $4096, %eax",
    "    lock xaddl %eax, (%ebx)",
    "    cmpl $0xFFFDF000, %eax",
    "    ja .Lcow_oom",
    "    movl %eax, %ecx",

    "    movl %eax, %ebx",
    "    movl %edi, %eax",
    "    pushl %esi",
    "    movl %ebx, %edi",
    "    movl %eax, %esi",
    "    movl $1024, %ecx",
    "    cld",
    "    rep movsl",
    "    popl %esi",

    "    movl %edx, %eax",
    "    andl $0x00000FFF, %eax",
    "    andl $~{PTE_COW}, %eax",
    "    orl ${PTE_RW}, %eax",
    "    andl ${PTE_ADDR_MASK}, %ebx",
    "    orl %ebx, %eax",

    "    movl %eax, (%esi)",

    "    movl %cr2, %edi",
    "    andl $0xFFFFF000, %edi",
    "    invlpg (%edi)",

    "    popl %edi",
    "    popl %esi",
    "    popl %ebx",
    "    popl %edx",
    "    popl %ecx",
    "    popl %eax",
    "    addl $4, %esp",
    "    iretl",

    ".Lcow_oom:",
    "    movw $102, %dx",
    "    movb $43, %al",
    "    outb %al, %dx",
    "1:  hlt",
    "    jmp 1b",

    ".Lcow_unhandled:",
    "    popl %edi",
    "    popl %esi",
    "    popl %ebx",
    "    popl %edx",
    "    popl %ecx",
    "    popl %eax",
    "    .extern _do_excp14",
    "    jmp _do_excp14",

    // Rust-backed CoW handler installed after HAL init.
    // Delegates to cow_handle_page_fault() which uses Nanvix's frame
    // allocator instead of the bootstrap bump allocator.
    ".code32",
    ".align 4",
    ".globl _cow_pf_handler_rust",
    "_cow_pf_handler_rust:",
    "    pushl %eax",
    "    pushl %ecx",
    "    pushl %edx",
    "    pushl %ebx",
    "    pushl %esi",
    "    pushl %edi",

    "    movl 24(%esp), %eax",
    "    pushl %eax",
    "    movl %cr2, %eax",
    "    pushl %eax",
    "    .extern cow_handle_page_fault",
    "    call cow_handle_page_fault",
    "    addl $8, %esp",

    "    testl %eax, %eax",
    "    jz .Lrust_cow_unhandled",

    "    popl %edi",
    "    popl %esi",
    "    popl %ebx",
    "    popl %edx",
    "    popl %ecx",
    "    popl %eax",
    "    addl $4, %esp",
    "    iretl",

    ".Lrust_cow_unhandled:",
    "    popl %edi",
    "    popl %esi",
    "    popl %ebx",
    "    popl %edx",
    "    popl %ecx",
    "    popl %eax",
    "    jmp _do_excp14",

    // Dispatch entry for evolve->call lifecycle.
    // After evolve, Hyperlight resumes the VM here via sandbox.call().
    // GDT, IDT, page tables, and scratch are all intact from the evolve
    // phase (no snapshot/restore zeroes memory). Just restore kstack and
    // call the Rust dispatch handler.
    ".code32",
    ".align 4",
    ".globl _nanvix_dispatch",
    "_nanvix_dispatch:",
    "    .extern kstack",
    "    movl $kstack, %esp",
    "    movl %esp, %ebp",
    "    .extern nanvix_dispatch_handler",
    "    call nanvix_dispatch_handler",
    "1:  hlt",
    "    jmp 1b",

    SCRATCH_SIZE_GVA = const SCRATCH_SIZE_GVA,
    ALLOCATOR_GVA = const ALLOCATOR_GVA,
    PTE_RW = const PTE_RW,
    PTE_COW = const PTE_COW,
    PTE_ADDR_MASK = const PTE_ADDR_MASK,
    options(att_syntax),
);
