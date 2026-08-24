/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <fcntl.h>
#include <stdio.h>
#include <sys/select.h>
#include <sys/stat.h>
#include <unistd.h>
#include <utime.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can update file timestamps with `utime()`.
void test_utime(void)
{
    fprintf(stderr, "testing utime() ... ");

    struct stat st = {0};
    const char *filename = "testfile.tmp";

    // Create a temporary file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd >= 0);
    assert(close(fd) == 0);

    // Get the current file timestamps.
    assert(stat(filename, &st) == 0);

    // Set new timestamps.
    struct utimbuf times = {
        .actime = st.st_atime + 20,
        .modtime = st.st_mtime + 10,
    };
    assert(utime(filename, &times) == 0);

    // Verify the updated timestamps.
    assert(stat(filename, &st) == 0);
    assert(st.st_atime == times.actime);
    assert(st.st_mtime == times.modtime);

    // Clean up.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}

// Tests whether `utime()` preserves timestamps imported by mkramfs.
void test_utime_preserve_imported_timestamps(void)
{
    fprintf(stderr, "testing utime() with imported timestamps ... ");

    struct stat st = {0};
    const char *filename = "testfile-imported.tmp";

    // Create a temporary file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd >= 0);
    assert(close(fd) == 0);

    // Preserve timestamps from a file imported by mkramfs.
    assert(stat("marker.txt", &st) == 0);
    struct utimbuf times = {
        .actime = st.st_atime,
        .modtime = st.st_mtime,
    };
    assert(utime(filename, &times) == 0);

    // Verify the preserved timestamps.
    assert(stat(filename, &st) == 0);
    assert(st.st_atime == times.actime);
    assert(st.st_mtime == times.modtime);

    // Clean up.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
