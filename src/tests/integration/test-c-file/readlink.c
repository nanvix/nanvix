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
void test_readlink(void)
{
    fprintf(stderr, "testing readlink() ... ");

    const char *filename = "readlink-target.tmp";
    const char *linkname = "readlink-file.link";
    assert(strlen(filename) <= NAME_MAX);

    // Create a symbolic link.
    assert(symlinkat(filename, AT_FDCWD, linkname) == 0);

    // Read the symbolic link.
    char buffer[PATH_MAX + 1];
    ssize_t len = readlink(linkname, buffer, sizeof(buffer) - 1);
    assert(len >= 0);
    assert(len < (ssize_t)sizeof(buffer));
    buffer[len] = '\0'; // Conforming applications should not assume that the returned contents of
                        // the symbolic link are null-terminated.

    // Check if the readlink was successful.
    assert(strcmp(buffer, filename) == 0); // Check if the link points to the original file.

    // Remove the symbolic link.
    assert(unlinkat(AT_FDCWD, linkname, 0) == 0);

    fprintf(stderr, "passed\n");
}
