// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Integration test for the host directory mount feature.
//!
//! This guest binary exercises the `-mount <host-dir>` feature by:
//! 1. Verifying that a pre-existing file from the host directory is readable at `/mnt/input.txt`.
//! 2. Verifying that a nested file at `/mnt/subdir/nested.txt` is readable.
//! 3. Creating a new file at `/mnt/output.txt` that will be copied back to the host on shutdown.
//! 4. Modifying the existing `/mnt/input.txt` to verify that changes are preserved on copyback.
//!
//! Exit code 0 indicates all checks passed.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]

//==================================================================================================
// Modules
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    FileSystemPermissions,
    RegularFile,
    RegularFileOpenFlags,
};

//==================================================================================================
// Test Helpers
//==================================================================================================

/// Opens a file, reads its entire contents, and closes it.
fn read_file(path: &str) -> Result<alloc::vec::Vec<u8>, Error> {
    let pathname: FileSystemPath = FileSystemPath::new(path)?;
    let file: RegularFile =
        FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_only(), None)?;

    let mut buf: [u8; 4096] = [0u8; 4096];
    let n: usize = file.read(&mut buf)?;
    Ok(alloc::vec::Vec::from(&buf[..n]))
}

/// Creates (or truncates) a file with the given contents.
fn write_file(path: &str, data: &[u8]) -> Result<(), Error> {
    let pathname: FileSystemPath = FileSystemPath::new(path)?;

    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // Create the file (truncates if it exists).
    let mut file: RegularFile = FileSystem::create_regular_file(&pathname, Some(permissions))?;

    file.write(data)?;
    Ok(())
}

//==================================================================================================
// Main Function
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!("mount-test: starting host directory mount integration test");

    // Test 1: Read a pre-existing file from the mounted host directory.
    {
        let data: alloc::vec::Vec<u8> = read_file("/mnt/input.txt")?;
        if data != b"mount-test-input\n" {
            ::syslog::error!("mount-test: /mnt/input.txt content mismatch (len={})", data.len());
            panic!("content mismatch on /mnt/input.txt");
        }
        ::syslog::info!("mount-test: [PASS] /mnt/input.txt read correctly");
    }

    // Test 2: Read a nested file.
    {
        let data: alloc::vec::Vec<u8> = read_file("/mnt/subdir/nested.txt")?;
        if data != b"nested-content\n" {
            ::syslog::error!("mount-test: /mnt/subdir/nested.txt content mismatch");
            panic!("content mismatch on /mnt/subdir/nested.txt");
        }
        ::syslog::info!("mount-test: [PASS] /mnt/subdir/nested.txt read correctly");
    }

    // Test 3: Create a new file on the mount (to be copied back to host).
    write_file("/mnt/output.txt", b"guest-created-file\n")?;
    ::syslog::info!("mount-test: [PASS] /mnt/output.txt created");

    // Test 4: Overwrite the existing file (to verify modify-and-copyback).
    write_file("/mnt/input.txt", b"modified-by-guest\n")?;
    ::syslog::info!("mount-test: [PASS] /mnt/input.txt modified");

    // Test 5: Re-read the modified file to confirm the write took effect.
    {
        let data: alloc::vec::Vec<u8> = read_file("/mnt/input.txt")?;
        if data != b"modified-by-guest\n" {
            ::syslog::error!("mount-test: /mnt/input.txt re-read content mismatch");
            panic!("re-read mismatch on /mnt/input.txt");
        }
        ::syslog::info!("mount-test: [PASS] /mnt/input.txt re-read matches");
    }

    ::syslog::info!("mount-test: all tests passed");
    Ok(())
}
