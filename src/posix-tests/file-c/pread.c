/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _POSIX_C_SOURCE 200809 // pread()

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

// Tests whether we can read from a file using pread.
void test_pread(void)
{
    fprintf(stderr, "testing pread() ... ");

    const char *filename = "testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    // Data to write to the file.
    const char *data = "Hello Nanvix!";
    size_t data_len = strlen(data);
    assert(data_len <= DATA_LEN_MAX);

    char buffer[DATA_LEN_MAX + 1];

    // Define a constant for the read offset.
    const off_t DATA_OFFSET = 0;

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

    // Read data using pread.
    ssize_t bytes_read = pread(fd, buffer, data_len, DATA_OFFSET);
    assert(bytes_read == (ssize_t)data_len);

    // Null-terminate the buffer and assert contents.
    buffer[bytes_read] = '\0';
    assert(strcmp(buffer, data) == 0);

    // Close the file.
    assert(close(fd) == 0);

    // Remove the test file.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
