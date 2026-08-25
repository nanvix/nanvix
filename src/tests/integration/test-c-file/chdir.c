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
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
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

    // Regression: chdir() onto a regular file must fail with ENOTDIR and leave
    // the cwd unchanged.
    const char *filename = "testfile_chdir";
    int fd = open(filename, O_CREAT | O_WRONLY, S_IRUSR | S_IWUSR);
    assert(fd >= 0);
    assert(close(fd) == 0);
    errno = 0;
    assert(chdir(filename) != 0);
    assert(errno == ENOTDIR);
    assert(getcwd(new_cwd, sizeof(new_cwd)) != NULL);
    assert(strcmp(new_cwd, original_cwd) == 0);
    assert(unlink(filename) == 0);

    // Cross-mount coverage over hostfs (mounted at /mnt by the suite driver): chdir
    // must forward to hostfsd, succeed onto a directory, and reject a file with
    // ENOTDIR.
    if (getenv("NANVIX_TEST_HOSTFS") != NULL) {
        const char *hostdir = "/mnt/testdir_chdir";
        const char *hostfile = "/mnt/testfile_chdir";

        // chdir() onto a hostfs directory succeeds and updates the cwd.
        assert(mkdir(hostdir, S_IRUSR | S_IWUSR | S_IXUSR) == 0);
        assert(chdir(hostdir) == 0);
        assert(getcwd(new_cwd, sizeof(new_cwd)) != NULL);
        assert(strcmp(new_cwd, hostdir) == 0);
        assert(chdir(original_cwd) == 0);
        assert(unlinkat(AT_FDCWD, hostdir, AT_REMOVEDIR) == 0);

        // chdir() onto a hostfs file fails with ENOTDIR and leaves the cwd intact.
        fd = open(hostfile, O_CREAT | O_WRONLY, S_IRUSR | S_IWUSR);
        assert(fd >= 0);
        assert(close(fd) == 0);
        errno = 0;
        assert(chdir(hostfile) != 0);
        assert(errno == ENOTDIR);
        assert(getcwd(new_cwd, sizeof(new_cwd)) != NULL);
        assert(strcmp(new_cwd, original_cwd) == 0);
        assert(unlink(hostfile) == 0);
    }

    // Clean up.
    assert(unlinkat(AT_FDCWD, dirname, AT_REMOVEDIR) == 0);

    fprintf(stderr, "passed\n");
}
