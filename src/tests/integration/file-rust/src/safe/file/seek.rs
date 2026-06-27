// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    FileSystemPermissions,
    RegularFile,
    RegularFileOffset,
    RegularFileOpenFlags,
    RegularFileSeekWhence,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can manipulate the seek position of a file.
pub fn test() {
    const DATA: &[u8] = b"Hello Nanvix!";
    let filename: &str = "test-seek.txt";

    let pathname: FileSystemPath = match FileSystemPath::new(filename) {
        Ok(pathname) => pathname,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Create file and assert result.
    {
        let _file: RegularFile = match FileSystem::create_regular_file(
            &pathname,
            Some(
                FileSystemPermissions::empty()
                    .user_read(true)
                    .user_write(true),
            ),
        ) {
            Ok(file) => file,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        // File is automatically closed when it goes out of scope.
    }

    // Test file seek by writing and reading data at the beginning.
    {
        // Open file for reading and writing and assert result.
        let mut file: RegularFile = match FileSystem::open_regular_file(
            &pathname,
            &RegularFileOpenFlags::read_write(),
            None,
        ) {
            Ok(file) => file,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        // Write to the beginning of the file and assert result.
        match file.write(DATA) {
            Ok(n) if n == DATA.len() => {},
            Ok(n) => {
                panic!("expected to write {} bytes, but wrote {n} bytes", DATA.len());
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }

        // Rewind to the beginning of the file.
        if let Err(error) = file.seek(RegularFileSeekWhence::Start, RegularFileOffset::from(0)) {
            panic!("{error:?}");
        }

        // Read data back and assert result.
        let mut expected_data: [u8; DATA.len()] = [0; DATA.len()];
        match file.read(&mut expected_data) {
            Ok(n) if n == DATA.len() => {
                assert_eq!(&expected_data[..n], DATA);
            },
            Ok(n) => {
                panic!("expected to read {} bytes, but read {n} bytes", DATA.len());
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }
    }

    // Test file seek by writing and reading data at the end.
    {
        // Open file for reading and writing and assert result.
        let mut file: RegularFile = match FileSystem::open_regular_file(
            &pathname,
            &RegularFileOpenFlags::read_write(),
            None,
        ) {
            Ok(file) => file,
            Err(error) => {
                panic!("{error:?}");
            },
        };

        // Seek to the end of the file.
        if let Err(error) = file.seek(RegularFileSeekWhence::End, RegularFileOffset::from(0)) {
            panic!("{error:?}");
        }

        // Write to the end of the file and assert result.
        match file.write(DATA) {
            Ok(n) if n == DATA.len() => {},
            Ok(n) => {
                panic!("expected to write {} bytes, but wrote {n} bytes", DATA.len());
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }

        // Rewind from the end of the file.
        let offset: RegularFileOffset = match i32::try_from(DATA.len()) {
            Ok(offset) => RegularFileOffset::from(-offset),
            Err(error) => {
                panic!("failed to convert length to offset: {error:?}");
            },
        };
        if let Err(error) = file.seek(RegularFileSeekWhence::End, offset) {
            panic!("{error:?}");
        }

        // Read data back and assert result.
        let mut expected_data: [u8; DATA.len()] = [0; DATA.len()];
        match file.read(&mut expected_data) {
            Ok(n) if n == DATA.len() => {
                assert_eq!(&expected_data[..n], DATA);
            },
            Ok(n) => {
                panic!("expected to read {} bytes, but read {n} bytes", DATA.len());
            },
            Err(error) => {
                panic!("{error:?}");
            },
        }
    }

    // Unlink the file and assert result.
    if let Err(error) = FileSystem::remove_file(&pathname) {
        panic!("{error:?}");
    }
}
