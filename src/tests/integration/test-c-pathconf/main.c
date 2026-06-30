/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <errno.h>
#include <limits.h>
#include <stddef.h>
#include <string.h>
#include <unistd.h>

//==================================================================================================
// Private Functions
//==================================================================================================

// Tests that pathconf() returns the configured limit for selectors with a determinate value, and
// leaves errno untouched. The values must match <limits.h> (PATH_MAX, NAME_MAX).
static void test_pathconf_known_limits(void)
{
    errno = 0;
    assert(pathconf("/", _PC_PATH_MAX) == PATH_MAX);
    assert(pathconf("/", _PC_NAME_MAX) == NAME_MAX);
    assert(pathconf("/", _PC_SYMLINK_MAX) == PATH_MAX);
    assert(pathconf("/", _PC_LINK_MAX) == 32767);
    assert(pathconf("/", _PC_MAX_CANON) == 255);
    assert(pathconf("/", _PC_MAX_INPUT) == 255);
    assert(pathconf("/", _PC_PIPE_BUF) == 4096);
    assert(pathconf("/", _PC_FILESIZEBITS) == 64);
    assert(pathconf("/", _PC_CHOWN_RESTRICTED) == 1);
    assert(pathconf("/", _PC_NO_TRUNC) == 1);
    assert(pathconf("/", _PC_2_SYMLINKS) == 1);
    assert(pathconf("/", _PC_VDISABLE) == 0);
    assert(errno == 0);
}

// Tests that fpathconf() behaves identically to pathconf() (the limits are system-global, so the
// descriptor is not consulted).
static void test_fpathconf_matches_pathconf(void)
{
    errno = 0;
    assert(fpathconf(STDOUT_FILENO, _PC_PATH_MAX) == PATH_MAX);
    assert(fpathconf(STDOUT_FILENO, _PC_NAME_MAX) == NAME_MAX);
    assert(fpathconf(STDIN_FILENO, _PC_NAME_MAX) == NAME_MAX);
    assert(errno == 0);
}

// Tests that selectors with no determinate limit return -1 WITHOUT modifying errno, so callers
// (e.g. libc++'s <filesystem>) take the "no limit -> fall back" branch rather than reporting an
// error. A nonzero sentinel is installed in errno first to prove it is left untouched (not merely
// observed as zero).
static void test_no_limit_selectors_leave_errno(void)
{
    const int no_limit_selectors[] = {
        _PC_SYNC_IO,
        _PC_ASYNC_IO,
        _PC_PRIO_IO,
        _PC_ALLOC_SIZE_MIN,
        _PC_REC_INCR_XFER_SIZE,
        _PC_REC_MAX_XFER_SIZE,
        _PC_REC_MIN_XFER_SIZE,
        _PC_REC_XFER_ALIGN,
    };
    const int sentinel = 0x5A5A;

    for (size_t i = 0; i < sizeof(no_limit_selectors) / sizeof(no_limit_selectors[0]); i++) {
        errno = sentinel;
        assert(pathconf("/", no_limit_selectors[i]) == -1);
        assert(errno == sentinel);

        errno = sentinel;
        assert(fpathconf(STDOUT_FILENO, no_limit_selectors[i]) == -1);
        assert(errno == sentinel);
    }
}

// Tests that an unrecognized selector returns -1 and sets errno to EINVAL (per POSIX), not ENOSYS.
static void test_unknown_selector_sets_einval(void)
{
    errno = 0;
    assert(pathconf("/", 9999) == -1);
    assert(errno == EINVAL);

    errno = 0;
    assert(fpathconf(STDOUT_FILENO, -1) == -1);
    assert(errno == EINVAL);
}

// Tests that pathconf() rejects a NULL path with -1 and errno = EFAULT (per POSIX, an invalid
// pointer must not be treated as success).
static void test_pathconf_null_path_sets_efault(void)
{
    errno = 0;
    assert(pathconf(NULL, _PC_PATH_MAX) == -1);
    assert(errno == EFAULT);
}

// Tests that fpathconf() rejects an invalid (negative) descriptor with -1 and errno = EBADF (per
// POSIX, an invalid file descriptor must not be treated as success).
static void test_fpathconf_bad_fd_sets_ebadf(void)
{
    errno = 0;
    assert(fpathconf(-1, _PC_PATH_MAX) == -1);
    assert(errno == EBADF);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests pathconf()/fpathconf() and the `_PC_*` selector constants.
 *
 * @param argc Number of command-line arguments.
 * @param argv List of command-line arguments.
 *
 * @returns Always returns zero. If a test fails, the program will abort.
 */
int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    // Assert command-line arguments.
    assert(argc == 1);
    assert(argv[0] != NULL);
    assert(argv[1] == NULL);
    assert(strcmp(argv[0], "test-c-pathconf.elf") == 0);

    test_pathconf_known_limits();
    test_fpathconf_matches_pathconf();
    test_no_limit_selectors_leave_errno();
    test_unknown_selector_sets_einval();
    test_pathconf_null_path_sets_efault();
    test_fpathconf_bad_fd_sets_ebadf();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
