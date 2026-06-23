/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/select.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can update file timestamps with `utimes()`.
void test_utimes(void)
{
    fprintf(stderr, "testing utimes() ... ");

    struct stat st = {0};
    const char *filename = "testfile.tmp";

    // Create a temporary file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd >= 0);
    assert(close(fd) == 0);

    // Get the current file timestamps.
    assert(stat(filename, &st) == 0);

    // Set new timestamps.
    struct timeval times[2] = {{
                                   .tv_sec = st.st_atime + 20, // Access time.
                                   .tv_usec = 0,
                               },
                               {
                                   .tv_sec = st.st_mtime + 10, // Modification time.
                                   .tv_usec = 0,
                               }};

    // Update the file timestamps using futimes and check the result.
    assert(utimes(filename, times) == 0);

    // Verify the updated timestamps.
    assert(stat(filename, &st) == 0);
    assert(st.st_atime == times[0].tv_sec);
    assert(st.st_mtime == times[1].tv_sec);

    // Clean up.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
