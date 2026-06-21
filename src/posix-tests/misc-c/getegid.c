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

// Tests whether we can get the effective group ID of the calling process with `getegid()`.
void test_getegid(void)
{
    fprintf(stderr, "testing getegid() ... ");

    assert(getegid() != (gid_t)-1);

    fprintf(stderr, "passed\n");
}
