// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

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

    // The following tests depend on `getcwd()`.
    fs::chdir::test();

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
}
