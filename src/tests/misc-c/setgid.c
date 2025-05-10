/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether `setgid()` can be used to set the real group ID of the calling process.
void test_setgid(void)
{
    fprintf(stderr, "testing setgid() ... ");

    gid_t gid = getgid();
    assert(setgid(gid) == 0);

    fprintf(stderr, "passed\n");
}
