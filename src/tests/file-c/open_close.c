/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <fcntl.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can open and close a file.
void test_open_close(void)
{
    const char *filename = "README.md";

    // Open a file.
    int fd = open(filename, O_RDONLY, 0);
    assert(fd != -1);

    // Close the file.
    assert(close(fd) == 0);
}
