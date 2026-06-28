// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod dir;
mod file;
mod fs;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Run tests on the unsafe interface.
pub fn test() {
    // The following tests have no dependencies on previous tests.
    fs::getcwd::test();
    fs::fstatat::test();
    fs::stat::test();
    fs::open_close::test();
    fs::opendir_closedir::test();

    // The following tests depend on `getcwd()`.
    fs::chdir::test();

    // The following test depends on `opendir()` and `closedir()`.
    dir::readdir::test();

    // The following tests depend on `open()` and `close()`.
    file::fstat::test();
    file::fadvise::test();
    file::isatty::test();

    // The following tests depend on `close()` and `stat()`.
    fs::open_unlink::test();
    fs::creat_unlink::test();

    // The following tests depend on `creat()`, `close()`, `unlink()` and `stat()`.
    file::fallocate::test();
    file::write_read::test();

    // The following tests depend on `creat()`, `close()`, `unlink()`, `stat()` and `write()`.
    file::fsync::test();

    // The following tests depend on `creat()`, `close()`, `unlink()`, `write()` and `read()`.
    file::lseek::test();

    // The following tests depend on `creat()`, `close()`, unlink()`, `write()`, `read()` and
    // `lseek()`.
    file::pwrite_pread::test();

    // The following tests depend on `openat()`, `unlinkat()`, `renameat()` and `mkdir()`.
    fs::unlinkat_dirfd::test();
    fs::renameat_dirfd::test();

    // The following test exercises open/close with long relative paths through the VFS.
    // Hostfs multi-part messaging is tested in mount-test with /mnt-prefixed paths.
    fs::open_close_long_path::test();

    // TODO: Add unsafe test mirrors for the following C test files:
    // - chmod.c, fchmod.c, fchmodat.c (permission changes)
    // - chown.c, fchown.c, fchownat.c, lchown.c (ownership changes)
    // - link.c, linkat.c (hard links)
    // - symlinkat.c, readlink.c, readlinkat.c (symbolic links)
    // - access.c, faccessat.c (access permission checks)
    // - mkdir.c, mkdirat.c (directory creation)
    // - futimens.c, utimensat.c (timestamp updates)
    // - fdatasync.c (data synchronization)
    // - ftruncate.c (file truncation)
    // - poll.c (I/O multiplexing via poll)
    // - select.c (I/O multiplexing via select)
    // - readv.c, writev.c, preadv.c, pwritev.c (vectored I/O — no wrappers yet)
    // - utime.c, utimes.c (legacy timestamp updates — no wrappers yet)
    // - lchmod.c (change symlink permissions — no wrapper yet)
}
