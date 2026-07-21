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
#include <stdlib.h>
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Imported Symbols
//==================================================================================================

extern int mount(
    const char *source,
    const char *target,
    const char *filesystemtype,
    unsigned long mountflags);

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can set the file mode creation mask.
void test_umask(void)
{
    fprintf(stderr, "testing umask() ... ");

    const mode_t permissions = S_IRWXU | S_IRWXG | S_IRWXO;
    const mode_t mask = S_IWUSR | S_IRWXG | S_IRWXO;
    const mode_t previous_mask = umask(0);

    // Set the mask and verify that umask() returns its previous normalized value.
    assert(umask(mask) == 0);
    assert(umask(mask | ~permissions) == mask);
    assert(umask(mask) == mask);

    if (getenv("NANVIX_TEST_HOSTFS_PERMISSIONS") != NULL) {
        const mode_t expected_mode = permissions & ~mask;
        const char *filename = "/mnt/umask-file";
        const char *dirname = "/mnt/umask-dir";
        struct stat st = {0};

        // Hostfs exposes Unix permission bits, unlike the standalone FAT32 filesystem.
        assert(mount("", "/mnt", "hostfs", 0) == 0);

        // Verify that the mask is applied when creating a regular file.
        int fd = open(filename, O_CREAT | O_EXCL | O_WRONLY, permissions);
        assert(fd >= 0);
        assert(close(fd) == 0);
        assert(stat(filename, &st) == 0);
        assert(S_ISREG(st.st_mode));
        assert((st.st_mode & permissions) == expected_mode);
        assert(unlink(filename) == 0);

        // Verify that the mask is applied when creating a directory.
        assert(mkdir(dirname, permissions) == 0);
        assert(stat(dirname, &st) == 0);
        assert(S_ISDIR(st.st_mode));
        assert((st.st_mode & permissions) == expected_mode);
        assert(unlinkat(AT_FDCWD, dirname, AT_REMOVEDIR) == 0);
    }

    // Restore the mask for the remaining tests.
    assert(umask(previous_mask) == mask);

    fprintf(stderr, "passed\n");
}
