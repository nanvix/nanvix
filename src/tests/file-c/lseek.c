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

// Tests whether we can manipulate file offsets using lseek.
void test_lseek(void)
{
    fprintf(stderr, "testing lseek() ... ");

    const char *filename = "testfile.tmp";
    assert(strlen(filename) <= NAME_MAX);

    // Data to write using write.
    const char *data1 = "Hello ";
    const char *data2 = "Nanvix!";
    size_t data1_len = strlen(data1);
    size_t data2_len = strlen(data2);
    assert(data1_len + data2_len <= DATA_LEN_MAX);

    char buffer[DATA_LEN_MAX + 1];

    // Create and open a test file.
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    // Write data1 to the file.
    ssize_t bytes_written = write(fd, data1, data1_len);
    assert(bytes_written == (ssize_t)data1_len);

    // Use lseek to move the file offset to the end of data1.
    off_t offset = lseek(fd, 0, SEEK_END);
    assert(offset == (off_t)data1_len);

    // Write data2 at the current offset.
    bytes_written = write(fd, data2, data2_len);
    assert(bytes_written == (ssize_t)data2_len);

    // Use lseek to move the file offset back to the beginning.
    offset = lseek(fd, 0, SEEK_SET);
    assert(offset == 0);

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
