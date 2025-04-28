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
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    assert(close(fd) == 0);

    // Check read and write access.
    assert(faccessat(AT_FDCWD, filename, R_OK | W_OK, 0) == 0);

    // Remove write access and check again.
    assert(fchmodat(AT_FDCWD, filename, S_IRUSR, 0) == 0);
    assert(faccessat(AT_FDCWD, filename, W_OK, 0) == -1);

    // Restore write access and check again.
    assert(fchmodat(AT_FDCWD, filename, S_IRUSR | S_IWUSR, 0) == 0);
    assert(faccessat(AT_FDCWD, filename, W_OK, 0) == 0);

    // Close and remove the test file.
    assert(unlinkat(AT_FDCWD, filename, 0) == 0);

    fprintf(stderr, "passed\n");
}
