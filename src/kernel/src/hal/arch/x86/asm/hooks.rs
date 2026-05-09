// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// x86 Exception, Interrupt, Kernel-Call Hooks and Low-Level Routines
//==================================================================================================

use crate::hal::arch::x86::{
    cpu::ExceptionInformation,
    ContextInformation,
};
use ::arch::{
    cpu::tss::Tss,
    mem::WORD_SIZE,
};
use ::core::arch::global_asm;

//==================================================================================================
// Constants
//==================================================================================================

/// Offset to exception structure.
const EXCEPTION_SKIP: i32 = -(ContextInformation::CONTEXT_SW_SIZE as i32)
    - ExceptionInformation::EXCEPTION_SIZE as i32
    + ExceptionInformation::EXCEPTION_ERR as i32;

/// Whether the microvm feature is enabled (1) or not (0). Used by the assembly `.if` directive
/// to conditionally compile the stack-overflow guard macro body.
const MICROVM: u32 = if cfg!(feature = "microvm") { 1 } else { 0 };

/// VMM ACPI power-management I/O port (microvm only; dummy value when disabled).
#[cfg(feature = "microvm")]
const VMM_PORT: u32 = ::config::microvm::DEFAULT_VMM_PORT as u32;
#[cfg(not(feature = "microvm"))]
const VMM_PORT: u32 = 0;

/// Compound shutdown value: (SHUTDOWN_CMD << 16) | exit_status (microvm only; dummy when disabled).
#[cfg(feature = "microvm")]
const SHUTDOWN_VALUE: u32 = ((::config::microvm::DEFAULT_VMM_SHUTDOWN_CMD as u32) << 16)
    | ::sys::ExitStatus::STACK_OVERFLOW_EXCEPTION.as_u32();
#[cfg(not(feature = "microvm"))]
const SHUTDOWN_VALUE: u32 = 0;

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
    // EXCP_STACK_GUARD is only needed when the microvm guard-check code is compiled in; when
    // `{MICROVM}=0`, this `.extern` is compiled out together with the guard-check block.
    ".if {MICROVM}",
    ".extern EXCP_STACK_GUARD",
    ".endif",

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
    // Saves the content of general purpose and segment registers in the
    // stack of the caller. A pointer to the saved execution context is
    // saved into ret.
    //
    // Note that EIP, CS, EFLAGS, ESP, and SS are not saved by this
    // macro because they are automatically saved by the hardware upon
    // an exception/interrupt.
    //
    ".macro context_save ret",
    "    subl ${CONTEXT_SW_SIZE}, %esp",
    "    movl %eax, {CONTEXT_EAX}(%esp)",
    "    movl %ebx, {CONTEXT_EBX}(%esp)",
    "    movl %ecx, {CONTEXT_ECX}(%esp)",
    "    movl %edx, {CONTEXT_EDX}(%esp)",
    "    movl %ebp, {CONTEXT_EBP}(%esp)",
    "    movl %esi, {CONTEXT_ESI}(%esp)",
    "    movl %edi, {CONTEXT_EDI}(%esp)",
    "    movw %ds, {CONTEXT_DS}(%esp)",
    "    movw %es, {CONTEXT_ES}(%esp)",
    "    movw %fs, {CONTEXT_FS}(%esp)",
    "    movw %gs, {CONTEXT_GS}(%esp)",
    //   Clear the direction flag. User code may set DF=1 (via std),
    //   and all compiler-generated code assumes DF=0.
    "    cld",
    r"    movl %esp, \ret",
    ".endm",

    // -----------------------------------------------------------------
    // context_restore
    // -----------------------------------------------------------------
    //
    // Restores the content of general purpose and segment registers
    // from the stack of the caller.
    //
    ".macro context_restore",
    "    movl {CONTEXT_EAX}(%esp), %eax",
    "    movl {CONTEXT_EBX}(%esp), %ebx",
    "    movl {CONTEXT_ECX}(%esp), %ecx",
    "    movl {CONTEXT_EDX}(%esp), %edx",
    "    movl {CONTEXT_EBP}(%esp), %ebp",
    "    movl {CONTEXT_ESI}(%esp), %esi",
    "    movl {CONTEXT_EDI}(%esp), %edi",
    "    movw {CONTEXT_DS}(%esp), %ds",
    "    movw {CONTEXT_ES}(%esp), %es",
    "    movw {CONTEXT_FS}(%esp), %fs",
    "    movw {CONTEXT_GS}(%esp), %gs",
    "    addl ${CONTEXT_SW_SIZE}, %esp",
    ".endm",

    // -----------------------------------------------------------------
    // excp_stack_guard_check
    // -----------------------------------------------------------------
    //
    // Dynamic stack overflow guard. Compares ESP against the global
    // variable EXCP_STACK_GUARD. A value of 0 disables the check.
    //
    // On microvm, a stack overflow triggers a clean VMM shutdown via
    // the ACPI power-management port.
    //
    // Clobbers: EDX, EAX (only on the overflow path which never returns).
    //
    ".macro excp_stack_guard_check",
    ".if {MICROVM}",
    "    cmpl $0, EXCP_STACK_GUARD",
    "    je 1f",
    "    cmpl EXCP_STACK_GUARD, %esp",
    "    jae 1f",
    // Stack overflow — trigger a clean VMM shutdown.
    "    movl ${SHUTDOWN_VALUE}, %eax",
    "    movw ${VMM_PORT}, %dx",
    "    outl %eax, (%dx)",
    // Fallback halt loop.
    "2:  hlt",
    "    jmp 2b",
    "1:",
    ".endif",
    ".endm",

    // -----------------------------------------------------------------
    // _do_excp_noerr_code — exception without error code
    // -----------------------------------------------------------------
    r".macro _do_excp_noerr_code number",
    r"_do_excp\number:",
    "    excp_stack_guard_check",
    "    push $0",
    "    xchg %eax, (%esp)",
    "    xchg %eax, {EXCEPTION_SKIP}(%esp)",
    "    xchg %eax, (%esp)",
    "    context_save %eax",
    r"    movl $\number, %ebx",
    "    movl $0, %ecx",
    "    jmp _do_excp",
    ".endm",

    // -----------------------------------------------------------------
    // _do_excp_err_code — exception with error code
    // -----------------------------------------------------------------
    r".macro _do_excp_err_code number",
    r"_do_excp\number:",
    "    excp_stack_guard_check",
    "    xchg %eax, (%esp)",
    "    xchg %eax, {EXCEPTION_SKIP}(%esp)",
    "    xchg %eax, (%esp)",
    "    context_save %eax",
    r"    movl $\number, %ebx",
    "    movl $0, %ecx",
    "    jmp _do_excp",
    ".endm",

    // -----------------------------------------------------------------
    // _do_excp_err2_code — exception with error code and CR2
    // -----------------------------------------------------------------
    r".macro _do_excp_err2_code number",
    r"_do_excp\number:",
    "    excp_stack_guard_check",
    "    xchg %eax, (%esp)",
    "    xchg %eax, {EXCEPTION_SKIP}(%esp)",
    "    xchg %eax, (%esp)",
    "    context_save %eax",
    r"    movl $\number, %ebx",
    "    movl %cr2, %ecx",
    "    jmp _do_excp",
    ".endm",

    // -----------------------------------------------------------------
    // _do_hwint_macro — low-level hardware interrupt dispatcher
    // -----------------------------------------------------------------
    r".macro _do_hwint_macro num",
    r"_do_hwint\num:",
    "    context_save %eax",
    r"    pushl $\num",
    "    call do_interrupt",
    "    addl ${WORD_SIZE}, %esp",
    "    context_restore",
    "    iret",
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

    "_do_excp:",
    // Save exception information.
    "    movl {CONTEXT_EIP}(%eax), %edx",
    "    subl ${EXCEPTION_SIZE}, %esp",
    "    movl %ebx, {EXCEPTION_NR}(%esp)",
    "    movl %ecx, {EXCEPTION_DATA}(%esp)",
    "    movl %edx, {EXCEPTION_CODE}(%esp)",
    "    movl {EXCEPTION_ERR}(%esp), %ebx",
    "    movl %ebx, {CONTEXT_ERR}(%eax)",
    "    movl %esp, %ebx",

    // Call high-level exception dispatcher.
    "    pushl %eax", // Execution context.
    "    pushl %ebx", // Exception context.
    "    call do_exception",
    "    addl $(2*{WORD_SIZE}), %esp",
    "    addl ${EXCEPTION_SIZE}, %esp",

    "    context_restore",

    // Pop error code.
    "    addl ${WORD_SIZE}, %esp",

    "    jmp __leave_kernel",

    // =================================================================
    // Kernel Call Hook
    // =================================================================
    //
    // Registered as an interrupt gate in the IDT, so we do not
    // clear/set the IF flag — the hardware handles it.
    //
    "_do_kcall:",
    // Save execution context (esp and eflags saved by hardware;
    // we do not save scratch registers eax, ecx, edx).
    "    pushl %ebp",
    "    pushl %esi",
    "    pushl %edi",
    "    pushl %ebx",

    // Clear the direction flag.
    "    cld",

    "    mov %esp, %ebp",

    // Push kernel call parameters.
    "    pushl %edi",
    "    pushl %edx",
    "    pushl %ecx",
    "    pushl %ebx",
    "    pushl %eax",

    // Handle kernel call.
    "    call do_kcall",

    // Wipe out kernel call parameters.
    "    addl $5*{WORD_SIZE}, %esp",

    // Restore execution context.
    "    popl %ebx",
    "    popl %edi",
    "    popl %esi",
    "    popl %ebp",

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
    "__context_switch:",
    "    movl 4(%esp), %eax",   // from
    "    movl 8(%esp), %edx",   // to
    "    movl 12(%esp), %ecx",  // tss

    // Save execution context (callee-saved only).
    "    movl %ebx, {CONTEXT_EBX}(%eax)",
    "    movl %esi, {CONTEXT_ESI}(%eax)",
    "    movl %edi, {CONTEXT_EDI}(%eax)",
    "    movl %ebp, {CONTEXT_EBP}(%eax)",
    "    movl %esp, {CONTEXT_ESP}(%eax)",
    "    movw %gs,  {CONTEXT_GS}(%eax)",
    "    movw %fs,  {CONTEXT_FS}(%eax)",
    "    pushf",
    "    pop {CONTEXT_EFLAGS}(%eax)",

    // Save address space.
    "    movl %cr3, %ebx",
    "    movl %ebx, {CONTEXT_CR3}(%eax)",

    // Restore execution context.
    "    movl {CONTEXT_EBX}(%edx), %ebx",
    "    movl {CONTEXT_ESI}(%edx), %esi",
    "    movl {CONTEXT_EDI}(%edx), %edi",
    "    movl {CONTEXT_EBP}(%edx), %ebp",
    "    movl {CONTEXT_ESP}(%edx), %esp",
    "    movw {CONTEXT_GS}(%edx), %gs",
    "    movw {CONTEXT_FS}(%edx), %fs",
    "    push {CONTEXT_EFLAGS}(%edx)",
    "    popfl",

    // Restore address space.
    "    movl {CONTEXT_CR3}(%edx), %eax",
    "    movl %eax, %cr3",

    // Update ESP0 on TSS.
    "    movl {CONTEXT_ESP0}(%edx), %eax",
    "    movl %eax, {TSS_ESP0}(%ecx)",

    "    ret",

    // =================================================================
    // __leave_kernel() / __leave_kernel_to_user_mode()
    // =================================================================

    "__leave_kernel_to_user_mode:",

    "    popl %ecx",  // user function argument
    "    popl %edx",  // user function

    // Set segment registers for the user-space thread data area.
    "    popl %eax",  // eax gets user_tda
    "    movw %ax, %fs",
    "    movw %ax, %gs",

    // Restore data segment registers.
    "    movl 16(%esp), %eax", // eax gets user_ds
    "    movw %ax, %ds",
    "    movw %ax, %es",

    // Clear volatile kernel registers to avoid leaking kernel state.
    "    xorl %eax, %eax",
    "    xorl %ebx, %ebx",
    "    xorl %esi, %esi",
    "    xorl %edi, %edi",
    "    xorl %ebp, %ebp",

    "__leave_kernel:",
    "    iret",

    // =================================================================
    // Const Operands
    // =================================================================
    WORD_SIZE = const WORD_SIZE,
    CONTEXT_SW_SIZE = const ContextInformation::CONTEXT_SW_SIZE,
    CONTEXT_ESP0 = const ContextInformation::CONTEXT_ESP0,
    CONTEXT_CR3 = const ContextInformation::CONTEXT_CR3,
    CONTEXT_GS = const ContextInformation::CONTEXT_GS,
    CONTEXT_FS = const ContextInformation::CONTEXT_FS,
    CONTEXT_ES = const ContextInformation::CONTEXT_ES,
    CONTEXT_DS = const ContextInformation::CONTEXT_DS,
    CONTEXT_EDI = const ContextInformation::CONTEXT_EDI,
    CONTEXT_ESI = const ContextInformation::CONTEXT_ESI,
    CONTEXT_EBP = const ContextInformation::CONTEXT_EBP,
    CONTEXT_EDX = const ContextInformation::CONTEXT_EDX,
    CONTEXT_ECX = const ContextInformation::CONTEXT_ECX,
    CONTEXT_EBX = const ContextInformation::CONTEXT_EBX,
    CONTEXT_EAX = const ContextInformation::CONTEXT_EAX,
    CONTEXT_ERR = const ContextInformation::CONTEXT_ERR,
    CONTEXT_EIP = const ContextInformation::CONTEXT_EIP,
    CONTEXT_EFLAGS = const ContextInformation::CONTEXT_EFLAGS,
    CONTEXT_ESP = const ContextInformation::CONTEXT_ESP,
    EXCEPTION_SIZE = const ExceptionInformation::EXCEPTION_SIZE,
    EXCEPTION_NR = const ExceptionInformation::EXCEPTION_NR,
    EXCEPTION_ERR = const ExceptionInformation::EXCEPTION_ERR,
    EXCEPTION_DATA = const ExceptionInformation::EXCEPTION_DATA,
    EXCEPTION_CODE = const ExceptionInformation::EXCEPTION_CODE,
    EXCEPTION_SKIP = const EXCEPTION_SKIP,
    TSS_ESP0 = const Tss::TSS_ESP0,
    MICROVM = const MICROVM,
    SHUTDOWN_VALUE = const SHUTDOWN_VALUE,
    VMM_PORT = const VMM_PORT,
    options(att_syntax),
);
