/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _BSD_SOURCE // pwrivev()

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

// Tests whether we can write to a file using pwritev.
void test_pwritev(void)
{
    fprintf(stderr, "testing pwritev() ... ");

    const char *filename = "testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    // Data to write using pwritev.
    const char *data1 = "Hello ";
    const char *data2 = "Nanvix!";
    size_t data1_len = strlen(data1);
    size_t data2_len = strlen(data2);
    assert(data1_len + data2_len <= DATA_LEN_MAX);

    char buffer[DATA_LEN_MAX + 1];

    // Define a constant for the read offset.
    const off_t DATA_OFFSET = 10;

    // Create and open a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    // Prepare iovec structures for pwritev.
    struct iovec iov[2];
    iov[0].iov_base = (void *)data1;
    iov[0].iov_len = data1_len;
    iov[1].iov_base = (void *)data2;
    iov[1].iov_len = data2_len;

    // Write data1 and data2 using pwritev at the specified offset.
    ssize_t bytes_written = pwritev(fd, iov, 2, DATA_OFFSET);
    assert(bytes_written == (ssize_t)(data1_len + data2_len));

    // Close the file.
    assert(close(fd) == 0);

    // Reopen the file for reading.
    fd = open(filename, O_RDONLY);
    assert(fd != -1);

    // Use lseek to set the read offset.
    assert(lseek(fd, DATA_OFFSET, SEEK_SET) == DATA_OFFSET);

    // Read the data back.
    ssize_t bytes_read = read(fd, buffer, data1_len + data2_len);
    assert(bytes_read == (ssize_t)(data1_len + data2_len));

    // Null-terminate the buffer and assert contents.
    buffer[bytes_read] = '\0';
    assert(strcmp(buffer, "Hello Nanvix!") == 0);

    // Close the file.
    assert(close(fd) == 0);

    // Remove the test file.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
