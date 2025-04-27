/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _POSIX_C_SOURCE 200809 // strnlen()

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

//==================================================================================================
// Imports
//==================================================================================================

// Tests whether we can get the current working directory.
void test_getcwd(void)
{
    fprintf(stderr, "testing getcwd() ... ");

    char pathbuf[PATH_MAX] = {0};

    // Get current working directory and assert result.
    char *cwd = getcwd(pathbuf, sizeof(pathbuf));
    assert(cwd != NULL);
    assert(cwd == pathbuf);
    size_t len = strnlen(cwd, PATH_MAX);
    assert((len > 0) && (len < PATH_MAX));

    fprintf(stderr, "passed\n");
}
