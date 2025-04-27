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
#include <sys/stat.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can get file status information.
void test_stat(void)
{
    fprintf(stderr, "testing stat() ... ");

    const char *filename = "README.md";

    struct stat st = {0};

    // Get file status information and assert result.
    assert(stat(filename, &st) == 0);
    assert(st.st_mode & S_IFREG);
    assert(st.st_size > 0);
    assert(st.st_nlink > 0);
    assert(st.st_blocks > 0);
    assert(st.st_atim.tv_sec > 0);
    assert(st.st_mtim.tv_sec > 0);
    assert(st.st_ctim.tv_sec > 0);
    assert(st.st_atim.tv_nsec >= 0);
    assert(st.st_mtim.tv_nsec >= 0);
    assert(st.st_ctim.tv_nsec >= 0);
    assert(st.st_blocks > 0);
    assert(st.st_dev != 0);
    assert(st.st_ino != 0);
    assert(st.st_nlink > 0);

    fprintf(stderr, "passed\n");
}
