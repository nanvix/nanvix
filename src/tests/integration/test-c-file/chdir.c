/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _POSIX_C_SOURCE 200809

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

// Tests whether we can change the current working directory.
void test_chdir(void)
{
    fprintf(stderr, "testing chdir() ... ");

    const char *dirname = "testdir_chdir";
    assert(strlen(dirname) <= NAME_MAX);

    // Create a temporary directory.
    assert(mkdir(dirname, S_IRUSR | S_IWUSR | S_IXUSR) == 0);

    char original_cwd[PATH_MAX];
    char new_cwd[PATH_MAX];

    // Get the current working directory.
    assert(getcwd(original_cwd, sizeof(original_cwd)) != NULL);

    // Change to the target directory.
    assert(chdir(dirname) == 0);

    // Verify the current working directory has changed.
    assert(getcwd(new_cwd, sizeof(new_cwd)) != NULL);
    assert(strcmp(new_cwd, original_cwd) != 0);

    // Restore the original working directory.
    assert(chdir(original_cwd) == 0);

    // Verify the current working directory is restored.
    assert(getcwd(new_cwd, sizeof(new_cwd)) != NULL);
    assert(strcmp(new_cwd, original_cwd) == 0);

    // Clean up.
    assert(unlinkat(AT_FDCWD, dirname, AT_REMOVEDIR) == 0);

    fprintf(stderr, "passed\n");
}
