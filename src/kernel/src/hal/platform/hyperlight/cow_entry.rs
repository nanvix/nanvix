// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Early CoW Page Fault Entry Point and IDT Installation
//==================================================================================================
//
// This module provides:
//
// 1. `_cow_page_fault_entry` — a minimal assembly stub that intercepts page faults (vector 14)
//    and delegates to the Rust CoW handler [`super::cow_handler::try_handle_cow_page_fault`].
//    If the fault is not a CoW fault the stub chains to the standard `_do_excp14` hook so the
//    normal exception dispatching path is followed.
//
// 2. `install_early_idt()` — builds a minimal IDT in the scratch region (writable from boot)
//    with only vector 14 wired to `_cow_page_fault_entry`, then loads IDTR.  The scratch
//    region is outside the snapshot, so these writes never cause CoW page faults.
//
// After HAL initialization the standard IDT (populated by `idt::init()`) takes over. The
// post-init CoW interception is handled by the `do_exception` modification in the arch-level
// exception controller (see `controller.rs`), which calls the same Rust handler through the
// existing high-level exception dispatching path — no extra assembly required.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86::mem::gdt;
use ::arch::cpu::{
    excp::Exception,
    idt::{
        DescriptorPrivilegeLevel,
        Flags,
        GateType,
        Idte,
        PresentBit,
    },
    idtr::Idtr,
};
use ::core::arch::global_asm;

//==================================================================================================
// Constants
//==================================================================================================

/// Kernel code segment selector (GDT entry 1 << 3 = 0x08).
const KERNEL_CS: u16 = crate::hal::arch::x86::mem::gdt::SegmentSelector::KernelCode as u16;

/// Page fault exception vector number.
const PAGE_FAULT_VECTOR: usize = Exception::PageFault as usize;

/// Number of entries in the early IDT. Must cover up to and including [`PAGE_FAULT_VECTOR`].
const EARLY_IDT_LEN: usize = PAGE_FAULT_VECTOR + 1;

//==================================================================================================
// Assembly Stub
//==================================================================================================

global_asm!(
    r#".section .text,"ax",@progbits"#,
    ".code32",
    // =================================================================
    // _cow_page_fault_entry — Minimal page-fault handler stub (vector 14)
    // =================================================================
    //
    // On entry the hardware has pushed: [error_code, EIP, CS, EFLAGS, ...].
    // The stub saves GPRs, extracts error_code and CR2, and calls the Rust
    // try_handle_cow_page_fault(fault_addr, error_code).
    //
    // If the Rust handler returns true (CoW resolved): restore, skip error code, iret.
    // If false: restore and chain to _do_excp14 for normal exception dispatch.
    //
    ".align 4",
    ".globl _cow_page_fault_entry",
    "_cow_page_fault_entry:",
    // Debug: output '!' to confirm handler entry.
    "    pushl %eax",
    "    pushl %edx",
    "    mov $103, %dx",
    "    mov $0x21, %al", // '!'
    "    outb %al, %dx",
    "    mov $0x0A, %al", // '\n'
    "    outb %al, %dx",
    "    popl %edx",
    "    popl %eax",
    // Save caller-saved registers.
    "    pushl %eax",
    "    pushl %ecx",
    "    pushl %edx",
    "    pushl %ebx",
    "    pushl %esi",
    "    pushl %edi",
    // Extract error code: 6 pushes * 4 bytes = 24 bytes above ESP.
    "    movl 24(%esp), %eax",
    // Extract faulting address from CR2.
    "    movl %cr2, %ecx",
    // Call try_handle_cow_page_fault(fault_addr: u32, error_code: u32).
    // System V i386: arguments pushed right-to-left.
    "    pushl %eax", // error_code
    "    pushl %ecx", // fault_addr
    "    .extern try_handle_cow_page_fault",
    "    call try_handle_cow_page_fault",
    "    addl $8, %esp",
    // Check return value.
    "    testl %eax, %eax",
    "    jz .Lcow_not_handled",
    // CoW resolved — restore registers, skip error code, iret.
    "    popl %edi",
    "    popl %esi",
    "    popl %ebx",
    "    popl %edx",
    "    popl %ecx",
    "    popl %eax",
    "    addl $4, %esp", // Skip hardware error code.
    "    iretl",
    ".Lcow_not_handled:",
    // Not a CoW fault — during early boot this is unrecoverable (no exception
    // infrastructure exists yet). Issue ud2 to immediately triple-fault with a
    // distinguishable signature rather than silently chaining to uninitialized code.
    "    ud2",
    //
    // --- Diagnostic stubs for non-PF exceptions during early boot ---
    //
    ".align 4",
    ".globl _early_gp_entry",
    "_early_gp_entry:",
    "    addl $4, %esp", // skip error code
    "    mov $103, %dx",
    "    mov $0x47, %al", // 'G'
    "    outb %al, %dx",
    "    mov $0x50, %al", // 'P'
    "    outb %al, %dx",
    "    mov $0x0A, %al",
    "    outb %al, %dx",
    "1:  hlt",
    "    jmp 1b",
    //
    ".align 4",
    ".globl _early_df_entry",
    "_early_df_entry:",
    "    addl $4, %esp", // skip error code
    "    mov $103, %dx",
    "    mov $0x44, %al", // 'D'
    "    outb %al, %dx",
    "    mov $0x46, %al", // 'F'
    "    outb %al, %dx",
    "    mov $0x0A, %al",
    "    outb %al, %dx",
    "1:  hlt",
    "    jmp 1b",
    //
    ".align 4",
    ".globl _early_ss_entry",
    "_early_ss_entry:",
    "    addl $4, %esp", // skip error code
    "    mov $103, %dx",
    "    mov $0x53, %al", // 'S'
    "    outb %al, %dx",
    "    mov $0x53, %al", // 'S'
    "    outb %al, %dx",
    "    mov $0x0A, %al",
    "    outb %al, %dx",
    "1:  hlt",
    "    jmp 1b",
    //
    ".align 4",
    ".globl _early_ud_entry",
    "_early_ud_entry:",
    "    mov $103, %dx",
    "    mov $0x55, %al", // 'U'
    "    outb %al, %dx",
    "    mov $0x44, %al", // 'D'
    "    outb %al, %dx",
    "    mov $0x0A, %al",
    "    outb %al, %dx",
    "1:  hlt",
    "    jmp 1b",
    options(att_syntax),
);

//==================================================================================================
// Standalone Functions
//==================================================================================================

unsafe extern "C" {
    /// Early CoW page fault assembly stub.
    fn _cow_page_fault_entry();
    fn _early_gp_entry();
    fn _early_df_entry();
    fn _early_ss_entry();
    fn _early_ud_entry();
}

///
/// # Description
///
/// Installs a minimal early-boot IDT with only vector 14 (page fault) wired to the CoW handler
/// stub [`_cow_page_fault_entry`].
///
/// The IDT is written to the scratch region at a compile-time known address
/// ([`HYPERLIGHT_EARLY_IDT_BASE`](::config::memory_layout::HYPERLIGHT_EARLY_IDT_BASE)).
/// The scratch region is always writable (it is outside the CoW snapshot), so these writes
/// never trigger page faults.  Scratch memory is zeroed by Hyperlight before the guest starts,
/// so entries 0–13 are already not-present.
///
/// All other vectors are left as not-present; any exception other than #PF during early boot will
/// triple-fault (which is the correct behavior — there is no meaningful recovery path before the
/// full HAL is initialized).
///
/// # Safety
///
/// This function is unsafe because it:
/// - Writes to memory at `HYPERLIGHT_EARLY_IDT_BASE` (scratch region).
/// - Executes the `lidt` instruction to load a new IDT.
///
/// It must be called exactly once, from `_do_start2`, after the stack is set to
/// `HYPERLIGHT_BOOT_STACK_TOP` and before `init_scratch_kstack()`.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn install_early_idt() {
    use ::config::memory_layout::HYPERLIGHT_EARLY_IDT_BASE;

    // ---- Step 1: Relocate GDT to writable scratch memory ----
    //
    // The Hyperlight VMM starts the guest with GDTR base=0, limit=0 (no usable GDT in
    // memory). Segment descriptors are cached in hidden registers, so normal code runs
    // fine. However, when the CPU delivers an exception it reloads CS from the GDT and
    // writes the Accessed bit into the descriptor. With no writable GDT the CS load
    // faults, causing a double fault, then a triple fault — the page-fault handler
    // never runs.
    //
    // This MUST happen before the IDT is loaded so that exception delivery works from
    // the very first moment the IDT is active.
    let new_gdt_base: usize = ::config::memory_layout::HYPERLIGHT_EARLY_GDT_BASE;
    let gdt_dst: *mut ::arch::mem::gdt::Gdte = new_gdt_base as *mut ::arch::mem::gdt::Gdte;

    // Copy DEFAULT_ENTRIES (const in rodata, readable) to scratch (writable).
    core::ptr::copy_nonoverlapping(gdt::DEFAULT_ENTRIES.as_ptr(), gdt_dst, gdt::GDT_NUM_ENTRIES);

    // Build and load GDTR pointing to the scratch copy.
    let gdt_byte_len: u16 =
        (gdt::GDT_NUM_ENTRIES * core::mem::size_of::<::arch::mem::gdt::Gdte>()) as u16;
    #[repr(C, packed)]
    struct GdtrDesc {
        limit: u16,
        base: u32,
    }
    let gdtr = GdtrDesc {
        limit: gdt_byte_len - 1,
        base: new_gdt_base as u32,
    };
    core::arch::asm!(
        "lgdt [{ptr}]",
        ptr = in(reg) &gdtr as *const GdtrDesc,
        options(nostack)
    );

    // Reload CS via a far jump so the CPU fetches the descriptor from the new GDT.
    // DS/ES/SS already have cached descriptors that match selector 0x10 in the new
    // GDT, so they don't need reloading (the CPU only accesses the GDT for them on
    // an explicit MOV to segment register).
    core::arch::asm!("ljmp $0x08, $2f", "2:", options(att_syntax, nostack));

    // ---- Step 2: Install minimal early IDT ----

    let idt_base: usize = HYPERLIGHT_EARLY_IDT_BASE;

    // Scratch is zeroed by Hyperlight, so entries 0–13 are already not-present.
    // Write only the page fault entry.
    let handler_addr: u32 = _cow_page_fault_entry as *const () as u32;
    let entry: Idte = Idte::new(
        handler_addr,
        KERNEL_CS,
        Flags::new(PresentBit::Present, DescriptorPrivilegeLevel::Ring0, GateType::Int32),
    );

    let entry_offset: usize = PAGE_FAULT_VECTOR * core::mem::size_of::<Idte>();
    let entry_ptr: *mut Idte = (idt_base + entry_offset) as *mut Idte;
    core::ptr::write(entry_ptr, entry);

    // Register diagnostic handlers for other common exception vectors.
    let idte_size: usize = core::mem::size_of::<Idte>();

    // Vector 6: #UD (Invalid Opcode)
    let ud_entry = Idte::new(
        _early_ud_entry as *const () as u32,
        KERNEL_CS,
        Flags::new(PresentBit::Present, DescriptorPrivilegeLevel::Ring0, GateType::Int32),
    );
    core::ptr::write((idt_base + 6 * idte_size) as *mut Idte, ud_entry);

    // Vector 8: #DF (Double Fault) — has error code (always 0)
    let df_entry = Idte::new(
        _early_df_entry as *const () as u32,
        KERNEL_CS,
        Flags::new(PresentBit::Present, DescriptorPrivilegeLevel::Ring0, GateType::Int32),
    );
    core::ptr::write((idt_base + 8 * idte_size) as *mut Idte, df_entry);

    // Vector 12: #SS (Stack-Segment Fault) — has error code
    let ss_entry = Idte::new(
        _early_ss_entry as *const () as u32,
        KERNEL_CS,
        Flags::new(PresentBit::Present, DescriptorPrivilegeLevel::Ring0, GateType::Int32),
    );
    core::ptr::write((idt_base + 12 * idte_size) as *mut Idte, ss_entry);

    // Vector 13: #GP (General Protection Fault) — has error code
    let gp_entry = Idte::new(
        _early_gp_entry as *const () as u32,
        KERNEL_CS,
        Flags::new(PresentBit::Present, DescriptorPrivilegeLevel::Ring0, GateType::Int32),
    );
    core::ptr::write((idt_base + 13 * idte_size) as *mut Idte, gp_entry);

    // Build a stack-local IDTR and load it. The CPU caches the 6-byte descriptor in the
    // IDTR register, so the stack copy is safe to drop after lidt returns.
    let idt_size: u16 = (EARLY_IDT_LEN * core::mem::size_of::<Idte>()) as u16;
    let mut idtr = Idtr { size: 0, ptr: 0 };
    idtr.init(idt_base as u32, idt_size);
    idtr.load();

    // Cache the boot-time first-free-GPA from the scratch metadata bump allocator slot.
    // This must be done before any CoW fault can advance the bump pointer.
    // We store it in the early-GDT scratch page at a fixed offset.
    let boot_first_free_gpa: usize = super::first_free_scratch_gpa();
    super::save_boot_first_free_gpa(boot_first_free_gpa);

    // Advance the bump allocator pointer past the scratch-reserved region so that
    // CoW frame allocations (which use the bump allocator before the full frame
    // allocator is initialized) never overlap our scratch-resident data structures.
    //
    // The scratch-reserved region starts at boot_first_free_gpa (the host places it
    // right after the page tables), and has size SCRATCH_RESERVED_SIZE.
    let reserved_end_gpa: usize = boot_first_free_gpa + super::SCRATCH_RESERVED_SIZE;
    let alloc_ptr = ::hyperlight_guest::layout::allocator_gva();
    core::ptr::write_volatile(alloc_ptr, reserved_end_gpa as u64);
}
