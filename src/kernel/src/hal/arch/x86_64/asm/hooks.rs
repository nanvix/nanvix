// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// x86_64 Exception, Interrupt, Kernel-Call Hooks and Low-Level Routines
//==================================================================================================

use crate::hal::arch::x86::{
    ContextInformation,
    ExceptionInformation,
};
use ::arch::cpu::tss::Tss;
use ::core::arch::global_asm;

//==================================================================================================
// Constants
//==================================================================================================

/// Size of a quad-word (8 bytes).
const QWORD_SIZE: u32 = core::mem::size_of::<u64>() as u32;

/// Offset to exception structure.
const EXCEPTION_SKIP: i32 = -(ContextInformation::CONTEXT_SW_SIZE as i32)
    - ExceptionInformation::EXCEPTION_SIZE as i32
    + ExceptionInformation::EXCEPTION_ERR as i32;

//==================================================================================================
// Assembly: Macros, Exception/Interrupt/Kcall Hooks, Context Switch, Leave Kernel, Physical Memory
//==================================================================================================

global_asm!(
    // =====================================================================
    // Text Section
    // =====================================================================
    r#".section .text,"ax",@progbits"#,

    // -----------------------------------------------------------------
    // Imported Symbols
    // -----------------------------------------------------------------
    ".extern do_exception",
    ".extern do_kcall",
    ".extern do_interrupt",

    // -----------------------------------------------------------------
    // Exported Symbols
    // -----------------------------------------------------------------

    // Exception hooks.
    ".globl _do_excp0",
    ".globl _do_excp1",
    ".globl _do_excp2",
    ".globl _do_excp3",
    ".globl _do_excp4",
    ".globl _do_excp5",
    ".globl _do_excp6",
    ".globl _do_excp7",
    ".globl _do_excp8",
    ".globl _do_excp9",
    ".globl _do_excp10",
    ".globl _do_excp11",
    ".globl _do_excp12",
    ".globl _do_excp13",
    ".globl _do_excp14",
    ".globl _do_excp15",
    ".globl _do_excp16",
    ".globl _do_excp17",
    ".globl _do_excp18",
    ".globl _do_excp19",
    ".globl _do_excp20",
    ".globl _do_excp30",

    // Kernel call hook.
    ".globl _do_kcall",

    // Hardware interrupt hooks.
    ".globl _do_hwint0",
    ".globl _do_hwint1",
    ".globl _do_hwint2",
    ".globl _do_hwint3",
    ".globl _do_hwint4",
    ".globl _do_hwint5",
    ".globl _do_hwint6",
    ".globl _do_hwint7",
    ".globl _do_hwint8",
    ".globl _do_hwint9",
    ".globl _do_hwint10",
    ".globl _do_hwint11",
    ".globl _do_hwint12",
    ".globl _do_hwint13",
    ".globl _do_hwint14",
    ".globl _do_hwint15",

    // Other.
    ".global __context_switch",
    ".global __leave_kernel",
    ".global __leave_kernel_to_user_mode",

    // =================================================================
    // Macros
    // =================================================================

    // -----------------------------------------------------------------
    // context_save
    // -----------------------------------------------------------------
    //
    // Saves the content of general purpose registers in the stack of the
    // caller. A pointer to the saved execution context is stored into
    // ret.
    //
    // Note that RIP, CS, RFLAGS, RSP, and SS are not saved by this
    // macro because they are automatically saved by the hardware upon
    // an exception/interrupt.
    //
    r".macro context_save ret",
    "    subq ${CONTEXT_SW_SIZE}, %rsp",
    "    movq %rax, {CONTEXT_RAX}(%rsp)",
    "    movq %rbx, {CONTEXT_RBX}(%rsp)",
    "    movq %rcx, {CONTEXT_RCX}(%rsp)",
    "    movq %rdx, {CONTEXT_RDX}(%rsp)",
    "    movq %rdi, {CONTEXT_RDI}(%rsp)",
    "    movq %rsi, {CONTEXT_RSI}(%rsp)",
    "    movq %rbp, {CONTEXT_RBP}(%rsp)",
    "    movq %r8,  {CONTEXT_R8}(%rsp)",
    "    movq %r9,  {CONTEXT_R9}(%rsp)",
    "    movq %r10, {CONTEXT_R10}(%rsp)",
    "    movq %r11, {CONTEXT_R11}(%rsp)",
    "    movq %r12, {CONTEXT_R12}(%rsp)",
    "    movq %r13, {CONTEXT_R13}(%rsp)",
    "    movq %r14, {CONTEXT_R14}(%rsp)",
    "    movq %r15, {CONTEXT_R15}(%rsp)",
    //   Clear the direction flag. User code may set DF=1 (via std),
    //   and all compiler-generated code (including Rust's rep-based
    //   memcpy) assumes DF=0. The hardware-saved RFLAGS in the
    //   exception frame preserves the original DF for restoration on
    //   iretq.
    "    cld",
    r"    movq %rsp, \ret",
    ".endm",

    // -----------------------------------------------------------------
    // context_restore
    // -----------------------------------------------------------------
    //
    // Restores the content of general purpose registers from the stack
    // of the caller.
    //
    ".macro context_restore",
    "    movq {CONTEXT_RAX}(%rsp), %rax",
    "    movq {CONTEXT_RBX}(%rsp), %rbx",
    "    movq {CONTEXT_RCX}(%rsp), %rcx",
    "    movq {CONTEXT_RDX}(%rsp), %rdx",
    "    movq {CONTEXT_RDI}(%rsp), %rdi",
    "    movq {CONTEXT_RSI}(%rsp), %rsi",
    "    movq {CONTEXT_RBP}(%rsp), %rbp",
    "    movq {CONTEXT_R8}(%rsp),  %r8",
    "    movq {CONTEXT_R9}(%rsp),  %r9",
    "    movq {CONTEXT_R10}(%rsp), %r10",
    "    movq {CONTEXT_R11}(%rsp), %r11",
    "    movq {CONTEXT_R12}(%rsp), %r12",
    "    movq {CONTEXT_R13}(%rsp), %r13",
    "    movq {CONTEXT_R14}(%rsp), %r14",
    "    movq {CONTEXT_R15}(%rsp), %r15",
    "    addq ${CONTEXT_SW_SIZE}, %rsp",
    ".endm",

    // -----------------------------------------------------------------
    // _do_excp_noerr_code — exception without error code
    // -----------------------------------------------------------------
    r".macro _do_excp_noerr_code number",
    r"_do_excp\number:",
    "    pushq $0",
    "    xchg %rax, (%rsp)",
    "    xchg %rax, {EXCEPTION_SKIP}(%rsp)",
    "    xchg %rax, (%rsp)",
    "    context_save %rdi",
    r"    movq $\number, %rsi",
    "    movq $0, %rdx",
    "    jmp _do_excp",
    ".endm",

    // -----------------------------------------------------------------
    // _do_excp_err_code — exception with error code
    // -----------------------------------------------------------------
    r".macro _do_excp_err_code number",
    r"_do_excp\number:",
    "    xchg %rax, (%rsp)",
    "    xchg %rax, {EXCEPTION_SKIP}(%rsp)",
    "    xchg %rax, (%rsp)",
    "    context_save %rdi",
    r"    movq $\number, %rsi",
    "    movq $0, %rdx",
    "    jmp _do_excp",
    ".endm",

    // -----------------------------------------------------------------
    // _do_excp_err2_code — exception with error code and CR2
    // -----------------------------------------------------------------
    r".macro _do_excp_err2_code number",
    r"_do_excp\number:",
    "    xchg %rax, (%rsp)",
    "    xchg %rax, {EXCEPTION_SKIP}(%rsp)",
    "    xchg %rax, (%rsp)",
    "    context_save %rdi",
    r"    movq $\number, %rsi",
    "    movq %cr2, %rdx",
    "    jmp _do_excp",
    ".endm",

    // -----------------------------------------------------------------
    // _do_hwint_macro — low-level hardware interrupt dispatcher
    // -----------------------------------------------------------------
    r".macro _do_hwint_macro num",
    r"_do_hwint\num:",
    "    context_save %rax",
    r"    movq $\num, %rdi",
    "    call do_interrupt",
    "    context_restore",
    "    iretq",
    ".endm",

    // =================================================================
    // Exception Hooks
    // =================================================================

    "_do_excp_noerr_code  0",  // Division-by-Zero Error
    "_do_excp_noerr_code  1",  // Debug Exception
    "_do_excp_noerr_code  2",  // Non-Maskable Interrupt
    "_do_excp_noerr_code  3",  // Breakpoint Exception
    "_do_excp_noerr_code  4",  // Overflow Exception
    "_do_excp_noerr_code  5",  // Bounds Check Exception
    "_do_excp_noerr_code  6",  // Invalid Opcode Exception
    "_do_excp_noerr_code  7",  // Coprocessor Not Available
    "_do_excp_err_code    8",  // Double Fault
    "_do_excp_noerr_code  9",  // Coprocessor Segment Overrun
    "_do_excp_err_code   10",  // Invalid TSS
    "_do_excp_err_code   11",  // Segment Not Present
    "_do_excp_err_code   12",  // Stack Segment Fault
    "_do_excp_err_code   13",  // General Protection Fault
    "_do_excp_err2_code  14",  // Page Fault
    "_do_excp_noerr_code 15",  // Reserved
    "_do_excp_noerr_code 16",  // Floating Point Exception
    "_do_excp_err_code   17",  // Alignment Check Exception
    "_do_excp_noerr_code 18",  // Machine Check Exception
    "_do_excp_noerr_code 19",  // SIMD Unit Exception
    "_do_excp_noerr_code 20",  // Virtual Exception
    "_do_excp_err_code   30",  // Security Exception

    // =================================================================
    // Low-Level Exception Handler Dispatcher
    // =================================================================
    //
    // On entry (x86_64 SysV ABI):
    //  - RDI: pointer to saved context
    //  - RSI: exception number
    //  - RDX: faulting address (CR2 for page faults, 0 otherwise)
    //
    "_do_excp:",
    // Save exception information.
    "    movq {CONTEXT_RIP}(%rdi), %rcx",
    "    subq ${EXCEPTION_SIZE}, %rsp",
    "    movq %rsi, {EXCEPTION_NR}(%rsp)",
    "    movq %rdx, {EXCEPTION_DATA}(%rsp)",
    "    movq %rcx, {EXCEPTION_CODE}(%rsp)",
    "    movq {EXCEPTION_ERR}(%rsp), %rcx",
    "    movq %rcx, {CONTEXT_ERR}(%rdi)",

    // Call high-level exception dispatcher.
    // RDI already has context pointer.
    "    movq %rsp, %rsi", // Exception info pointer.
    "    call do_exception",
    "    addq ${EXCEPTION_SIZE}, %rsp",

    "    context_restore",

    // Pop error code.
    "    addq ${QWORD_SIZE}, %rsp",

    "    jmp __leave_kernel",

    // =================================================================
    // Kernel Call Hook
    // =================================================================
    //
    // On entry (from int 0x81, the kcall trap vector):
    //  - RAX: kernel call number
    //  - RDI: arg0
    //  - RSI: arg1
    //  - RDX: arg2
    //  - R10: arg3
    //
    "_do_kcall:",
    //   Clear the direction flag. User code may set DF=1 (via std),
    //   and all compiler-generated code (including Rust's rep-based
    //   memcpy) assumes DF=0. The int instruction does not clear DF,
    //   so we must do it explicitly before calling into Rust kernel
    //   code.
    "    cld",

    // Save callee-saved registers.
    "    pushq %rbp",
    "    pushq %r12",
    "    pushq %r13",
    "    pushq %r14",
    "    pushq %r15",
    "    pushq %rbx",

    "    movq %rsp, %rbp",

    // Set up x86_64 SysV ABI arguments for
    // do_kcall(kcall_nr, arg0, arg1, arg2, arg3).
    "    movq %r10, %r8",  // arg3 → r8 (5th SysV param)
    "    movq %rdx, %rcx", // arg2 → rcx (4th SysV param)
    "    movq %rsi, %rdx", // arg1 → rdx (3rd SysV param)
    "    movq %rdi, %rsi", // arg0 → rsi (2nd SysV param)
    "    movq %rax, %rdi", // kcall_nr → rdi (1st SysV param)

    // Handle kernel call.
    "    call do_kcall",

    // Restore callee-saved registers. Return value is in RAX.
    "    popq %rbx",
    "    popq %r15",
    "    popq %r14",
    "    popq %r13",
    "    popq %r12",
    "    popq %rbp",

    "    jmp __leave_kernel",

    // =================================================================
    // Hardware Interrupt Hooks
    // =================================================================

    "_do_hwint_macro  0",
    "_do_hwint_macro  1",
    "_do_hwint_macro  2",
    "_do_hwint_macro  3",
    "_do_hwint_macro  4",
    "_do_hwint_macro  5",
    "_do_hwint_macro  6",
    "_do_hwint_macro  7",
    "_do_hwint_macro  8",
    "_do_hwint_macro  9",
    "_do_hwint_macro 10",
    "_do_hwint_macro 11",
    "_do_hwint_macro 12",
    "_do_hwint_macro 13",
    "_do_hwint_macro 14",
    "_do_hwint_macro 15",

    // =================================================================
    // __context_switch()
    // =================================================================
    //
    // Saves the execution context of the calling process and restores
    // the context of the target process.
    //
    // void __context_switch(
    //   ContextInformation *from,  // RDI
    //   ContextInformation *to,    // RSI
    //   Tss *tss                   // RDX
    // );
    //
    "__context_switch:",

    // Save execution context (callee-saved only).
    "    movq %rbx, {CONTEXT_RBX}(%rdi)",
    "    movq %r12, {CONTEXT_R12}(%rdi)",
    "    movq %r13, {CONTEXT_R13}(%rdi)",
    "    movq %r14, {CONTEXT_R14}(%rdi)",
    "    movq %r15, {CONTEXT_R15}(%rdi)",
    "    movq %rbp, {CONTEXT_RBP}(%rdi)",
    "    movq %rsp, {CONTEXT_RSP}(%rdi)",
    "    pushfq",
    "    popq {CONTEXT_RFLAGS}(%rdi)",

    // Save address space.
    "    movq %cr3, %rax",
    "    movq %rax, {CONTEXT_CR3}(%rdi)",

    // Restore execution context.
    "    movq {CONTEXT_RBX}(%rsi), %rbx",
    "    movq {CONTEXT_R12}(%rsi), %r12",
    "    movq {CONTEXT_R13}(%rsi), %r13",
    "    movq {CONTEXT_R14}(%rsi), %r14",
    "    movq {CONTEXT_R15}(%rsi), %r15",
    "    movq {CONTEXT_RBP}(%rsi), %rbp",
    "    movq {CONTEXT_RSP}(%rsi), %rsp",
    "    pushq {CONTEXT_RFLAGS}(%rsi)",
    "    popfq",

    // Restore address space.
    // Only reload CR3 if the target context has a non-zero CR3. A zero
    // CR3 indicates the initial boot context before per-process page
    // tables exist.
    "    movq {CONTEXT_CR3}(%rsi), %rax",
    "    testq %rax, %rax",
    "    jz .Lskip_cr3",
    "    movq %rax, %cr3",
    ".Lskip_cr3:",

    // Update RSP0 on TSS.
    "    movq {CONTEXT_RSP0}(%rsi), %rax",
    "    movl %eax, {TSS_RSP0}(%rdx)",
    "    shrq $32, %rax",
    "    movl %eax, ({TSS_RSP0} + 4)(%rdx)",

    "    ret",

    // =================================================================
    // __leave_kernel() / __leave_kernel_to_user_mode()
    // =================================================================

    "__leave_kernel_to_user_mode:",

    // Pop user function arguments pushed by forge_user_stack().
    // Stack order: arg1 (top), arg0 — map to SysV ABI registers.
    "    popq %rsi",  // arg1 (second argument: envp).
    "    popq %rdi",  // arg0 (first argument: argp).

    // Clear volatile registers to avoid leaking kernel state.
    // Preserve RDI and RSI which carry user function arguments.
    "    xorq %rax, %rax",
    "    xorq %rbx, %rbx",
    "    xorq %rcx, %rcx",
    "    xorq %rdx, %rdx",
    "    xorq %r8,  %r8",
    "    xorq %r9,  %r9",
    "    xorq %r10, %r10",
    "    xorq %r11, %r11",
    "    xorq %r12, %r12",
    "    xorq %r13, %r13",
    "    xorq %r14, %r14",
    "    xorq %r15, %r15",
    "    xorq %rbp, %rbp",

    // RSP now points at the iretq frame: RIP, CS, RFLAGS, RSP, SS.

    "__leave_kernel:",
    "    iretq",

    // =================================================================
    // Const Operands
    // =================================================================
    QWORD_SIZE = const QWORD_SIZE,
    CONTEXT_SW_SIZE = const ContextInformation::CONTEXT_SW_SIZE,
    CONTEXT_RSP0 = const ContextInformation::CONTEXT_RSP0,
    CONTEXT_CR3 = const ContextInformation::CONTEXT_CR3,
    CONTEXT_R15 = const ContextInformation::CONTEXT_R15,
    CONTEXT_R14 = const ContextInformation::CONTEXT_R14,
    CONTEXT_R13 = const ContextInformation::CONTEXT_R13,
    CONTEXT_R12 = const ContextInformation::CONTEXT_R12,
    CONTEXT_R11 = const ContextInformation::CONTEXT_R11,
    CONTEXT_R10 = const ContextInformation::CONTEXT_R10,
    CONTEXT_R9 = const ContextInformation::CONTEXT_R9,
    CONTEXT_R8 = const ContextInformation::CONTEXT_R8,
    CONTEXT_RBP = const ContextInformation::CONTEXT_RBP,
    CONTEXT_RSI = const ContextInformation::CONTEXT_RSI,
    CONTEXT_RDI = const ContextInformation::CONTEXT_RDI,
    CONTEXT_RDX = const ContextInformation::CONTEXT_RDX,
    CONTEXT_RCX = const ContextInformation::CONTEXT_RCX,
    CONTEXT_RBX = const ContextInformation::CONTEXT_RBX,
    CONTEXT_RAX = const ContextInformation::CONTEXT_RAX,
    CONTEXT_ERR = const ContextInformation::CONTEXT_ERR,
    CONTEXT_RIP = const ContextInformation::CONTEXT_RIP,
    CONTEXT_RFLAGS = const ContextInformation::CONTEXT_RFLAGS,
    CONTEXT_RSP = const ContextInformation::CONTEXT_RSP,
    EXCEPTION_SIZE = const ExceptionInformation::EXCEPTION_SIZE,
    EXCEPTION_NR = const ExceptionInformation::EXCEPTION_NR,
    EXCEPTION_ERR = const ExceptionInformation::EXCEPTION_ERR,
    EXCEPTION_DATA = const ExceptionInformation::EXCEPTION_DATA,
    EXCEPTION_CODE = const ExceptionInformation::EXCEPTION_CODE,
    EXCEPTION_SKIP = const EXCEPTION_SKIP,
    TSS_RSP0 = const Tss::TSS_RSP0,
    options(att_syntax),
);
