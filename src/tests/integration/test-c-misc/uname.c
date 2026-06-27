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
#include <sys/utsname.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can get system information with `uname()`.
void test_uname(void)
{
    fprintf(stderr, "testing uname() ... ");

    // Sanity check size of `utsname` structure.
    STATIC_ASSERT_SIZE(struct utsname,
                       _UTSNAME_LENGTH * sizeof(char) +     // sysname
                           _UTSNAME_LENGTH * sizeof(char) + // nodename
                           _UTSNAME_LENGTH * sizeof(char) + // release
                           _UTSNAME_LENGTH * sizeof(char) + // version
                           _UTSNAME_LENGTH * sizeof(char)   // machine
    );

    // Get system information.
    struct utsname utsname = {0};
    assert(uname(&utsname) == 0);

    // Check if the system information structure is not empty.
    assert(utsname.sysname[0] != '\0');
    assert(utsname.nodename[0] != '\0');
    assert(utsname.release[0] != '\0');
    assert(utsname.version[0] != '\0');
    assert(utsname.machine[0] != '\0');

    fprintf(stderr, "passed\n");
}
