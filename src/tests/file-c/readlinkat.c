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
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can read a symbolic link.
void test_readlinkat(void)
{
    fprintf(stderr, "testing readlinkat() ... ");

    const char *filename = "README.md";
    const char *linkname = "README.link";
    assert(strlen(filename) <= NAME_MAX);

    // Create a symbolic link.
    assert(symlinkat(filename, AT_FDCWD, linkname) == 0);

    // Read the symbolic link.
    char buffer[PATH_MAX + 1];
    ssize_t len = readlinkat(AT_FDCWD, linkname, buffer, sizeof(buffer) - 1);
    buffer[len] = '\0'; // Conforming applications should not assume that the returned contents of
                        // the symbolic link are null-terminated.

    // Check if the readlinkat was successful.
    assert(len >= 0);
    assert(len <= (ssize_t)sizeof(buffer));
    assert(strncmp(buffer, filename, len) == 0); // Check if the link points to the original file.

    // Remove the hard link.
    assert(unlinkat(AT_FDCWD, linkname, 0) == 0);

    fprintf(stderr, "passed\n");
}
