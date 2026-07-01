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
#include <stddef.h>
#include <stdio.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether `setgroups()` can set the supplementary group IDs of the calling process.
void test_setgroups(void)
{
    fprintf(stderr, "testing setgroups() ... ");

    // Clearing the (already empty) supplementary group list always succeeds.
    assert(setgroups(0, NULL) == 0);

    // A non-empty list must be a valid pointer.
    errno = 0;
    assert(setgroups(1, NULL) == -1);
    assert(errno == EFAULT);

    // Setting the list to the current real group ID is honored on a single-user system.
    gid_t gid = getgid();
    assert(setgroups(1, &gid) == 0);

    // Nanvix has no supplementary group memberships, so any other group is not permitted.
    gid_t other = (gid == 0) ? 1 : 0;
    errno = 0;
    assert(setgroups(1, &other) == -1);
    assert(errno == EPERM);

    fprintf(stderr, "passed\n");
}
