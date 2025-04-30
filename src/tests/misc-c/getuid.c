/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <stdio.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can the user ID of the calling process with `getuid()`.
void test_getuid(void)
{
    fprintf(stderr, "testing getuid() ... ");

    assert(getuid() != (uid_t)-1);

    fprintf(stderr, "passed\n");
}
