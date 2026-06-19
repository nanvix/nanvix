// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Comprehensive integration test for the host-mounted filesystem (hostfs).
//!
//! This guest binary exercises the entire set of VFS operations over a hostfs-mounted
//! directory at `/mnt`, mirroring the tests in `file-rust` but targeting the host filesystem
//! daemon (hostfsd) backend. The test:
//! 1. Mounts hostfs at `/mnt`.
//! 2. Runs comprehensive file and filesystem tests over `/mnt`.
//! 3. Unmounts and verifies mount lifecycle invariants.
//!
//! This test is valid only for standalone mode (uses `-mount <host-dir>` feature).
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
extern crate nvx_crt0;

mod cross_fs;
mod dirfd_reject;
mod file_ops;
mod fs_ops;
mod mount_lifecycle;
mod readdir_ops;
mod stdio_reuse;
mod symlink_ops;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::syscall::unistd;
use sysapi::unistd::STDOUT_FILENO;

//==================================================================================================
// Main Function
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!("mount-test: starting comprehensive hostfs integration test");

    // Phase 1: Mount lifecycle tests.
    mount_lifecycle::test()?;

    // Phase 2: Filesystem operations (mkdir, unlink, rename).
    fs_ops::test()?;

    // Phase 3: File operations (write/read, seek, fstat, truncate, fsync).
    file_ops::test()?;

    // Phase 3.5: Symbolic link operations (symlink, readlink, lstat).
    symlink_ops::test()?;

    // Phase 3.6: Directory listing operations (getdents/readdir sweep).
    readdir_ops::test()?;

    // Phase 3.7: Invalid-dirfd rejection for hostfs-routed *at() operations.
    dirfd_reject::test()?;

    // Phase 3.8: Standard descriptor close/reuse semantics under the flat namespace.
    stdio_reuse::test()?;

    // Phase 4: Cross-filesystem consistency tests (RAMFS vs HostFS).
    cross_fs::test()?;

    // Phase 5: Final unmount.
    ::syscall::sys::mount::umount("/mnt")?;
    ::syslog::info!("mount-test: final umount succeeded");

    ::syslog::info!("mount-test: all tests passed");

    // Magic string.
    {
        let magic_string: &[u8] = "ok".as_bytes();
        unistd::write(STDOUT_FILENO, magic_string)?;
    }

    Ok(())
}
