/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
#define _POSIX_C_SOURCE 200809 // strnlen()

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
// Imports
//==================================================================================================

// Tests whether we can change the current working directory.
void test_fchdir(void)
{
    fprintf(stderr, "testing fchdir() ... ");

    const char *TARGET_DIRECTORY = "src/";

    char initial_working_directory[PATH_MAX] = {0};

    // Open initial directory.
    int initial_directory_fd = open(".", O_RDONLY);
    assert(initial_directory_fd != -1);

    // Store initial working directory.
    char *initial_cwd = getcwd(initial_working_directory, sizeof(initial_working_directory));
    assert(initial_cwd != NULL);
    assert(initial_cwd == initial_working_directory);
    size_t initial_cwd_length = strnlen(initial_cwd, PATH_MAX);
    assert((initial_cwd_length > 0) && (initial_cwd_length < PATH_MAX));

    // Open target directory.
    int target_directory_fd = open(TARGET_DIRECTORY, O_RDONLY);
    assert(target_directory_fd != -1);

    // Change working directory.
    int ret = fchdir(target_directory_fd);
    assert(ret == 0);

    // Get current working directory and assert result.
    char target_working_directory[PATH_MAX] = {0};
    char *target_cwd = getcwd(target_working_directory, sizeof(target_working_directory));
    assert(target_cwd != NULL);
    assert(target_cwd == target_working_directory);
    size_t target_cwd_length = strnlen(target_cwd, PATH_MAX);
    assert((target_cwd_length > 0) && (target_cwd_length < PATH_MAX));
    assert(strcmp(target_cwd, initial_cwd) != 0);

    // Move back to initial working directory.
    ret = fchdir(initial_directory_fd);
    assert(ret == 0);

    // Get current working directory and assert result.
    char restored_working_directory[PATH_MAX] = {0};
    char *restored_cwd = getcwd(restored_working_directory, sizeof(restored_working_directory));
    assert(restored_cwd != NULL);
    assert(restored_cwd == restored_working_directory);
    size_t restored_cwd_length = strnlen(restored_cwd, PATH_MAX);
    assert((restored_cwd_length > 0) && (restored_cwd_length < PATH_MAX));
    assert(strcmp(restored_cwd, initial_cwd) == 0);

    // Close target directory.
    ret = close(target_directory_fd);
    assert(ret == 0);

    // Close initial directory.
    ret = close(initial_directory_fd);
    assert(ret == 0);

    fprintf(stderr, "passed\n");
}
