// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

::core::arch::global_asm!(
    r#"
    .global _do_start_thread
    .extern _do_exit_thread
    .type _do_start_thread, @function

    _do_start_thread:
        #
        # Entry point for newly created threads (x86_64).
        #
        # The kernel passes the thread function pointer in RDX and its argument
        # in RCX via the trap frame.
        #

        # Save func and arg into callee-saved registers.
        mov r12, rdx        # R12 = func
        mov r13, rcx        # R13 = arg

        # Force 16-byte stack alignment.
        and rsp, -16

        # Call func(arg) using x86_64 SysV ABI (first arg in RDI).
        mov rdi, r13
        call r12

        # Call _do_exit_thread(status). Return value in EAX → first arg in EDI.
        and rsp, -16
        mov edi, eax
        call _do_exit_thread

    # Safety net.
    1: jmp 1b
    "#
);
