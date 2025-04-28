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

// Tests whether we can read from a file using vectorized I/O.
void test_readv(void)
{
    fprintf(stderr, "testing readv() ... ");

    const char *filename = "testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    // Data to write to the file.
    const char *data = "Hello Nanvix!";
    size_t data_len = strlen(data);
    assert(data_len <= DATA_LEN_MAX);

    // Create and open a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    // Write data to the file.
    ssize_t bytes_written = write(fd, data, data_len);
    assert(bytes_written == (ssize_t)data_len);

    // Close the file.
    assert(close(fd) == 0);

    // Reopen the file for reading.
    fd = open(filename, O_RDONLY);
    assert(fd != -1);

    // Prepare iovec structures for reading.
    char buffer1[7]; // To read "Hello "
    char buffer2[8]; // To read "Nanvix!"
    struct iovec iov[2];
    iov[0].iov_base = buffer1;
    iov[0].iov_len = sizeof(buffer1) - 1;
    iov[1].iov_base = buffer2;
    iov[1].iov_len = sizeof(buffer2) - 1;

    // Read data from the file using readv.
    ssize_t bytes_read = readv(fd, iov, 2);
    assert(bytes_read == (ssize_t)data_len);

    // Null-terminate the buffers.
    buffer1[sizeof(buffer1) - 1] = '\0';
    buffer2[sizeof(buffer2) - 1] = '\0';

    // Assert contents.
    assert(strcmp(buffer1, "Hello ") == 0);
    assert(strcmp(buffer2, "Nanvix!") == 0);

    // Close the file.
    assert(close(fd) == 0);

    // Remove the test file.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
