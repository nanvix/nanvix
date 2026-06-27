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

// Tests whether we can check access permissions of a file.
void test_faccessat(void)
{
    fprintf(stderr, "testing faccessat() ... ");

    const char *filename = "testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    // Create a test file.
    int fd = open(filename, O_CREAT | O_RDONLY, S_IRUSR | S_IRGRP | S_IROTH);
    assert(fd != -1);
    assert(close(fd) == 0);

    // Check read and write access.
    assert(faccessat(AT_FDCWD, filename, R_OK, 0) == 0);
    assert(faccessat(AT_FDCWD, filename, W_OK, 0) == -1);

    // Add write access and check again.
    assert(fchmodat(AT_FDCWD, filename, S_IWUSR | S_IRUSR, 0) == 0);
    assert(faccessat(AT_FDCWD, filename, R_OK, 0) == 0);
    assert(faccessat(AT_FDCWD, filename, W_OK, 0) == 0);

    // Remove write access and check again.
    assert(fchmodat(AT_FDCWD, filename, S_IRUSR, 0) == 0);
    assert(faccessat(AT_FDCWD, filename, R_OK, 0) == 0);
    assert(faccessat(AT_FDCWD, filename, W_OK, 0) == -1);

    // Close and remove the test file.
    assert(unlinkat(AT_FDCWD, filename, 0) == 0);

    fprintf(stderr, "passed\n");
}
