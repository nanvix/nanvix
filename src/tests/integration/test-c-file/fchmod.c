/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

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
void test_fchmod(void)
{
    fprintf(stderr, "testing fchmod() ... ");

    struct stat st = {0};

    const char *filename = "testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    // Create and open a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    // Save current access permissions.
    assert(fstat(fd, &st) == 0);
    mode_t original_mode = st.st_mode;

    // Change access permissions and assert result.
    assert(fchmod(fd, st.st_mode & ~(S_IRGRP | S_IROTH)) == 0);
    assert(fstat(fd, &st) == 0);
    assert((st.st_mode & S_IRGRP) == 0);
    assert((st.st_mode & S_IROTH) == 0);

    // Restore the original access permissions.
    assert(fchmod(fd, original_mode) == 0);
    assert(fstat(fd, &st) == 0);
    assert(st.st_mode == original_mode);

    // Close the file.
    assert(close(fd) == 0);

    // Remove the test file.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
