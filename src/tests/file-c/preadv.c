/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _BSD_SOURCE // preadv()

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

// Tests whether we can read from a file using preadv.
void test_preadv(void)
{
    fprintf(stderr, "testing preadv() ... ");

    const char *filename = "testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    // Data to write to the file.
    const char *data = "Hello Nanvix!";
    size_t data_len = strlen(data);
    assert(data_len <= DATA_LEN_MAX);

    // Define a constant for the write offset.
    const off_t DATA_OFFSET = 10;

    // Create and open a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    // Move the file offset to the specified position.
    assert(lseek(fd, DATA_OFFSET, SEEK_SET) == DATA_OFFSET);

    // Write data to the file at the current offset.
    assert(write(fd, data, data_len) == (ssize_t)data_len);

    // Close the file.
    assert(close(fd) == 0);

    // Reopen the file for reading.
    fd = open(filename, O_RDONLY);
    assert(fd != -1);

    // Prepare buffers for preadv.
    char buffer1[7]; // "Hello "
    char buffer2[8]; // "Nanvix!"
    struct iovec iov[2];
    iov[0].iov_base = buffer1;
    iov[0].iov_len = 6; // Length of "Hello "
    iov[1].iov_base = buffer2;
    iov[1].iov_len = 7; // Length of "Nanvix!"

    // Read data using preadv at the specified offset.
    ssize_t bytes_read = preadv(fd, iov, 2, DATA_OFFSET);
    assert(bytes_read == (ssize_t)data_len);

    // Null-terminate the buffers.
    buffer1[6] = '\0';
    buffer2[7] = '\0';

    // Assert contents.
    assert(strcmp(buffer1, "Hello ") == 0);
    assert(strcmp(buffer2, "Nanvix!") == 0);

    // Close the file.
    assert(close(fd) == 0);

    // Remove the test file.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
