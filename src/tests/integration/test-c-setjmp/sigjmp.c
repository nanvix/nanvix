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

// Tests sigsetjmp()/siglongjmp() non-local control flow.
void test_sigsetjmp_siglongjmp(void)
{
    fprintf(stderr, "testing sigsetjmp()/siglongjmp() ... ");

    sigjmp_buf env;
    volatile int reached = 0;

    switch (sigsetjmp(env, 1)) {
        case 0:
            reached = 1;
            siglongjmp(env, 5);
            assert(0 && "siglongjmp() returned");
        case 5:
            assert(reached == 1);
            break;
        default:
            assert(0 && "siglongjmp() delivered the wrong value");
    }

    fprintf(stderr, "passed\n");
}
