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

// Tests whether we can change the permision of fd.
void test_fchmod(void)
{
    const char *filename = "README.md";

    struct stat st;

    // Open a file.
    int fd = open(filename, O_RDONLY, 0);
    assert(fd != -1);

    // Change file permissions to make it unreadable by others and group.
    assert(fchmod(fd, st.st_mode & ~(S_IRGRP | S_IROTH)) == 0);

    assert(fstat(fd, &st) == 0);

    fprintf(stderr, "New mode: %08o\n", st.st_mode);

    // Check that the file is not readable by others and group.
    assert((st.st_mode & S_IRGRP) == 0);
    assert((st.st_mode & S_IROTH) == 0);

    // Close the file.
    assert(close(fd) == 0);
}
