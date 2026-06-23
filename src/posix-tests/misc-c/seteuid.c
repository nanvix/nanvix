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

// Tests whether `seteuid()` can be used to set the effective user ID of the calling process.
void test_seteuid(void)
{
    fprintf(stderr, "testing seteuid() ... ");

    uid_t uid = getuid();
    assert(seteuid(uid) == 0);

    fprintf(stderr, "passed\n");
}
