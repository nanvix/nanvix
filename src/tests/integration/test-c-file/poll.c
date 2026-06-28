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
#include <poll.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// Maximum length for file content.
#define POLL_TEST_DATA_MAX 256

// Timeout for poll() system call.
#define POLL_TIMEOUT 1000

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can poll a file descriptor for read/write readiness.
void test_poll(void)
{
    fprintf(stderr, "testing poll() ... ");

    const char *filename = "poll_testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    const char *data = "Hello Nanvix!";
    size_t data_len = strlen(data);
    assert(data_len <= POLL_TEST_DATA_MAX);
    char buffer[POLL_TEST_DATA_MAX + 1];

    // Open file with O_NONBLOCK so that readiness semantics are exercised.
    int fd = open(filename, O_CREAT | O_EXCL | O_RDWR | O_NONBLOCK, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    // Prepare pollfd for write readiness (POLLOUT).
    struct pollfd pfd;
    pfd.fd = fd;
    pfd.events = POLLOUT;
    pfd.revents = 0;

    // Poll for write readiness. Should be ready immediately.
    int ret = poll(&pfd, 1, POLL_TIMEOUT);
    assert(ret == 1);
    assert((pfd.revents & POLLOUT) != 0);

    // Write data.
    ssize_t bytes_written = write(fd, data, data_len);
    assert(bytes_written == (ssize_t)data_len);

    // Rewind to begin of the file.
    assert(lseek(fd, 0, SEEK_SET) == 0);

    // Reset pollfd for read readiness.
    pfd.events = POLLIN;
    pfd.revents = 0;

    // Poll for read readiness. Should be ready immediately.
    ret = poll(&pfd, 1, POLL_TIMEOUT);
    assert(ret == 1);
    assert((pfd.revents & POLLIN) != 0);

    // Read data and sanity check.
    ssize_t bytes_read = read(fd, buffer, (size_t)bytes_written);
    assert(bytes_read == bytes_written);
    buffer[bytes_read] = '\0';
    assert(strcmp(buffer, data) == 0);

    // Close file descriptor.
    assert(close(fd) == 0);

    // Re-open read-only non-blocking and poll for read.
    fd = open(filename, O_RDONLY | O_NONBLOCK);
    assert(fd != -1);

    // Seek to end so that no data is available.
    assert(lseek(fd, 0, SEEK_END) != -1);
    pfd.fd = fd;
    pfd.events = POLLIN;
    pfd.revents = 0;

    // Poll for read readiness. It should either timeout, or return readiness with POLLHUP.
    ret = poll(&pfd, 1, POLL_TIMEOUT);
    assert(ret == 0 || ((ret == 1) && ((pfd.revents & (POLLIN | POLLHUP)) != 0)));

    // Close file descriptor.
    assert(close(fd) == 0);

    // Remove test file.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
