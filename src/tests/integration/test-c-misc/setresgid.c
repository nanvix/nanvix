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

// Tests whether `setresgid()` can set the real, effective, and saved-set group IDs.
void test_setresgid(void)
{
    fprintf(stderr, "testing setresgid() ... ");

    gid_t gid = getgid();

    // Setting every identifier to the current real group ID is a no-op and must succeed.
    assert(setresgid(gid, gid, gid) == 0);

    // A value of (gid_t)-1 leaves the corresponding identifier unchanged, so this succeeds too.
    assert(setresgid((gid_t)-1, (gid_t)-1, (gid_t)-1) == 0);

    // Nanvix is a single-user system, so switching to another group is not permitted.
    gid_t other = (gid == 0) ? 1 : 0;
    errno = 0;
    assert(setresgid(other, (gid_t)-1, (gid_t)-1) == -1);
    assert(errno == EPERM);

    fprintf(stderr, "passed\n");
}
