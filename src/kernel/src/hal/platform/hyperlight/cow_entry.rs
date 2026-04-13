// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Hyperlight entry point and Copy-on-Write page fault handler for i686 guests.
//!
//! When Hyperlight starts the guest in 32-bit protected mode with paging,
//! writable snapshot pages are marked Copy-on-Write (PTE bit 9 set, write bit
//! cleared). This module provides:
//!
//! 1. `_hyperlight_entry` — The ELF entry point that sets up a minimal IDT
//!    with a CoW-resolving #PF handler, then jumps to `_do_start2`.
//!
//! 2. `_cow_pf_handler` — Assembly page fault handler that walks the 2-level
//!    page table, detects CoW pages, allocates from the scratch bump allocator,
//!    copies page content, and updates the PTE.
//!
//! Scratch memory layout (i686, at top of 32-bit address space):
//!   MAX_GVA - 0x08 + 1: scratch_size (u64)
//!   MAX_GVA - 0x10 + 1: allocator state (u64, bump pointer = next free GPA)
//!   MAX_GVA - 0x20 + 1: exception stack start (grows downward)
//!
//! i686 page table format (2-level, non-PAE):
//!   PD index = (VA >> 22) & 0x3FF — 1024 entries × 4 bytes
//!   PT index = (VA >> 12) & 0x3FF — 1024 entries × 4 bytes
//!   PTE: [31:12] = physical frame, [9] = CoW bit, [1] = RW, [0] = Present

use core::arch::global_asm;

// Force the linker to include this module's assembly even though
// nothing in Rust references the symbols directly.
#[unsafe(no_mangle)]
#[used]
static _COW_ENTRY_ANCHOR: u8 = 0;

// Scratch memory metadata addresses (i686: MAX_GVA = 0xFFFFFFFF)
const SCRATCH_SIZE_GVA: u32 = 0xFFFF_FFF8; // MAX_GVA - 0x08 + 1
const ALLOCATOR_GVA: u32 = 0xFFFF_FFF0; // MAX_GVA - 0x10 + 1

// PTE flags
const _PTE_PRESENT: u32 = 1;
const PTE_RW: u32 = 1 << 1;
const PTE_COW: u32 = 1 << 9;
const PTE_ADDR_MASK: u32 = 0xFFFFF000;

global_asm!(
    // Hyperlight entry point for i686 guests.
    // Called in 32-bit protected mode with paging enabled (CR0.PE+PG).
    // Sets up a minimal IDT with a CoW #PF handler on the scratch stack,
    // then falls through to _do_start2.
    r#".section .bootstrap,"ax",@progbits"#,
    ".code32",
    ".align 4",
    ".globl _hyperlight_entry",
    "_hyperlight_entry:",

    // Copy the trampoline GDT to scratch memory so the CPU can set the
    // "accessed" bit in segment descriptors without triggering CoW faults.
    // Without this, exception delivery fails (GDT write → nested #PF → triple fault).
    "    .extern gdt",
    "    movl $gdt, %esi",
    "    movl $0xFFFDF000, %edi",  // scratch GDT location
    "    movl $6, %ecx",           // 24 bytes / 4 = 6 dwords
    "    cld",
    "    rep movsl",
    // Build GDTR descriptor on scratch stack pointing to scratch GDT
    "    subl $8, %esp",
    "    movw $23, (%esp)",        // limit = 24 - 1
    "    movl $0xFFFDF000, 2(%esp)", // base = scratch GDT
    "    lgdt (%esp)",
    "    addl $8, %esp",
    // Reload CS via far jump to use the new GDT
    "    ljmp $0x08, $.Lgdt_reloaded",
    ".Lgdt_reloaded:",
    "    movl $0x10, %eax",
    "    movl %eax, %ds",
    "    movl %eax, %es",
    "    movl %eax, %ss",
    "    movl %eax, %fs",
    "    movl %eax, %gs",

    // Check if CoW is active (scratch_size > 0)
    "    movl ${SCRATCH_SIZE_GVA}, %eax",
    "    movl (%eax), %ecx",       // ecx = scratch_size (low 32 bits)
    "    testl %ecx, %ecx",
    "    jz _do_start2",           // No scratch → no CoW → skip IDT setup

    // Compute scratch stack pointer: use the exception stack area
    // at SCRATCH_SIZE_GVA - 0x18 (below the metadata)
    "    movl ${SCRATCH_SIZE_GVA}, %esp",
    "    subl $0x20, %esp",        // ESP = exception stack area

    // Set up scratch stack for LIDT descriptor and exception handling.
    "    movl ${SCRATCH_SIZE_GVA}, %esp",
    "    subl $0x20, %esp",

    // Build IDT at a fixed scratch location.
    "    movl $0xFFFE0000, %esi",  // IDT base (fixed in scratch)
    "    movl %esi, %edi",
    "    xorl %eax, %eax",
    "    movl $512, %ecx",         // 2048 / 4 = 512 dwords
    "    cld",
    "    rep stosl",

    // Fill ONLY IDT entry #14 (#PF) with the CoW handler.
    // Only #PF pushes an error code in the format the handler expects.
    // Other exceptions are left empty (not-present) so they triple-fault
    // and get reported as a host VM exit.
    "    movl $_cow_pf_handler, %eax",
    "    movw %ax, 112(%esi)",     // entry #14 at offset 14×8=112
    "    movw $0x08, 114(%esi)",   // selector = code segment
    "    movb $0, 116(%esi)",
    "    movb $0x8E, 117(%esi)",   // type_attr = interrupt gate
    "    shrl $16, %eax",
    "    movw %ax, 118(%esi)",     // offset[31:16]

    // Load the IDT
    "    subl $8, %esp",           // IDTR descriptor on stack
    "    movw $2047, (%esp)",      // limit = 256×8 - 1
    "    movl %esi, 2(%esp)",      // base = IDT address
    "    lidt (%esp)",
    "    addl $8, %esp",

    // Pre-fault the boot stack so it's CoW'd to scratch before _do_start2
    // uses it. Do NOT pre-fault the guard page — it must remain read-only
    // so stack overflows are caught by the normal #PF handler.
    "    .extern kstack",
    "    movl $kstack, %eax",
    "    movl $0, -4(%eax)",       // Fault top of kstack page

    // Jump to the regular bootstrap entry (already in protected mode)
    "    jmp _do_start2",

    // ================================================================
    // CoW Page Fault Handler for i686 2-level page tables.
    //
    // On entry (CPU-pushed):
    //   [error_code]  <- ESP
    //   [EIP] [CS] [EFLAGS]
    //
    // Uses scratch memory for the bump allocator and page copies.
    // ================================================================
    ".code32",
    ".align 4",
    ".globl _cow_pf_handler",
    "_cow_pf_handler:",
    // Save registers
    "    pushl %eax",
    "    pushl %ecx",
    "    pushl %edx",
    "    pushl %ebx",
    "    pushl %esi",
    "    pushl %edi",

    // Stack layout: edi(0) esi(4) ebx(8) edx(12) ecx(16) eax(20) err(24) eip(28) cs(32) eflags(36)

    // Get faulting address from CR2, page-align it
    "    movl %cr2, %edi",         // edi = faulting VA
    "    andl $0xFFFFF000, %edi",  // page-align

    // Check error code: must be present(1) + write(2). User mode is OK.
    "    movl 24(%esp), %eax",     // error code
    "    testl $1, %eax",          // Present?
    "    jz .Lcow_unhandled",
    "    testl $2, %eax",          // Write?
    "    jz .Lcow_unhandled",

    // Walk 2-level page table: CR3 → PD → PT
    "    movl %cr3, %eax",
    "    andl $0xFFFFF000, %eax",  // PD physical base

    // Compute scratch mapping offset for phys→virt conversion.
    // scratch_base_gpa = MAX_GPA - scratch_size + 1 = 0xFFFFFFFF - scratch_size + 1
    // scratch_base_gva = MAX_GVA - scratch_size + 1 (same for i686: GPA==GVA for scratch)
    // Since i686 identity-maps scratch, phys_to_virt is identity for scratch pages.
    // For non-scratch pages (snapshot region), phys==virt because the host
    // set up identity mapping in the page tables.
    // So we can just dereference physical addresses directly as virtual addresses.

    // PD index = (VA >> 22) & 0x3FF
    "    movl %edi, %ebx",
    "    shrl $22, %ebx",
    "    andl $0x3FF, %ebx",
    // Read PDE at PD_base + PD_index * 4
    "    movl (%eax, %ebx, 4), %eax",  // eax = PDE
    "    testl $1, %eax",               // Present?
    "    jz .Lcow_unhandled",
    "    andl ${PTE_ADDR_MASK}, %eax",  // eax = PT physical base

    // PT index = (VA >> 12) & 0x3FF
    "    movl %edi, %ebx",
    "    shrl $12, %ebx",
    "    andl $0x3FF, %ebx",
    // Compute PTE address
    "    leal (%eax, %ebx, 4), %esi",  // esi = PTE address (phys==virt, identity mapped)
    "    movl (%esi), %eax",            // eax = PTE value

    // Check CoW bit (bit 9). Also handle supervisor writes to
    // EPT-readonly pages (present+RW in PTE but EPT blocks write).
    // If PTE has RW bit set but no CoW bit, and the fault is
    // present+write, this is an EPT-readonly page — resolve anyway.
    "    testl ${PTE_COW}, %eax",
    "    jnz .Lcow_resolve",            // CoW bit set → resolve
    "    testl ${PTE_RW}, %eax",        // Not CoW — check if PTE has RW
    "    jz .Lcow_unhandled",           // Not writable → real fault
    // PTE says writable but EPT blocked → resolve as CoW anyway
    ".Lcow_resolve:",

    // === Resolve CoW: allocate, copy, update PTE ===
    "    movl %eax, %edx",             // edx = old PTE (save for flags)

    // Bump-allocate one page from scratch
    "    movl ${ALLOCATOR_GVA}, %ebx",
    "    movl $4096, %eax",
    "    lock xaddl %eax, (%ebx)",     // eax = new page GPA (old allocator value)
    // Check for allocator overflow (wraps past MAX_GVA metadata area)
    "    cmpl $0xFFFDF000, %eax",      // stop before GDT at 0xFFFDF000
    "    ja .Lcow_oom",                // out of scratch memory
    "    movl %eax, %ecx",             // ecx = new page GPA (save for PTE)

    // Copy 4KB: src = faulting page (edi), dst = new page (eax)
    // Since pages are identity-mapped, we can use the physical addresses directly.
    "    movl %eax, %ebx",             // ebx = dst (new page)
    "    movl %edi, %eax",             // eax = src (old page, still readable)
    "    pushl %esi",                   // save PTE address
    "    movl %ebx, %edi",             // edi = dst for rep movsl
    "    movl %eax, %esi",             // esi = src for rep movsl
    "    movl $1024, %ecx",            // 4096 / 4 = 1024 dwords
    "    cld",
    "    rep movsl",
    "    popl %esi",                    // restore PTE address

    // Build new PTE: new_page_gpa | old_flags | RW, clear CoW
    "    movl %edx, %eax",             // eax = old PTE
    "    andl $0x00000FFF, %eax",      // keep only flags
    "    andl $~{PTE_COW}, %eax",      // clear CoW bit
    "    orl ${PTE_RW}, %eax",         // set RW bit

    // ecx still has the new page GPA from the xaddl, but we used it for rep movsl.
    // Reload from saved value.
    "    movl %ebx, %ecx",             // Actually ebx was dst = new page GPA... wait
    // Let me reconsider register usage. After rep movsl:
    // edi = dst + 4096 (advanced past end), esi = PTE addr (restored from push/pop)
    // ebx = original dst (new page GPA)
    // So new page GPA is in ebx, but edi was clobbered by rep movsl.
    // ecx was clobbered by rep movsl (counted down to 0).
    // edx still has old PTE.

    // New PTE = new_page_gpa | flags
    "    andl ${PTE_ADDR_MASK}, %ebx", // ensure page-aligned
    "    orl %ebx, %eax",             // eax = new_page_gpa | flags

    // Write updated PTE
    "    movl %eax, (%esi)",

    // Reload faulting address for invlpg (edi was clobbered by rep movsl)
    "    movl %cr2, %edi",
    "    andl $0xFFFFF000, %edi",
    "    invlpg (%edi)",

    // Restore registers and return
    "    popl %edi",
    "    popl %esi",
    "    popl %ebx",
    "    popl %edx",
    "    popl %ecx",
    "    popl %eax",
    "    addl $4, %esp",               // skip error code
    "    iretl",

    ".Lcow_oom:",
    // Out of scratch memory — abort with code 43
    "    movw $102, %dx",
    "    movb $43, %al",           // exit 43 = CoW out of memory
    "    outb %al, %dx",
    "1:  hlt",
    "    jmp 1b",

    ".Lcow_unhandled:",
    // Not a CoW fault — delegate to the kernel's #PF handler.
    "    popl %edi",
    "    popl %esi",
    "    popl %ebx",
    "    popl %edx",
    "    popl %ecx",
    "    popl %eax",
    "    .extern _do_excp14",
    "    jmp _do_excp14",

    // ================================================================
    // Nanvix dispatch function for evolve→restore→call lifecycle.
    //
    // Called by Hyperlight after restore+call. Re-enters the kernel
    // event loop. On return, halts the VM via port 108.
    // ================================================================
    ".code32",
    ".align 4",
    ".globl _nanvix_dispatch",
    "_nanvix_dispatch:",
    "    movl ${SCRATCH_SIZE_GVA}, %esp",
    "    subl $0x20, %esp",

    // Copy kernel's GDT (via sgdt) to scratch, reload, ljmp.
    "    subl $8, %esp",
    "    sgdt (%esp)",
    "    movzwl (%esp), %ecx",
    "    movl 2(%esp), %esi",
    "    addl $8, %esp",
    "    movl $0xFFFDF000, %edi",
    "    addl $1, %ecx",
    "    pushl %ecx",
    "    addl $3, %ecx",
    "    shrl $2, %ecx",
    "    cld",
    "    rep movsl",
    "    popl %ecx",
    "    subl $1, %ecx",
    "    subl $8, %esp",
    "    movw %cx, (%esp)",
    "    movl $0xFFFDF000, 2(%esp)",
    "    lgdt (%esp)",
    "    addl $8, %esp",
    "    ljmp $0x08, $.Ldispatch_gdt_ok",
    ".Ldispatch_gdt_ok:",
    "    movl $0x10, %eax",
    "    movl %eax, %ds",
    "    movl %eax, %es",
    "    movl %eax, %ss",
    "    movl %eax, %fs",
    "    movl %eax, %gs",

    // Build scratch IDT with CoW #PF handler at entry #14.
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

    // Pre-fault ALL kstack CoW pages.
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

    // Pre-fault ALL CoW pages by walking the page tables.
    // This is required because interrupt delivery from ring 3→0 needs to
    // push frames onto the user process's kernel stack. If that stack is
    // CoW (read-only), the CPU can't push → nested fault → triple fault.
    "    movl %cr3, %esi",
    "    andl $0xFFFFF000, %esi",
    "    xorl %ebx, %ebx",
    ".Lpf_pd_loop:",
    // Only walk PD entries 0-2 (kernel text/data/BSS at 0x00000000-0x00BFFFFF).
    // Higher entries include user process pages and PEB — skip for now.
    "    cmpl $3, %ebx",
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
    "    jz .Lpf_pt_next",
    "    testl ${PTE_COW}, %edx",
    "    jz .Lpf_pt_next",
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
    ".Lpf_pt_next:",
    "    incl %ecx",
    "    jmp .Lpf_pt_loop",
    ".Lpf_pd_next:",
    "    incl %ebx",
    "    jmp .Lpf_pd_loop",
    ".Lpf_pd_done:",
    "    movl %cr3, %eax",
    "    movl %eax, %cr3",

    // kstack is now writable. Switch to it.
    "    movl $kstack, %esp",
    "    movl %esp, %ebp",

    // Clear TSS busy bit and reload TR.
    // TSS is at GDT entry 6 (selector 0x30), byte 5 at offset 53.
    "    movl $0xFFFDF000, %eax",
    "    andb $0xFD, 53(%eax)",
    "    movw $0x30, %ax",
    "    ltr %ax",

    // Clear CR0.TS to prevent #NM on FPU instructions.
    "    clts",

    // Call the Rust dispatch handler.
    "    .extern nanvix_dispatch_handler",
    "    call nanvix_dispatch_handler",
    // Unreachable.
    "1:  hlt",
    "    jmp 1b",

    SCRATCH_SIZE_GVA = const SCRATCH_SIZE_GVA,
    ALLOCATOR_GVA = const ALLOCATOR_GVA,
    PTE_RW = const PTE_RW,
    PTE_COW = const PTE_COW,
    PTE_ADDR_MASK = const PTE_ADDR_MASK,
    options(att_syntax),
);
