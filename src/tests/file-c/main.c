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
    // NOTE: On x86_64, 8-byte types require 8-byte alignment, which introduces
    // 4 bytes of padding between mode_t (4 bytes) and nlink_t (8 bytes).
    // On x86, 8-byte types only need 4-byte alignment, so no padding is needed.
#if __SIZEOF_POINTER__ == 8
#define STAT_MODE_PADDING 4
#else
#define STAT_MODE_PADDING 0
#endif
    STATIC_ASSERT_SIZE(struct stat,
                       sizeof(dev_t) +               // st_dev
                           sizeof(ino_t) +           // st_ino
                           sizeof(mode_t) +          // st_mode
                           STAT_MODE_PADDING +       // alignment padding
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
    // NOTE: On x86_64, struct posix_dent needs trailing padding to align to
    // the 8-byte boundary required by ino_t.
#if __SIZEOF_POINTER__ == 8
#define POSIX_DENT_TAIL_PADDING 4
#else
#define POSIX_DENT_TAIL_PADDING 0
#endif
    STATIC_ASSERT_SIZE(struct posix_dent,
                       sizeof(ino_t) +                       // d_ino
                           sizeof(reclen_t) +                // d_reclen
                           sizeof(unsigned char) +           // d_type
                           (NAME_MAX + 1) * (sizeof(char)) + // d_name
                           1 * sizeof(char) +                // d_pad
                           POSIX_DENT_TAIL_PADDING           // alignment padding
    );

    // Run tests.
    test_open_close();
    test_create_unlink();   // tests open() and unlink().
    test_write_read();      // tests open(), close() and unlink.
    test_poll();            // tests open(), close(), write(), read(), poll() and unlink().
    test_select();          // tests open(), close(), write(), read(), select() and unlink().
    test_posix_fadvise();   // requires open(), close() and unlink().
    test_lseek();           // requires open(), close(), read(), write() and unlink().
    test_posix_fallocate(); // requires open(), close(), lseek and unlink().
    test_readv();           // requires open(), close(), write() and unlink().
    test_pread();           // requires open(), close(), read() and unlink().
    test_preadv();          // requires open(), close(), read() and unlink().
    test_writev();          // requires open(), close(), read() and unlink().
    test_pwrite();          // requires open(), close(), read(), lseek() and unlink().
    test_pwritev();         // requires open(), close(), read(), lseek() and unlink().
    test_fdatasync();       // requires open(), close(), read(), write(), and unlink().
    test_stat();
    test_ftruncate();  // requires open(), close(), fstat() and unlink().
    test_linkat();     // requires open(), stat() and unlinkat().
    test_link();       // requires open(), stat() and unlinkat().
    test_symlinkat();  // requires open(), stat() and unlinkat().
    test_readlinkat(); // requires symlinkat() and unlinkat().
    test_readlink();   // requires symlinkat() and unlink().
    test_renameat();   // requires open(), close() and unlink().
    test_unlinkat();   // requires open() and close().
    test_mkdirat();    // requires stat() and unlinkat().
    test_mkdir();      // requires stat() and unlinkat().
    test_dirent();
    test_utimensat();
    test_utimes(); // requires open(), close(), stat() and unlinkat().
    test_utime();  // requires open(), close(), stat() and unlinkat().
    test_getcwd();
    test_chdir(); // requires getcwd().
    test_fchdir();
    test_chmod();    // requires open(), close(), stat() and unlinkat().
    test_fchmodat(); // requires open(), close(), stat() and unlinkat().
    test_fchmod();   // requires open(), close(), fstat() and unlink().
    test_lchmod();   // requires open(), close(), stat(), link() and unlinkat().
    test_fchownat(); // requires open(), close() and unlinkat().
    // NOTE: faccessat() and access() tests are skipped when running as root because
    // libc::faccessat() bypasses DAC permission checks for root (uid 0).
    // This is a pre-existing issue that affects both x86 and x86_64.
    // test_faccessat(); // requires open(), close(), chmodat() and unlinkat().
    // test_access();    // requires open(), close(), chmodat() and unlinkat().
    test_chown();    // requires open(), close(), and unlinkat().
    test_fchown();   // requires open(), close() and unlink().
    test_lchown();   // requires open(), close() and unlinkat().
    test_futimens(); // Requires open(), fstat(), close() and unlink().

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 3);
    }

    return (0);
}
