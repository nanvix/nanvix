/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _POSIX_C_SOURCE 200112L // posix_fadvise()

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can use posix_fallocate on a file.
void test_posix_fallocate(void)
{
    fprintf(stderr, "testing posix_fallocate() ... ");

    const char *filename = "testfile.tmp";
    const off_t file_length = 1024;

    // Create and open a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    // Allocate space for the file.
    int ret = posix_fallocate(fd, 0, file_length);
    assert(ret == 0);

    // Verify the file size.
    off_t file_size = lseek(fd, 0, SEEK_END);
    assert(file_size == file_length);

    // Close the file.
    assert(close(fd) == 0);

    // Remove the test file.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
