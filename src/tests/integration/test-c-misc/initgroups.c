/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <grp.h>
#include <stdio.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether `initgroups()` initializes the supplementary group access list.
void test_initgroups(void)
{
    fprintf(stderr, "testing initgroups() ... ");

    // Nanvix has no supplementary group memberships, so the access list is already correct and
    // initializing it succeeds.
    assert(initgroups("root", getgid()) == 0);

    fprintf(stderr, "passed\n");
}
