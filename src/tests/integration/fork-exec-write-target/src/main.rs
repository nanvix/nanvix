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

//! # `execv()` Target That Writes a File Then `_exit()`s Without `close()`
//!
//! Loaded into the guest ramfs at `/target` and `execv()`'d by the child that
//! `fork-exec-write-test` forks. After exec it:
//!
//!   1. creates `/exec_write.out` and `write()`s a known payload to it, then
//!   2. terminates via `_exit()` WITHOUT calling `close()` and without running the normal C runtime
//!      shutdown, so no implicit flush happens.
//!
//! POSIX requires that once a `write()` has returned successfully, the written bytes are visible to
//! any other process that reads the file: `close()` is not required for visibility, and `_exit()`
//! does not discard data that `write()` already committed. The companion caller reads the file back
//! to check this. This mirrors how a `fork()`+`execv()`'d helper (e.g. an interpreter running a
//! script, then terminating) leaves an output file behind for its parent to collect.

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
    fcntl::{
        file_access_mode::O_WRONLY,
        file_creation_flags::{
            O_CREAT,
            O_TRUNC,
        },
    },
    ffi::c_int,
    sys_types::mode_t,
};
use ::syscall::{
    fcntl::open,
    unistd::{
        bindings,
        write,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Path of the file the target creates on the writable guest filesystem. Shared with the caller.
const OUTPUT_PATH: &str = "/exec_write.out";

/// Payload the target writes. Shared with the caller, which reads it back.
const PAYLOAD: &[u8] = b"FORK-EXEC-WRITE-PAYLOAD";

/// Permissions for the created file (rw for the owner).
const FILE_MODE: mode_t = 0o600;

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    // Create the output file and write the payload. Once write() returns, those bytes are -- per
    // POSIX -- visible to other processes reading the file.
    let fd: c_int = open(OUTPUT_PATH, O_WRONLY | O_CREAT | O_TRUNC, FILE_MODE)?;

    let mut remaining: &[u8] = PAYLOAD;
    while !remaining.is_empty() {
        let n: usize = usize::try_from(write(fd, remaining)?)
            .map_err(|_| Error::new(ErrorCode::InvalidArgument, "invalid write length"))?;
        if n == 0 {
            return Err(Error::new(ErrorCode::TryAgain, "short write to /exec_write.out"));
        }
        remaining = &remaining[n..];
    }

    // Terminate WITHOUT close() and WITHOUT the normal C runtime shutdown: data already accepted by
    // write() must remain visible to the parent. A forked+exec'd helper that ends this way (or is
    // killed) must not silently lose its output.
    // SAFETY: the process holds no resources requiring cleanup; terminate immediately.
    unsafe { bindings::_exit::_exit(0) };
}
