/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SETJMP_H
#define _NANVIX_SETJMP_H

/**
 * @file setjmp.h
 * @brief Non-local jumps.
 *
 * Declares setjmp()/longjmp() and the POSIX sigsetjmp()/siglongjmp() functions
 * for non-local control flow. jmp_buf and sigjmp_buf store the callee-saved
 * registers for the target ABI (x86-32: EBX, ESI, EDI, EBP, ESP, EIP; x86-64:
 * RBX, RBP, R12-R15, RSP, RIP; AArch64: X19-X30, SP, D8-D15); sigjmp_buf
 * additionally records whether the signal mask was saved and the saved mask.
 * Implemented by the libc_setjmp Rust crate using global_asm.
 */

#include <signal.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Buffer type for saving and restoring execution context. */
typedef struct {
#if defined(__x86_64__)
    long regs[8]; /**< Saved registers: RBX, RBP, R12, R13, R14, R15, RSP, RIP. */
#elif defined(__aarch64__)
    long regs[21]; /**< Saved registers: X19-X30, SP, D8-D15. */
#else
    int regs[6]; /**< Saved registers: EBX, ESI, EDI, EBP, ESP, EIP. */
#endif
} jmp_buf[1];

/** @brief Buffer type for sigsetjmp()/siglongjmp() execution context. */
typedef struct {
#if defined(__x86_64__)
    long regs[8]; /**< Saved registers: RBX, RBP, R12, R13, R14, R15, RSP, RIP. */
#elif defined(__aarch64__)
    long regs[21]; /**< Saved registers: X19-X30, SP, D8-D15. */
#else
    int regs[6]; /**< Saved registers: EBX, ESI, EDI, EBP, ESP, EIP. */
#endif
    int savemask; /**< Nonzero if sigsetjmp() was asked to save the mask. */
    sigset_t sigmask; /**< Signal mask saved when savemask is nonzero. */
} sigjmp_buf[1];

/*==================================================================================================
 * Functions
 *==================================================================================================*/

extern int setjmp(jmp_buf env) __attribute__((__returns_twice__));
extern void longjmp(jmp_buf env, int val) __attribute__((__noreturn__));

/*==================================================================================================
 * Signal Jumps (POSIX)
 *==================================================================================================*/

extern int sigsetjmp(sigjmp_buf env, int savemask) __attribute__((__returns_twice__));
extern void siglongjmp(sigjmp_buf env, int val) __attribute__((__noreturn__));

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SETJMP_H */
