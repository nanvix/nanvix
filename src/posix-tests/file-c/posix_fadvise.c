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
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can use posix_fadvise on a file.
void test_posix_fadvise(void)
{
    fprintf(stderr, "testing posix_fadvise() ... ");

    const char *filename = "testfile.tmp";

    // Create and open a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    // Write some data to the file.
    const char *data = "Hello Nanvix!";
    ssize_t bytes_written = write(fd, data, strlen(data));
    assert(bytes_written == (ssize_t)strlen(data));

    // Use posix_fadvise to give advice about file access.
    int ret = posix_fadvise(fd, 0, 0, POSIX_FADV_SEQUENTIAL);
    assert(ret == 0);

    // Close the file.
    assert(close(fd) == 0);

    // Remove the test file.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
