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

// Tests whether we can truncate a file descriptor.
void test_ftruncate(void)
{
    fprintf(stderr, "testing ftruncate() ... ");

    const size_t SIZE = 1024;

    const char *filename = "testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    // Create a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    assert(ftruncate(fd, SIZE) == 0);

    // Get file size and assert result.
    struct stat st = {0};
    assert(fstat(fd, &st) == 0);
    assert(st.st_size == SIZE);

    // Close and remove the test file.
    assert(close(fd) == 0);
    assert(unlinkat(AT_FDCWD, filename, 0) == 0);

    fprintf(stderr, "passed\n");
}
