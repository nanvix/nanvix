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

/// Run tests on the safe interface.
pub fn test() {
    // The following tests have no dependencies on previous tests.
    fs::get_current_directory::test();
    fs::get_file_attributes::test();
    fs::open_close::test();
    fs::opendir_closedir::test();
    fs::stat::test();

    // The following tests depend on `FileSystem::get_current_directory()`.
    fs::change_current_directory::test();

    // The following test depends on `FileSystem::open_directory()` and `Directory::next()`.
    dir::readdir::test();

    // The following tests depend on `FileSystem::open_regular_file()` and `RegularFile::drop()`.
    file::attributes::test();
    file::advise::test();
    file::is_a_terminal::test();

    // The following tests depend on `RegularFile::drop()` and `FileSystem::get_file_attributes()`.
    fs::open_unlink::test();
    fs::create_remove::test();

    // The following tests depend on `FileSystem::create_regular_file()`, `RegularFile::drop()`,
    // `FileSystem::remove_file()` and `FileSystem::get_file_attributes()`.
    file::allocate::test();
    file::write_read::test();

    // The following tests depend on `FileSystem::create_regular_file()`, `RegularFile::drop()`,
    // `FileSystem::remove_file()`, `FileSystem::get_file_attributes()` and `RegularFile::write()`.
    file::sync::test();

    // The following tests depend on `FileSystem::create_regular_file()`, `RegularFile::drop()`,
    // `FileSystem::remove_file()`, `RegularFile::write()` and `RegularFile::read()`.
    file::seek::test();

    // The following tests depend on `FileSystem::create_regular_file()`, `RegularFile::drop()`,
    // `FileSystem::remove_file()`, `FileSystem::get_file_attributes()`,
    // `RegularFile::write()` and `RegularFile::read()`.
    file::chmod::test();
    file::datasync::test();

    // The following tests depend on `FileSystem::get_file_attributes()` and
    // `FileSystem::remove_file()`.
    fs::chmod::test();
    fs::rename::test();
    fs::mkdir::test();
}
