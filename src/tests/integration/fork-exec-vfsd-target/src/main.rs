// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]
#![deny(clippy::all)]
#![deny(clippy::as_conversions)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

//! # `execv()` Target Performing a vfsd Read
//!
//! Loaded into the guest ramfs at `/target` and `execv()`'d by the child that `fork-exec-vfsd-test`
//! forks. After exec it performs a filesystem read through vfsd: it opens its own ELF image at
//! `/target` (guaranteed present in the ramfs) and reads the ELF magic.
//!
//! This is the operation that hangs when the process was reached via `fork()` + `execv()` (the vfsd
//! rendezvous is keyed by the post-exec thread identifier). On a correct system the read returns and
//! the target exits 0, which the caller observes via `waitpid()`.

//==================================================================================================
// Modules
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    fcntl::file_access_mode::O_RDONLY,
    sys_types::mode_t,
};
use ::syscall::{
    fcntl::open,
    unistd::{
        close,
        read,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Path of this program in the mounted ramfs.
const SELF_PATH: &str = "/target";

/// ELF magic expected at the start of the image.
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    // The vfsd filesystem read that hangs after fork() + execv().
    let mode: mode_t = 0;
    let fd: i32 = open(SELF_PATH, O_RDONLY, mode)?;

    let mut magic: [u8; 4] = [0u8; 4];
    let n: usize = usize::try_from(read(fd, &mut magic)?)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "invalid read length"))?;
    close(fd)?;

    if n != magic.len() || magic != ELF_MAGIC {
        return Err(Error::new(ErrorCode::InvalidArgument, "unexpected /target contents"));
    }

    ::syslog::info!("fork-exec-vfsd-target: vfsd read after exec succeeded");
    Ok(())
}
