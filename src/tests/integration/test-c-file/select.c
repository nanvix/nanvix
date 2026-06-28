/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <sys/select.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// Maximum length for file content.
#define SELECT_TEST_DATA_MAX 256

// Timeout (in seconds) for select() system call.
#define SELECT_TIMEOUT_SECS 1

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can use select() to check read/write readiness on a file descriptor.
void test_select(void)
{
    fprintf(stderr, "testing select() ... ");

    const char *filename = "select_testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    const char *data = "Hello Nanvix!";
    size_t data_len = strlen(data);
    assert(data_len <= SELECT_TEST_DATA_MAX);
    char buffer[SELECT_TEST_DATA_MAX + 1];

    // Open file with O_NONBLOCK so that readiness semantics are exercised.
    int fd = open(filename, O_CREAT | O_EXCL | O_RDWR | O_NONBLOCK, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    //----------------------------------------------------------------------------------------------
    // Write readiness (select on writefds).
    //----------------------------------------------------------------------------------------------
    fd_set writefds;
    FD_ZERO(&writefds);
    FD_SET(fd, &writefds);
    struct timeval tvw;
    tvw.tv_sec = SELECT_TIMEOUT_SECS;
    tvw.tv_usec = 0;

    int ret = select(fd + 1, NULL, &writefds, NULL, &tvw);
    assert(ret == 1);
    assert(FD_ISSET(fd, &writefds));

    // Write data.
    ssize_t bytes_written = write(fd, data, data_len);
    assert(bytes_written == (ssize_t)data_len);

    // Rewind to beginning of file.
    assert(lseek(fd, 0, SEEK_SET) == 0);

    //----------------------------------------------------------------------------------------------
    // Read readiness (select on readfds).
    //----------------------------------------------------------------------------------------------
    fd_set readfds;
    FD_ZERO(&readfds);
    FD_SET(fd, &readfds);
    struct timeval tvr;
    tvr.tv_sec = SELECT_TIMEOUT_SECS;
    tvr.tv_usec = 0;

    ret = select(fd + 1, &readfds, NULL, NULL, &tvr);
    assert(ret == 1);
    assert(FD_ISSET(fd, &readfds));

    // Read data and sanity check.
    ssize_t bytes_read = read(fd, buffer, (size_t)bytes_written);
    assert(bytes_read == bytes_written);
    buffer[bytes_read] = '\0';
    assert(strcmp(buffer, data) == 0);

    // Close file descriptor.
    assert(close(fd) == 0);

    //----------------------------------------------------------------------------------------------
    // Re-open read-only non-blocking; seek to end and attempt a read readiness check.
    //----------------------------------------------------------------------------------------------
    fd = open(filename, O_RDONLY | O_NONBLOCK);
    assert(fd != -1);

    // Seek to end so that no data is left to read.
    assert(lseek(fd, 0, SEEK_END) != -1);

    FD_ZERO(&readfds);
    FD_SET(fd, &readfds);
    struct timeval tve;
    tve.tv_sec = SELECT_TIMEOUT_SECS;
    tve.tv_usec = 0;

    // For regular files POSIX specifies they are always ready; but we tolerate a timeout for
    // implementation-specific behavior while still asserting correctness when ready.
    ret = select(fd + 1, &readfds, NULL, NULL, &tve);
    assert(ret == 0 || (ret == 1 && FD_ISSET(fd, &readfds)));

    // Close file descriptor.
    assert(close(fd) == 0);

    // Remove test file.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
