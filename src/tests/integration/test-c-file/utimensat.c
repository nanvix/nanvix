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
#include <sys/stat.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>
#include <utime.h>

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
#ifdef __NANVIX_STANDALONE__
    times[0].tv_sec = st.st_atime; // Access time is date-only on FAT.
#else
    times[0].tv_sec = st.st_atime + 20; // Access time.
#endif
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

    const char *dirname = "testdir_utimensat";
    const char *childname = "child.tmp";
    const char *childpath = "testdir_utimensat/child.tmp";

    assert(mkdir(dirname, S_IRWXU) == 0);
    int dirfd = open(dirname, O_RDONLY | O_DIRECTORY);
    assert(dirfd >= 0);
    int childfd = openat(dirfd, childname, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(childfd >= 0);
    assert(close(childfd) == 0);

    assert(stat(childpath, &st) == 0);
    times[0].tv_sec = st.st_atime;
    times[1].tv_sec = st.st_mtime + 2;

    assert(utimensat(dirfd, childname, times, 0) == 0);
    assert(stat(childpath, &st) == 0);
    assert(st.st_mtime == times[1].tv_sec);
    assert(utimensat(dirfd, childname, NULL, 0) == 0);

    assert(stat(childpath, &st) == 0);
    times[0].tv_sec = st.st_atime;
    times[1].tv_sec = st.st_mtime + 2;
    assert(utimensat(AT_FDCWD, childpath, times, AT_SYMLINK_NOFOLLOW) == 0);
    assert(stat(childpath, &st) == 0);
    assert(st.st_mtime == times[1].tv_sec);

    errno = 0;
    assert(utimensat(AT_FDCWD, childpath, times, 1 << 30) == -1);
    assert(errno == EINVAL);

    // Omitting both timestamps does not require resolving the path or directory descriptor.
    times[0].tv_nsec = UTIME_OMIT;
    times[1].tv_nsec = UTIME_OMIT;
    assert(utimensat(123456, "missing.tmp", times, 0) == 0);

    assert(close(dirfd) == 0);
    assert(unlink(childpath) == 0);
    assert(rmdir(dirname) == 0);

    fprintf(stderr, "passed\n");
}

// Tests whether the time-setting calls accept a NULL `times` (set to current
// time). NULL returns success even on FAT32, where timestamps are ignored.
void test_utimensat_now(void)
{
    fprintf(stderr, "testing NULL times (set to now) ... ");

    const char *filename = "testfile_now.tmp";

    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd >= 0);

    // A NULL `times` means "set both timestamps to the current time".
    assert(utimensat(AT_FDCWD, filename, NULL, 0) == 0);
    assert(futimens(fd, NULL) == 0);
    assert(utimes(filename, NULL) == 0);
    assert(utime(filename, NULL) == 0);

    assert(close(fd) == 0);
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
