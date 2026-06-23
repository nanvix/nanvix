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
// Standalone Functions
//==================================================================================================

// Tests whether we can create and unlink a file.
void test_create_unlink(void)
{
    fprintf(stderr, "testing create()/unlink() ... ");

    const char *filename = "testfile.txt";
    assert(strlen(filename) <= NAME_MAX);

    // Create a test file.
    int fd = open(filename, O_CREAT | O_WRONLY, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    assert(close(fd) == 0);

    // Remove test file.
    assert(unlink(filename) == 0);

    fprintf(stderr, "passed\n");
}
