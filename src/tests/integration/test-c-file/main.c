/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <dirent.h>
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
    //
    // The exact byte size is an i386-ABI layout lock: on x86_64 the 64-bit
    // members carry 8-byte alignment, so `struct stat` gains padding the member
    // sum below does not model. Gate the lock to i386; the functional tests run
    // on every target.
#if defined(__i386__)
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
#endif /* __i386__ */

    // Assert types in <dirent.h>.
    STATIC_ASSERT_SIZE(struct dirent,
                       sizeof(ino_t) +                     // d_ino
                           (NAME_MAX + 1) * (sizeof(char)) // d_name
    );
    // i386-ABI layout lock (see the note on `struct stat` above): `struct
    // posix_dent`'s `d_reclen`/`d_type`/`d_pad` members push the trailing
    // `d_name` off the 8-byte boundary `d_ino` forces on x86_64, so the struct
    // gains padding the member sum does not model. (`struct dirent` above stays
    // exact on every ABI: its `d_name` already lands on an 8-byte boundary.)
#if defined(__i386__)
    STATIC_ASSERT_SIZE(struct posix_dent,
                       sizeof(ino_t) +                       // d_ino
                           sizeof(reclen_t) +                // d_reclen
                           sizeof(unsigned char) +           // d_type
                           (NAME_MAX + 1) * (sizeof(char)) + // d_name
                           1 * sizeof(char)                  // d_pad
    );
#endif /* __i386__ */

    // Run tests.
    test_open_close();
    test_create_unlink(); // tests open() and unlink().
    test_write_read();    // tests open(), close() and unlink.
    test_poll();          // tests open(), close(), write(), read(), poll() and unlink().
    test_posix_fadvise();   // requires open(), close() and unlink().
    test_lseek();           // requires open(), close(), read(), write() and unlink().
    test_posix_fallocate(); // requires open(), close(), lseek and unlink().
    test_readv();           // requires open(), close(), write() and unlink().
    test_preadv();          // requires open(), close(), read() and unlink().
    test_writev();          // requires open(), close(), read() and unlink().
    test_pwritev();         // requires open(), close(), read(), lseek() and unlink().
    test_pread();           // requires open(), close(), read() and unlink().
    test_pwrite();          // requires open(), close(), read(), lseek() and unlink().
    test_fdatasync();       // requires open(), close(), read(), write(), and unlink().
    test_stat();
    test_ftruncate(); // requires open(), close(), fstat() and unlink().
    test_truncate();  // requires open(), close(), stat() and unlinkat().
    test_renameat();   // requires open(), close() and unlink().
    test_renameat_subdir(); // subdir rename + replace-existing.
    test_unlinkat();   // requires open() and close().
    test_mkdirat();    // requires stat() and unlinkat().
    test_mkdir();      // requires stat() and unlinkat().
    test_path_errno(); // path edge case errno values.
    test_mkfifo();     // mkfifo() is unsupported; verifies it fails with ENOTSUP.
    test_mknod();      // mknod() is unsupported; verifies it fails with ENOTSUP.
    test_umask_ramfs(); // tests umask(), open(), close(), stat(), and unlink() on RAMFS.
    if (getenv("NANVIX_TEST_HOSTFS") != NULL) {
        assert(mount("", "/mnt", "hostfs", 0) == 0);
    }
    test_umask();       // tests umask(), open(), stat(), mkdir(), and unlinkat().
    test_poll_hostfs(); // tests poll() with a hostfs descriptor.
    test_dirent();
    test_getcwd();
    test_chdir(); // requires getcwd().
    test_fchdir();
    test_utimensat();
    test_utimensat_now();
    test_utime_preserve_imported_timestamps();

    // Run tests that require Unix file-system semantics on hostfs.
    if (getenv("NANVIX_TEST_HOSTFS") != NULL) {
        assert(chdir("/mnt") == 0);
        test_select();
        test_linkat();
        test_link();
        test_symlinkat();
        test_readlinkat();
        test_readlink();
        test_utimensat();
        test_utimes();
        test_utime();
        test_chmod();
        test_fchmodat();
        test_fchmod();
        test_lchmod();
        test_fchownat();
        test_faccessat();
        test_access();
        test_chown();
        test_fchown();
        test_lchown();
        test_futimens();
    }

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
