/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <dirent.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests if we can open/close a directory stream.
static void test_opendir_closedir(const char *dirname)
{
    DIR *dirp = NULL;

    dirp = opendir(dirname);
    assert(dirp != NULL);

    assert(closedir(dirp) == 0);
}

// Tests if we can read a directory stream.
static void test_readdir(const char *dirname)
{
    fprintf(stderr, "testing readdir() ... ");

    DIR *dirp = NULL;
    struct dirent *entry = NULL;

    dirp = opendir(dirname);
    assert(dirp != NULL);

    assert(errno == 0);
    while ((entry = readdir(dirp)) != NULL) {
        assert(strlen(entry->d_name) > 0);
        assert(entry->d_ino != 0);
    }
    assert(errno == 0);

    assert(closedir(dirp) == 0);

    fprintf(stderr, "passed\n");
}

// Tests system calls on directory entries.
void test_dirent(void)
{
    const char *dirname = ".";

    test_opendir_closedir(dirname);
    test_readdir(dirname);
}
