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
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can get file status information.
void test_stat(void)
{
    fprintf(stderr, "testing stat() ... ");

    const char *filename = "testfile_stat.tmp";

    // Create a file with some content so st_size > 0.
    int fd = open(filename, O_CREAT | O_WRONLY, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    assert(write(fd, "hello", 5) == 5);
    assert(close(fd) == 0);

    struct stat st = {0};

    // Get file status information and assert result.
    assert(stat(filename, &st) == 0);
    assert(st.st_mode & S_IFREG);
    assert(st.st_size > 0);
    assert(st.st_nlink > 0);
    assert(st.st_blocks > 0);
    assert(st.st_dev != 0);
    assert(st.st_ino != 0);
    assert(st.st_uid == getuid());
    assert(st.st_gid == getgid());

    // Clean up.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}

// Tests whether hostfs reports a consistent identity through stat(), lstat(), and fstat().
void test_hostfs_stat_identity(void)
{
    fprintf(stderr, "testing hostfs stat identity ... ");

    const char *filename = "/mnt/testfile_hostfs_stat_identity.tmp";
    int fd = open(filename, O_CREAT | O_EXCL | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    struct stat path_stat = {0};
    struct stat path_lstat = {0};
    struct stat fd_stat = {0};

    assert(stat(filename, &path_stat) == 0);
    assert(lstat(filename, &path_lstat) == 0);
    assert(fstat(fd, &fd_stat) == 0);

    assert(path_stat.st_dev != 0);
    assert(path_stat.st_ino != 0);
    assert(path_stat.st_dev == path_lstat.st_dev);
    assert(path_stat.st_ino == path_lstat.st_ino);
    assert(path_stat.st_dev == fd_stat.st_dev);
    assert(path_stat.st_ino == fd_stat.st_ino);

    assert(close(fd) == 0);
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
