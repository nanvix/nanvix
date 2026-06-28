/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

#define _POSIX_C_SOURCE 200112

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

// Tests whether `setegid()` can be used to set the effective group ID of the calling process.
void test_setegid(void)
{
    fprintf(stderr, "testing setegid() ... ");

    gid_t gid = getgid();
    assert(setegid(gid) == 0);

    fprintf(stderr, "passed\n");
}
