/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

#define _POSIX_C_SOURCE 200112L // gethostname().

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

// TODO: Remove this constant when it exported by Newlib.
#define HOSTNAME_MAX 255

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether `gethostname()` works.
void test_gethostname(void)
{
    fprintf(stderr, "testing gethostname() ... ");

    char hostname[HOSTNAME_MAX + 1];
    const char *HOSTNAME = __NANVIX_NODENAME__;

    // Get host name and assert result.
    assert(gethostname(hostname, sizeof(hostname)) != -1);
    assert(strncmp(hostname, HOSTNAME, sizeof(hostname)) == 0);

    fprintf(stderr, "passed\n");
}
