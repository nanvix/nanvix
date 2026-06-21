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

// Tests whether we can change access permissions of a file.
void test_fchmodat(void)
{
    fprintf(stderr, "testing fchmodat() ... ");

    const char *filename = "testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    struct stat st = {0};

    // Create a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    assert(close(fd) == 0);

    // Save current access permissions.
    assert(stat(filename, &st) == 0);
    mode_t original_mode = st.st_mode;

    // Change access permissions and assert result.
    assert(fchmodat(AT_FDCWD, filename, st.st_mode & ~(S_IRGRP | S_IROTH), 0) == 0);
    assert(stat(filename, &st) == 0);
    assert((st.st_mode & S_IRGRP) == 0);
    assert((st.st_mode & S_IROTH) == 0);

    // Restore the original access permissions.
    assert(fchmodat(AT_FDCWD, filename, original_mode, 0) == 0);
    assert(stat(filename, &st) == 0);
    assert(st.st_mode == original_mode);

    // Close and remove the test file.
    assert(unlinkat(AT_FDCWD, filename, 0) == 0);

    fprintf(stderr, "passed\n");
}
