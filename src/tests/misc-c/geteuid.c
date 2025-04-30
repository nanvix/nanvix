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

// Tests whether we can get the effective user ID of the calling process with `geteuid()`.
void test_geteuid(void)
{
    fprintf(stderr, "testing geteuid() ... ");

    assert(geteuid() != (uid_t)-1);

    fprintf(stderr, "passed\n");
}
