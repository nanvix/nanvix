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

// Tests whether we can create a directory.
void test_mkdir(void)
{
    fprintf(stderr, "testing mkdir() ... ");

    const char *dirname = "testdir";
    assert(strlen(dirname) <= NAME_MAX);

    // Create a test directory.
    assert(mkdir(dirname, S_IRUSR | S_IWUSR | S_IXUSR) == 0);

    // Check if test directory exists.
    struct stat st = {0};
    assert(stat(dirname, &st) == 0);
    assert(S_ISDIR(st.st_mode) != 0);
    assert(st.st_mode & S_IRUSR);
    assert(st.st_mode & S_IWUSR);
    assert(st.st_mode & S_IXUSR);
    assert(st.st_nlink == 2); // Newly-created empty directory has 2 links: "." and "..".
    assert(st.st_atime != 0); // Access time should not be zero.
    assert(st.st_mtime != 0); // Modification time should not be zero.
    assert(st.st_ctime != 0); // Change time should not be zero.
    // TODO: Uncomment the following lines when user/group ID checks are supported.
    // assert(st.st_uid == getuid()); // User ID of the owner.
    // assert(st.st_gid == getgid()); // Group ID of the owner.

    // Remove test directory.
    assert(unlinkat(AT_FDCWD, dirname, AT_REMOVEDIR) == 0);

    fprintf(stderr, "passed\n");
}
