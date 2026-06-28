/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can open and close a file.
void test_open_close(void)
{
    const char *filename = "testfile_open_close.tmp";

    // Create a file.
    int fd = open(filename, O_CREAT | O_WRONLY, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    assert(close(fd) == 0);

    // Open the file read-only.
    fd = open(filename, O_RDONLY, 0);
    assert(fd != -1);

    // Close the file.
    assert(close(fd) == 0);

    // Clean up.
    assert(unlink(filename) == 0);
}
