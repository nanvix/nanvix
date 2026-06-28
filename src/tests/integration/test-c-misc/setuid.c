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

// Tests whether `setuid()` can be used to set the real user ID of the calling process.
void test_setuid(void)
{
    fprintf(stderr, "testing setuid() ... ");

    uid_t uid = getuid();
    assert(setuid(uid) == 0);

    fprintf(stderr, "passed\n");
}
