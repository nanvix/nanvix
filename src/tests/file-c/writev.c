/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <sys/uio.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// Maximum length for file content.
#define DATA_LEN_MAX 256

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can write to a file using vectorized I/O.
void test_writev(void)
{
    fprintf(stderr, "testing writev() ... ");

    const char *filename = "testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    // Data to write using writev.
    const char *data1 = "Hello ";
    const char *data2 = "Nanvix!";
    size_t data1_len = strlen(data1);
    size_t data2_len = strlen(data2);
    assert(data1_len + data2_len <= DATA_LEN_MAX);

    struct iovec iov[2];
    iov[0].iov_base = (void *)data1;
    iov[0].iov_len = data1_len;
    iov[1].iov_base = (void *)data2;
    iov[1].iov_len = data2_len;

    char buffer[DATA_LEN_MAX + 1];

    // Create and open a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    // Write data to the file using writev.
    ssize_t bytes_written = writev(fd, iov, 2);
    assert(bytes_written == (ssize_t)(data1_len + data2_len));

    // Close the file.
    assert(close(fd) == 0);

    // Reopen the file for reading.
    fd = open(filename, O_RDONLY);
    assert(fd != -1);

    // Read the data back.
    ssize_t bytes_read = read(fd, buffer, bytes_written);
    assert(bytes_read == bytes_written);

    // Null-terminate the buffer and assert contents.
    buffer[bytes_read] = '\0';
    assert(strcmp(buffer, "Hello Nanvix!") == 0);

    // Close the file.
    assert(close(fd) == 0);

    // Remove the test file.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
