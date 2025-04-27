/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _POSIX_C_SOURCE 200809 // AT_FDCWD

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can remove a file.
void test_unlinkat(void)
{
    fprintf(stderr, "testing unlinkat() ... ");

    const char *filename = "test.txt";
    assert(strlen(filename) <= NAME_MAX);

    // Create a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    assert(close(fd) == 0);

    // Remove test file.
    assert(unlinkat(AT_FDCWD, filename, 0) == 0);

    fprintf(stderr, "passed\n");
}
