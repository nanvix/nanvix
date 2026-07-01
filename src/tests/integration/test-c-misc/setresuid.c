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

// Tests whether `setresuid()` can set the real, effective, and saved-set user IDs.
void test_setresuid(void)
{
    fprintf(stderr, "testing setresuid() ... ");

    uid_t uid = getuid();

    // Setting every identifier to the current real user ID is a no-op and must succeed.
    assert(setresuid(uid, uid, uid) == 0);

    // A value of (uid_t)-1 leaves the corresponding identifier unchanged, so this succeeds too.
    assert(setresuid((uid_t)-1, (uid_t)-1, (uid_t)-1) == 0);

    // Nanvix is a single-user system, so switching to another user is not permitted.
    uid_t other = (uid == 0) ? 1 : 0;
    errno = 0;
    assert(setresuid(other, (uid_t)-1, (uid_t)-1) == -1);
    assert(errno == EPERM);

    fprintf(stderr, "passed\n");
}
