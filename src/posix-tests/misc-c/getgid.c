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

// Tests whether we can get the group ID of the calling process with `getgid()`.
void test_getgid(void)
{
    fprintf(stderr, "testing getgid() ... ");

    assert(getgid() != (gid_t)-1);

    fprintf(stderr, "passed\n");
}
