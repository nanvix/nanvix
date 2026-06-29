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

// Tests whether we can truncate a file referenced by a pathname.
void test_truncate(void)
{
    fprintf(stderr, "testing truncate() ... ");

    const size_t SIZE = 1024;

    const char *filename = "testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    // Create a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    assert(close(fd) == 0);

    // Extend the file by truncating it to a larger size.
    assert(truncate(filename, SIZE) == 0);

    // Get file size and assert result.
    struct stat st = {0};
    assert(stat(filename, &st) == 0);
    assert(st.st_size == (off_t)SIZE);

    // Shrink the file by truncating it to a smaller size.
    assert(truncate(filename, SIZE / 2) == 0);

    st = (struct stat){0};
    assert(stat(filename, &st) == 0);
    assert(st.st_size == (off_t)(SIZE / 2));

    // Remove the test file.
    assert(unlinkat(AT_FDCWD, filename, 0) == 0);

    fprintf(stderr, "passed\n");
}
