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
#include <string.h>
#include <unistd.h>

// TODO: Remove the following import when it is exported by Newlib.
extern int posix_fadvise(int fd, off_t offset, off_t len, int advice);

//==================================================================================================
// Constants
//==================================================================================================

// TODO: Remove the following constants when they are exported by Newlib.
#define POSIX_FADV_NORMAL 0
#define POSIX_FADV_SEQUENTIAL 1
#define POSIX_FADV_RANDOM 2
#define POSIX_FADV_WILLNEED 3
#define POSIX_FADV_DONTNEED 4
#define POSIX_FADV_NOREUSE 5

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
