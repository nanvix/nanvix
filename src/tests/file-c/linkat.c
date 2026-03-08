/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _POSIX_C_SOURCE 200809 // AT_FDCWD

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can create a hard link to a file.
void test_linkat(void)
{
    fprintf(stderr, "testing linkat() ... ");

    const char *filename = "README.md";
    const char *linkname = "README.link";
    assert(strlen(filename) <= NAME_MAX);

    // Create a hard link.
    assert(linkat(AT_FDCWD, filename, AT_FDCWD, linkname, 0) == 0);

    // Get information from the original file.
    struct stat st = {0};
    assert(stat(filename, &st) == 0);

    // Get information from the hard link.
    struct stat st_link = {0};
    assert(stat(linkname, &st_link) == 0);

    // Check if the hard link points to the same inode as the original file.
    assert(st.st_ino == st_link.st_ino);
    assert(st.st_dev == st_link.st_dev);
    assert(st.st_nlink == st_link.st_nlink); // Link count should be the same.
    assert(st.st_mode == st_link.st_mode);   // Mode should be the same.
    assert(st.st_size == st_link.st_size);   // Size should be the same.
    assert(st.st_mtime == st_link.st_mtime); // Modification time should be the same.
    assert(st.st_ctime == st_link.st_ctime); // Change time should be the same.
    assert(st.st_atime != 0);                // Access time should not be zero.
    assert(st.st_mtime != 0);                // Modification time should not be zero.
    assert(st.st_ctime != 0);                // Change time should not be zero.

    // Remove the hard link.
    assert(unlinkat(AT_FDCWD, linkname, 0) == 0);

    fprintf(stderr, "passed\n");
}
