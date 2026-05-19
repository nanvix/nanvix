// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! File operation tests over hostfs: write/read, seek, fstat, ftruncate, fsync.

use ::sys::error::Error;
use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    FileSystemPermissions,
    RegularFile,
    RegularFileOffset,
    RegularFileOpenFlags,
    RegularFileSeekWhence,
};

pub fn test() -> Result<(), Error> {
    test_write_read()?;
    test_seek()?;
    test_fstat()?;
    test_ftruncate()?;
    test_fsync()?;
    Ok(())
}

/// Tests basic write and read on hostfs.
fn test_write_read() -> Result<(), Error> {
    const DATA: &[u8] = b"Hello Nanvix HostFS!";
    let pathname: FileSystemPath = FileSystemPath::new("/mnt/test-write-read.txt")?;
    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // Create and write.
    {
        let mut file: RegularFile = FileSystem::create_regular_file(&pathname, Some(permissions))?;
        let n: usize = file.write(DATA)?;
        if n != DATA.len() {
            panic!("write returned wrong byte count: {} vs {}", n, DATA.len());
        }
    }

    // Open for read and verify contents.
    {
        let file: RegularFile =
            FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_only(), None)?;
        let mut buf: [u8; 64] = [0u8; 64];
        let n: usize = file.read(&mut buf)?;
        if n != DATA.len() {
            panic!("read returned wrong byte count: {} vs {}", n, DATA.len());
        }
        if &buf[..n] != DATA {
            panic!("write/read content mismatch");
        }
    }
    ::syslog::info!("mount-test: [PASS] write/read");

    // Cleanup.
    ::syscall::safe::fs::unlink(&pathname)?;

    Ok(())
}

/// Tests seek operations on hostfs.
fn test_seek() -> Result<(), Error> {
    const DATA: &[u8] = b"SeekTestData";
    let pathname: FileSystemPath = FileSystemPath::new("/mnt/test-seek.txt")?;
    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // Create file with known content.
    {
        let mut file: RegularFile = FileSystem::create_regular_file(&pathname, Some(permissions))?;
        file.write(DATA)?;
    }

    // Test seek from Start: write, rewind, read back.
    {
        let mut file: RegularFile =
            FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_write(), None)?;

        // Read to advance position.
        let mut buf: [u8; 64] = [0u8; 64];
        let n: usize = file.read(&mut buf)?;
        if n != DATA.len() {
            panic!("initial read returned wrong count");
        }

        // Seek back to start.
        file.seek(RegularFileSeekWhence::Start, RegularFileOffset::from(0))?;

        // Read again — should get same content.
        let mut buf2: [u8; 64] = [0u8; 64];
        let n2: usize = file.read(&mut buf2)?;
        if &buf2[..n2] != DATA {
            panic!("seek-to-start read mismatch");
        }
    }
    ::syslog::info!("mount-test: [PASS] seek from start");

    // Test seek from End.
    {
        let mut file: RegularFile =
            FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_write(), None)?;

        // Seek to 4 bytes before end.
        let offset: RegularFileOffset = RegularFileOffset::from(-4);
        file.seek(RegularFileSeekWhence::End, offset)?;

        // Read should return last 4 bytes.
        let mut buf: [u8; 4] = [0u8; 4];
        let n: usize = file.read(&mut buf)?;
        if n != 4 {
            panic!("seek-from-end read wrong count: {}", n);
        }
        if buf[..] != DATA[DATA.len() - 4..] {
            panic!("seek-from-end content mismatch");
        }
    }
    ::syslog::info!("mount-test: [PASS] seek from end");

    // Cleanup.
    ::syscall::safe::fs::unlink(&pathname)?;

    Ok(())
}

/// Tests fstat on an opened hostfs file descriptor.
fn test_fstat() -> Result<(), Error> {
    const DATA: &[u8] = b"fstat-test-content";
    let pathname: FileSystemPath = FileSystemPath::new("/mnt/test-fstat.txt")?;
    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // Create file with known content.
    {
        let mut file: RegularFile = FileSystem::create_regular_file(&pathname, Some(permissions))?;
        file.write(DATA)?;
    }

    // Open and check attributes via fstat.
    {
        let file: RegularFile =
            FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_only(), None)?;
        let attrs = file.attributes()?;

        // Verify size matches what we wrote.
        let size: i64 = attrs.size().into();
        if size != DATA.len() as i64 {
            panic!("fstat size mismatch: {} vs {}", size, DATA.len());
        }
    }
    ::syslog::info!("mount-test: [PASS] fstat size correct");

    // Cleanup.
    ::syscall::safe::fs::unlink(&pathname)?;

    Ok(())
}

/// Tests ftruncate on hostfs.
fn test_ftruncate() -> Result<(), Error> {
    const DATA: &[u8] = b"truncate-this-content";
    let pathname: FileSystemPath = FileSystemPath::new("/mnt/test-truncate.txt")?;
    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // Create file with known content.
    {
        let mut file: RegularFile = FileSystem::create_regular_file(&pathname, Some(permissions))?;
        file.write(DATA)?;
    }

    // Open for write and truncate to 5 bytes.
    {
        let file: RegularFile =
            FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::write_only(), None)?;
        let fd: i32 = file.as_raw_fd();
        ::syscall::unistd::ftruncate(fd, 5)?;
    }

    // Read back and verify truncation.
    {
        let file: RegularFile =
            FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_only(), None)?;
        let mut buf: [u8; 64] = [0u8; 64];
        let n: usize = file.read(&mut buf)?;
        if n != 5 {
            panic!("ftruncate: expected 5 bytes, got {}", n);
        }
        if buf[..5] != DATA[..5] {
            panic!("ftruncate content mismatch");
        }
    }
    ::syslog::info!("mount-test: [PASS] ftruncate");

    // Cleanup.
    ::syscall::safe::fs::unlink(&pathname)?;

    Ok(())
}

/// Tests fsync on hostfs.
fn test_fsync() -> Result<(), Error> {
    const DATA: &[u8] = b"fsync-data";
    let pathname: FileSystemPath = FileSystemPath::new("/mnt/test-fsync.txt")?;
    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // Create, write, sync, then verify by re-reading.
    {
        let mut file: RegularFile = FileSystem::create_regular_file(&pathname, Some(permissions))?;
        file.write(DATA)?;
        file.synchronize()?;
    }

    // Read back to verify.
    {
        let file: RegularFile =
            FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_only(), None)?;
        let mut buf: [u8; 64] = [0u8; 64];
        let n: usize = file.read(&mut buf)?;
        if &buf[..n] != DATA {
            panic!("fsync read-back content mismatch");
        }
    }
    ::syslog::info!("mount-test: [PASS] fsync");

    // Cleanup.
    ::syscall::safe::fs::unlink(&pathname)?;

    Ok(())
}
