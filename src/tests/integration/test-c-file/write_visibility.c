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
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// Maximum length for file content.
#define DATA_LEN_MAX 64

//==================================================================================================
// Private Functions
//==================================================================================================

// Asserts that a newly opened file descriptor and stat() see the given contents of a file.
static void assert_file_contents(const char *filename, const char *data)
{
    const size_t data_len = strlen(data);
    assert(data_len <= DATA_LEN_MAX);
    char buffer[DATA_LEN_MAX + 1];

    // Read the file through a new file descriptor, asking for more data than the file holds.
    int fd = open(filename, O_RDONLY);
    assert(fd != -1);
    size_t total = 0;
    ssize_t n = 0;
    while ((n = read(fd, buffer + total, sizeof(buffer) - total)) > 0) {
        total += (size_t)n;
    }
    assert(n == 0);
    assert(total == data_len);
    assert(memcmp(buffer, data, data_len) == 0);

    // Check the sizes reported by fstat() on the new file descriptor and by stat().
    struct stat st = {0};
    assert(fstat(fd, &st) == 0);
    assert(st.st_size == (off_t)data_len);
    assert(close(fd) == 0);
    assert(stat(filename, &st) == 0);
    assert(st.st_size == (off_t)data_len);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether data written through a file descriptor is visible to file descriptors that are
// opened afterwards, while the writing file descriptor is still open.
void test_write_visibility(void)
{
    fprintf(stderr, "testing write visibility ... ");

    const char *filename = "/testfile_visibility.tmp";
    const char *line = "KEY=value\n";
    const char *other_line = "OTHER=value\n";
    const char *prefix = "KEY";

    // Create the file like mkstemp() does, write to it, and keep it open.
    int fd = open(filename, O_CREAT | O_EXCL | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    assert(write(fd, line, strlen(line)) == (ssize_t)strlen(line));
    assert_file_contents(filename, "KEY=value\n");

    // Appending to the file should be visible too.
    assert(write(fd, other_line, strlen(other_line)) == (ssize_t)strlen(other_line));
    assert_file_contents(filename, "KEY=value\nOTHER=value\n");

    // And so should truncating it.
    assert(ftruncate(fd, (off_t)strlen(prefix)) == 0);
    assert_file_contents(filename, prefix);
    assert(close(fd) == 0);

    // Truncating the file when opening it should be visible before anything is written to it.
    fd = open(filename, O_WRONLY | O_TRUNC);
    assert(fd != -1);
    assert_file_contents(filename, "");
    assert(close(fd) == 0);

    // Remove the test file.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
