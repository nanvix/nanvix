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
// Standalone Functions
//==================================================================================================

// Tests that setjmp() returns zero when invoked directly (not through longjmp()).
void test_setjmp_direct_return(void)
{
    fprintf(stderr, "testing setjmp() direct return ... ");

    jmp_buf env;

    // A direct setjmp() call evaluates to zero. Nothing arms a longjmp() on this
    // buffer, so the body below must never run.
    if (setjmp(env) != 0) {
        assert(0 && "setjmp() reported a longjmp() return without one");
    }

    fprintf(stderr, "passed\n");
}

// Tests that longjmp() delivers its value to setjmp(), including the 0 -> 1 rule.
void test_longjmp_return_value(void)
{
    fprintf(stderr, "testing longjmp() return value ... ");

    // A non-zero value passed to longjmp() is delivered verbatim to setjmp().
    {
        jmp_buf env;
        switch (setjmp(env)) {
            case 0:
                longjmp(env, 42);
                assert(0 && "longjmp() returned");
            case 42:
                break;
            default:
                assert(0 && "longjmp() delivered the wrong value");
        }
    }

    // POSIX requires that longjmp(env, 0) make setjmp() return 1, never 0.
    {
        jmp_buf env;
        switch (setjmp(env)) {
            case 0:
                longjmp(env, 0);
                assert(0 && "longjmp() returned");
            case 1:
                break;
            default:
                assert(0 && "longjmp(env, 0) must make setjmp() return 1");
        }
    }

    fprintf(stderr, "passed\n");
}
