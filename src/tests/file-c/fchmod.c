/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <fcntl.h>
#include <stdio.h>
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can change access permissions of a file.
void test_fchmod(void)
{
    const char *filename = "README.md";

    struct stat st = {0};

    // Open a file.
    int fd = open(filename, O_RDONLY, 0);
    assert(fd != -1);

    // Save currente access permissions.
    assert(fstat(fd, &st) == 0);
    mode_t original_mode = st.st_mode;

    // Change access permissions and assert result.
    assert(fchmod(fd, st.st_mode & ~(S_IRGRP | S_IROTH)) == 0);
    assert(fstat(fd, &st) == 0);
    fprintf(stderr, "New mode: %08o\n", st.st_mode);
    assert((st.st_mode & S_IRGRP) == 0);
    assert((st.st_mode & S_IROTH) == 0);

    // Restore the original access permissions.
    assert(fchmod(fd, original_mode) == 0);
    assert(fstat(fd, &st) == 0);
    fprintf(stderr, "Restored mode: %08o\n", st.st_mode);
    assert(st.st_mode == original_mode);

    // Close the file.
    assert(close(fd) == 0);
}
