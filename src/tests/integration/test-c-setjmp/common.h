/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _COMMON_H_
#define _COMMON_H_

// Tests that setjmp() returns zero when invoked directly (not through longjmp()).
extern void test_setjmp_direct_return(void);

// Tests that longjmp() delivers its value to setjmp(), including the 0 -> 1 rule.
extern void test_longjmp_return_value(void);

// Tests that volatile automatic variables modified after setjmp() survive a longjmp().
extern void test_volatile_locals_preserved(void);

// Tests that setjmp() writes only within the jmp_buf and does not overflow into adjacent storage.
extern void test_jmp_buf_no_overflow(void);

// Tests that longjmp() unwinds intermediate call frames and resumes at the setjmp() frame, which
// then returns normally to its own caller.
extern void test_longjmp_across_calls(void);

// Tests repeated longjmp() to a single re-armed setjmp() target (retry-loop pattern).
extern void test_longjmp_retry_loop(void);

// Tests sigsetjmp()/siglongjmp() non-local control flow.
extern void test_sigsetjmp_siglongjmp(void);

// Tests that siglongjmp() restores the mask saved by sigsetjmp(env, 1).
extern void test_sigsetjmp_restores_signal_mask(void);

// Tests that sigsetjmp(env, 0) leaves the current signal mask unchanged on siglongjmp().
extern void test_sigsetjmp_without_savemask(void);

#endif /* _COMMON_H_ */
