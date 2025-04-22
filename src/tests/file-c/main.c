/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Macros
//==================================================================================================

/**
 * @brief Performs a static assertion.
 *
 * @param a Expression to assert.
 * @param b Expected value.
 *
 * @returns Nothing. If the assertion fails, compilation will fail.
 */
#define STATIC_ASSERT(a, b) ((void)sizeof(char[(((a) == (b)) ? 1 : -1)]))

/**
 * @brief Performs a static assertion on the size of a type.
 *
 * @param a Type to assert.
 * @param b Expected size.
 *
 * @returns Nothing. If the assertion fails, compilation will fail.
 */
#define STATIC_ASSERT_SIZE(a, b) STATIC_ASSERT(sizeof(a), b)

/**
 * @brief Performs a static assertion on the alignment of a type.
 *
 * @param a Type to assert.
 * @param b Expected alignment.
 *
 * @returns Nothing. If the assertion fails, compilation will fail.
 */
#define STATIC_ASSERT_ALIGNMENT(a, b) STATIC_ASSERT(_Alignof(a), b)

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests file system system calls.
 *
 * @param argc Number of command-line arguments (unused).
 * @param argv List of command-line arguments (unused).
 *
 * @returns Always returns zero. If a test fails, the program will abort.
 */
int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    // Assert types in <sys/stat.h>.
    STATIC_ASSERT_SIZE(struct stat,
                       sizeof(dev_t) +               // st_dev
                           sizeof(ino_t) +           // st_ino
                           sizeof(mode_t) +          // st_mode
                           sizeof(nlink_t) +         // st_nlink
                           sizeof(uid_t) +           // st_uid
                           sizeof(gid_t) +           // st_gid
                           sizeof(dev_t) +           // st_rdev
                           sizeof(off_t) +           // st_size
                           sizeof(struct timespec) + // st_atim
                           sizeof(struct timespec) + // st_mtim
                           sizeof(struct timespec) + // st_ctim
                           sizeof(blksize_t) +       // st_blksize
                           sizeof(blkcnt_t)          // st_blocks
    );

    // Assert types in <dirent.h>.
    STATIC_ASSERT_SIZE(struct dirent,
                       sizeof(ino_t) +                     // d_ino
                           (NAME_MAX + 1) * (sizeof(char)) // d_name
    );
    STATIC_ASSERT_SIZE(struct posix_dent,
                       sizeof(ino_t) +                       // d_ino
                           sizeof(reclen_t) +                // d_reclen
                           sizeof(unsigned char) +           // d_type
                           (NAME_MAX + 1) * (sizeof(char)) + // d_name
                           1 * sizeof(char)                  // d_pad
    );

    // Run tests.
    test_open_close();
    test_create_unlink();
    test_dirent();
    test_getcwd();
    test_fchdir();
    test_fchmod();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 3);
    }

    return (0);
}
