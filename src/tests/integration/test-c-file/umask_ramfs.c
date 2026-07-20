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
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can create a RAMFS file while a file mode creation mask is active.
void test_umask_ramfs(void)
{
    fprintf(stderr, "testing umask() on RAMFS ... ");

    const char *filename = "/umask-ramfs-file";
    const mode_t permissions = S_IRWXU | S_IRWXG | S_IRWXO;
    const mode_t mask = S_IRWXG | S_IRWXO;
    const mode_t previous_mask = umask(mask);

    // RAMFS does not preserve POSIX permissions, but file creation must succeed with an active mask.
    int fd = open(filename, O_CREAT | O_EXCL | O_WRONLY, permissions);
    assert(fd >= 0);
    assert(close(fd) == 0);

    struct stat st = {0};
    assert(stat(filename, &st) == 0);
    assert(S_ISREG(st.st_mode));
    assert(unlink(filename) == 0);

    // Restore the previous mask and verify that the mask remained active during file creation.
    assert(umask(previous_mask) == mask);

    fprintf(stderr, "passed\n");
}
