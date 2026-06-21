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
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// Maximum length for file content.
#define DATA_LEN_MAX 256

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can write and read to/from a file.
void test_write_read(void)
{
    fprintf(stderr, "testing write()/read() ... ");

    const char *filename = "testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    const char *data = "Hello Nanvix!";
    size_t data_len = strlen(data);
    assert(data_len <= DATA_LEN_MAX);
    char buffer[DATA_LEN_MAX + 1];

    // Create and open a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    // Write some data to the file.
    ssize_t bytes_written = write(fd, data, strlen(data));
    assert(bytes_written == (ssize_t)strlen(data));

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
    assert(strcmp(buffer, data) == 0);

    // Close the file.
    assert(close(fd) == 0);

    // Remove the test file.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
