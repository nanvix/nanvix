/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _POSIX_C_SOURCE 200809 // utimensat()

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can update file timestamps with `utimensat()`.
void test_utimensat(void)
{
    fprintf(stderr, "testing utimensat() ... ");

    const char *filename = "testfile.tmp";

    // Create a temporary file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd >= 0);

    struct stat st = {0};
    struct timespec times[2] = {0};

    // Get the current file timestamps.
    assert(fstat(fd, &st) == 0);

    // Set new timestamps.
    times[0].tv_sec = st.st_atime + 20; // Access time.
    times[0].tv_nsec = 0;
    times[1].tv_sec = st.st_mtime + 10; // Modification time.
    times[1].tv_nsec = 0;

    // Update the file timestamps using utimensat and check the result.
    assert(utimensat(AT_FDCWD, filename, times, 0) == 0);

    // Verify the updated timestamps.
    assert(stat(filename, &st) == 0);
    assert(st.st_atime == times[0].tv_sec);
    assert(st.st_mtime == times[1].tv_sec);

    // Clean up.
    assert(close(fd) == 0);
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
