/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _POSIX_C_SOURCE 200809 // pwrite()

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// Maximum length for file content.
#define DATA_LEN_MAX 256

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can write to a file using pwrite.
void test_pwrite(void)
{
    fprintf(stderr, "testing pwrite() ... ");

    const char *filename = "testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    // Data to write using pwrite.
    const char *data1 = "Hello ";
    const char *data2 = "Nanvix!";
    size_t data1_len = strlen(data1);
    size_t data2_len = strlen(data2);
    assert(data1_len + data2_len <= DATA_LEN_MAX);

    char buffer[DATA_LEN_MAX + 1];

    // Define a constant for the read offset.
    const off_t DATA_OFFSET = 0;

    // Create and open a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    // Write data1 at an offset of 10 using pwrite.
    ssize_t bytes_written = pwrite(fd, data1, data1_len, DATA_OFFSET);
    assert(bytes_written == (ssize_t)data1_len);

    // Write data2 at the end of data1 using pwrite.
    bytes_written = pwrite(fd, data2, data2_len, DATA_OFFSET + data1_len);
    assert(bytes_written == (ssize_t)data2_len);

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
