// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    self,
    FileSystem,
    FileSystemPath,
    FileSystemPermissions,
    RegularFile,
    RegularFileOpenFlags,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can synchronize file data to disk using `fdatasync()`.
pub fn test() {
    const DATA: &[u8] = b"Hello Nanvix!";
    let filename: &str = "test-fdatasync.tmp";

    let pathname: FileSystemPath = match FileSystemPath::new(filename) {
        Ok(pathname) => pathname,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // Create file, write data, and synchronize.
    {
        let mut file: RegularFile = match FileSystem::open_regular_file(
            &pathname,
            &RegularFileOpenFlags::read_write()
                .set_create(true)
                .set_truncate(true),
            Some(permissions),
        ) {
            Ok(file) => file,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        // Write data to the file.
        match file.write(DATA) {
            Ok(n) if n == DATA.len() => {},
            Ok(n) => {
                panic!("expected to write {} bytes, but wrote {n} bytes", DATA.len());
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }

        // Synchronize file data to disk.
        if let Err(error) = safe::file::fdatasync(file.as_raw_fd()) {
            panic!("{error:?}");
        }

        // File is automatically closed when it goes out of scope.
    }

    // Reopen the file for reading and verify the data.
    {
        let file: RegularFile = match FileSystem::open_regular_file(
            &pathname,
            &RegularFileOpenFlags::read_only(),
            None,
        ) {
            Ok(file) => file,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        let mut buffer: [u8; DATA.len()] = [0; DATA.len()];
        match file.read(&mut buffer) {
            Ok(n) if n == DATA.len() => {
                assert_eq!(&buffer[..DATA.len()], DATA);
            },
            Ok(n) => {
                panic!("expected to read {} bytes, but read {n} bytes", DATA.len());
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }

        // File is automatically closed when it goes out of scope.
    }

    // Remove the test file.
    if let Err(error) = FileSystem::remove_file(&pathname) {
        panic!("{error:?}");
    }
}
