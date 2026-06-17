// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

core::arch::global_asm!(
    r#"
    .extern _start

    .globl _do_start

    .section .crt0, "ax"

    _do_start:
        #
        # Entry point for newly created processes (x86_64).
        #
        # The kernel passes the argument pointer in RDI and the environment pointer
        # in RSI (SysV ABI first and second arguments).
        #
        # This stub aligns the stack to 16 bytes and calls _start(argp, envp).
        #
        and rsp, -16
        mov rbp, rsp
        call _start
    # Safety net: _start() calls exit() and never returns.
    # If it somehow does, spin forever rather than falling through.
    1:  jmp 1b
    "#
);
