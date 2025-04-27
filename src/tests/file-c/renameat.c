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
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can rename a file.
void test_renameat(void)
{
    fprintf(stderr, "testing renameat() ... ");

    const char *filename = "test.txt";
    assert(sizeof(filename) < PATH_MAX);

    const char *renamed_filename = "renamed.txt";
    assert(sizeof(renamed_filename) < PATH_MAX);

    // Create a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    assert(close(fd) == 0);

    // Rename test file and check result.
    assert(renameat(AT_FDCWD, filename, AT_FDCWD, renamed_filename) == 0);

    // Remove renamed test file.
    assert(unlink(renamed_filename) == 0);

    fprintf(stderr, "passed\n");
}
