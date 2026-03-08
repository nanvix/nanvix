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

// Tests whether we can change the current working directory.
void test_chdir(void)
{
    fprintf(stderr, "testing chdir() ... ");

    const char *dirname = "src";
    assert(strlen(dirname) <= NAME_MAX);

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

    fprintf(stderr, "passed\n");
}
