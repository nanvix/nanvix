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

// Tests wether we can create and unlink a file.
void test_create_unlink(void)
{
    const char *filename = "testfile.txt";

    // Create a file.
    int fd = open(filename, O_CREAT | O_WRONLY, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    // Unlink the file.
    assert(unlink(filename) == 0);
}
