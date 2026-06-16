// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Mount lifecycle tests: mount, double-mount, umount, double-umount, re-mount.

use ::sys::error::Error;
use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    FileSystemPermissions,
    RegularFile,
    RegularFileOpenFlags,
};

pub fn test() -> Result<(), Error> {
    // Test 1: Mount hostfs at /mnt.
    {
        ::syscall::sys::mount::mount("", "/mnt", "hostfs", 0)?;
        ::syslog::info!("mount-test: [PASS] mount succeeded");
    }

    // Test 2: Verify double-mount fails.
    {
        let result = ::syscall::sys::mount::mount("", "/mnt", "hostfs", 0);
        if result.is_ok() {
            panic!("double mount should have failed");
        }
        ::syslog::info!("mount-test: [PASS] double mount rejected");
    }

    // Test 3: Basic write and read-back to verify mount is functional.
    {
        let pathname: FileSystemPath = FileSystemPath::new("/mnt/lifecycle.txt")?;
        let permissions: FileSystemPermissions = FileSystemPermissions::empty()
            .user_read(true)
            .user_write(true);

        let mut file: RegularFile = FileSystem::create_regular_file(&pathname, Some(permissions))?;
        file.write(b"lifecycle-test\n")?;
        drop(file);

        let file: RegularFile =
            FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_only(), None)?;
        let mut buf: [u8; 64] = [0u8; 64];
        let n: usize = file.read(&mut buf)?;
        if &buf[..n] != b"lifecycle-test\n" {
            panic!("lifecycle.txt content mismatch");
        }
        ::syslog::info!("mount-test: [PASS] basic write/read after mount");
    }

    // Test 4: Unmount.
    {
        ::syscall::sys::mount::umount("/mnt")?;
        ::syslog::info!("mount-test: [PASS] umount succeeded");
    }

    // Test 5: Verify double-umount fails.
    {
        let result = ::syscall::sys::mount::umount("/mnt");
        if result.is_ok() {
            panic!("double umount should have failed");
        }
        ::syslog::info!("mount-test: [PASS] double umount rejected");
    }

    // Test 6: Re-mount to confirm mount point can be reused.
    {
        ::syscall::sys::mount::mount("", "/mnt", "hostfs", 0)?;

        let pathname: FileSystemPath = FileSystemPath::new("/mnt/lifecycle.txt")?;
        let file: RegularFile =
            FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_only(), None)?;
        let mut buf: [u8; 64] = [0u8; 64];
        let n: usize = file.read(&mut buf)?;
        if &buf[..n] != b"lifecycle-test\n" {
            panic!("content mismatch after re-mount");
        }
        ::syslog::info!("mount-test: [PASS] re-mount and access succeeded");
    }

    // Leave mounted for subsequent tests.
    Ok(())
}
