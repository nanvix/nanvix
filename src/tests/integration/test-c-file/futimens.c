/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _POSIX_C_SOURCE 200809 // futimens()

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

// Tests whether we can update file timestamps with `futimens()`.
void test_futimens(void)
{
    fprintf(stderr, "testing futimens() ... ");

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

    // Update the file timestamps and check the result.
    assert(futimens(fd, times) == 0);

    // Verify the updated timestamps.
    assert(fstat(fd, &st) == 0);
    assert(st.st_atime == times[0].tv_sec);
    assert(st.st_mtime == times[1].tv_sec);

    // Update one timestamp while preserving the other.
    const struct timespec old_atime = st.st_atim;
    times[0].tv_nsec = UTIME_OMIT;
    times[1].tv_sec = st.st_mtime + 10;
    times[1].tv_nsec = 123456789;
    assert(futimens(fd, times) == 0);
    assert(fstat(fd, &st) == 0);
    assert(st.st_atim.tv_sec == old_atime.tv_sec);
    assert(st.st_atim.tv_nsec == old_atime.tv_nsec);
    assert(st.st_mtim.tv_sec == times[1].tv_sec);
    assert(st.st_mtim.tv_nsec == times[1].tv_nsec);

    // Reject an invalid nanosecond value.
    times[0].tv_sec = 0;
    times[0].tv_nsec = -1;
    times[1].tv_nsec = UTIME_OMIT;
    errno = 0;
    assert(futimens(fd, times) == -1);
    assert(errno == EINVAL);

    // Clean up.
    assert(close(fd) == 0);
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
