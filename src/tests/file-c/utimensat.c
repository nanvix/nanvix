/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _POSIX_C_SOURCE 200809

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <fcntl.h> // AT_FDCWD
#include <stdio.h>
#include <sys/stat.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can change file access and modification times.
void test_utimensat(void)
{
    fprintf(stderr, "testing %s()...\n", __func__);

    const char *filename = "README.md";

    // Save file original access and modification times.
    struct stat saved_stat = {0};
    assert(stat(filename, &saved_stat) == 0);
    struct timespec saved_times[2] = {saved_stat.st_atim, saved_stat.st_mtim};

    // Change file access and modification times.
    struct timespec new_times[2] = {saved_stat.st_atim, saved_stat.st_mtim};
    new_times[0].tv_sec += 1;
    new_times[1].tv_nsec += 1;
    assert(utimensat(AT_FDCWD, filename, new_times, 0) == 0);
    assert(stat(filename, &saved_stat) == 0);
    assert(saved_stat.st_atim.tv_sec == new_times[0].tv_sec);
    assert(saved_stat.st_mtim.tv_nsec == new_times[1].tv_nsec);

    // Restore file original access and modification times.
    assert(utimensat(AT_FDCWD, filename, saved_times, 0) == 0);
    assert(stat(filename, &saved_stat) == 0);
    assert(saved_stat.st_atim.tv_sec == saved_times[0].tv_sec);
    assert(saved_stat.st_mtim.tv_nsec == saved_times[1].tv_nsec);
    assert(saved_stat.st_atim.tv_nsec == saved_times[0].tv_nsec);
}
