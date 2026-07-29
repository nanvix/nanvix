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
 * RBX, RBP, R12-R15, RSP, RIP); sigjmp_buf additionally records a savemask flag.
 * The guest does not yet maintain a signal mask, so the saved mask is currently
 * empty. Implemented by the libc_setjmp Rust crate using global_asm.
 */

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
#else
    int regs[6]; /**< Saved registers: EBX, ESI, EDI, EBP, ESP, EIP. */
#endif
} jmp_buf[1];

/** @brief Buffer type for sigsetjmp()/siglongjmp() execution context. */
typedef struct {
#if defined(__x86_64__)
    long regs[8]; /**< Saved registers: RBX, RBP, R12, R13, R14, R15, RSP, RIP. */
#else
    int regs[6]; /**< Saved registers: EBX, ESI, EDI, EBP, ESP, EIP. */
#endif
    int savemask; /**< Nonzero if sigsetjmp() was asked to save the mask. */
} sigjmp_buf[1];

/*==================================================================================================
 * Functions
 *==================================================================================================*/

extern int setjmp(jmp_buf env);
extern void longjmp(jmp_buf env, int val) __attribute__((__noreturn__));

/*==================================================================================================
 * Signal Jumps (POSIX)
 *==================================================================================================*/

extern int sigsetjmp(sigjmp_buf env, int savemask);
extern void siglongjmp(sigjmp_buf env, int val) __attribute__((__noreturn__));

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SETJMP_H */
