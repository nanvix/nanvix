/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <setjmp.h>
#include <stdio.h>

//==================================================================================================
// Private Variables
//==================================================================================================

// Shared jump target for the cross-frame unwinding test. It lives at file scope so
// the deeply nested callee can reach the buffer saved by run_protected().
static jmp_buf cross_call_env;

//==================================================================================================
// Private Functions
//==================================================================================================

// Innermost frame: jumps back to the saved setjmp() site, unwinding itself and its caller.
static void deep_callee(int code)
{
    longjmp(cross_call_env, code);
    assert(0 && "longjmp() returned");
}

// Intermediate frame: holds a live local across the call so the unwinding crosses a real,
// non-trivial stack frame, then calls the innermost frame.
static void middle_callee(int code)
{
    volatile int guard = 0x5A5A;
    deep_callee(code);
    // Unreachable: deep_callee() never returns.
    assert(guard == 0x5A5A);
}

// Saves a jump target, dives two frames deep, and is re-entered by longjmp(). Returns the value
// delivered by longjmp(), proving the frame resumed cleanly and can itself return normally.
static int run_protected(void)
{
    volatile int armed = 0;

    switch (setjmp(cross_call_env)) {
        case 0:
            armed = 1;
            middle_callee(7);
            // Unreachable: middle_callee() always longjmp()s back here.
            assert(0 && "middle_callee() returned");
            return (-1);
        case 7:
            // Re-entered via longjmp() from deep_callee(), two frames down.
            assert(armed == 1);
            return (7);
        default:
            assert(0 && "longjmp() delivered the wrong value");
            return (-1);
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests that longjmp() unwinds intermediate call frames and resumes at the setjmp() frame, which
// then returns normally to its own caller.
void test_longjmp_across_calls(void)
{
    fprintf(stderr, "testing longjmp() across call frames ... ");

    // run_protected() only returns normally if longjmp() unwound the two nested
    // frames, resumed at the setjmp() site, and left that frame intact enough to
    // return here.
    int result = run_protected();
    assert(result == 7);

    fprintf(stderr, "passed\n");
}

// Tests repeated longjmp() to a single re-armed setjmp() target (retry-loop pattern).
void test_longjmp_retry_loop(void)
{
    fprintf(stderr, "testing repeated longjmp() ... ");

    jmp_buf env;
    volatile int attempts = 0;

    // setjmp() saves the context once; each longjmp() resumes here without
    // re-running setjmp(). The volatile counter survives every jump, so the loop
    // terminates after three passes.
    (void)setjmp(env);

    attempts += 1;
    if (attempts < 3) {
        longjmp(env, attempts);
    }

    assert(attempts == 3);

    fprintf(stderr, "passed\n");
}
