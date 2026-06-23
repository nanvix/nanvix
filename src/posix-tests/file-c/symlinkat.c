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

// Tests whether we can create a symbolic link to a file.
void test_symlinkat(void)
{
    fprintf(stderr, "testing symlinkat() ... ");

    const char *filename = "README.md";
    const char *linkname = "README.link";
    assert(strlen(filename) <= NAME_MAX);

    // Create a symbolic link.
    assert(symlinkat(filename, AT_FDCWD, linkname) == 0);

    // Verify the symbolic link itself via lstat().
    struct stat st_link = {0};
    assert(lstat(linkname, &st_link) == 0);
    assert(S_ISLNK(st_link.st_mode));

    // Verify the stored path via readlink().
    char buf[PATH_MAX] = {0};
    ssize_t len = readlink(linkname, buf, sizeof(buf) - 1);
    assert(len > 0);
    buf[len] = '\0';
    assert(strcmp(buf, filename) == 0);

    // Verify the symbolic link resolves to the original file.
    struct stat st = {0};
    assert(stat(filename, &st) == 0);
    struct stat st_resolved = {0};
    assert(stat(linkname, &st_resolved) == 0);
    assert(st.st_ino == st_resolved.st_ino);
    assert(st.st_dev == st_resolved.st_dev);
    assert(st.st_atime != 0); // Access time should not be zero.
    assert(st.st_mtime != 0); // Modification time should not be zero.
    assert(st.st_ctime != 0); // Change time should not be zero.

    // Remove the symbolic link.
    assert(unlinkat(AT_FDCWD, linkname, 0) == 0);

    fprintf(stderr, "passed\n");
}
