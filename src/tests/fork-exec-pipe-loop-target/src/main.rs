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

//! # `execv()` Target That Echoes a Fixed Payload to Standard Output (loop helper)
//!
//! Loaded into the guest ramfs at `/target` and `execv()`'d, once per cycle, by the child that
//! `fork-exec-pipe-loop-test` forks after redirecting standard output onto the write end of a pipe
//! with `dup2()`. This target is deliberately a *large* image: its data segment is inflated to
//! `MEMORY_SIZE / 8` (like `execv-big-target`), because the fork()+execv()+pipe reliability defect
//! only reproduces when a *large* image is `execv()`'d -- a small one round-trips fine.
//!
//! After exec it validates that its large segment loaded, `write()`s exactly [`PAYLOAD_BYTES`] bytes
//! of the constant byte [`PATTERN`] to `STDOUT_FILENO` (now the pipe), looping over partial writes,
//! and then `_exit()`s with `0`. The caller repeats this many times and requires every cycle to
//! deliver the full payload.

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
    ffi::c_int,
    unistd::STDOUT_FILENO,
};
use ::syscall::unistd::{
    bindings,
    write,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of bytes the target writes to standard output each time it is exec'd. Shared with the
/// caller, which reads them back every cycle. 64 KiB exceeds the pipe buffer, so it must be streamed.
const PAYLOAD_BYTES: usize = 64 * 1024;

/// The constant byte written `PAYLOAD_BYTES` times. Shared with the caller, which verifies every byte.
const PATTERN: u8 = 0x5A;

/// Size of the stack buffer used to feed `write()`.
const CHUNK: usize = 4096;

/// Exit status reported by the target if a `write()` to standard output fails or returns zero.
const TARGET_WRITE_FAILED: c_int = 3;

/// Exit status reported by the target if its large data segment did not load.
const TARGET_BLOB_NOT_LOADED: c_int = 4;

/// Size of the inflation blob: one eighth of the guest's physical memory, so `execv()`ing this
/// target exercises the large-binary path -- the condition under which a forked+exec'd writer's pipe
/// output is unreliable (a small exec'd image does not trigger it). Scales with `MEMORY_SIZE`.
const BIG_SIZE: usize = ::config::kernel::MEMORY_SIZE / 8;

/// Sentinel byte stored throughout the blob and checked at runtime to confirm the segment loaded.
const BLOB_FILL: u8 = 0xAB;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Large, non-zero-initialized blob that inflates the target's on-disk image and loadable data
/// segment to [`BIG_SIZE`] bytes. `#[used]` plus the runtime reads in [`main`] keep it from being
/// optimized away.
#[used]
static BIG_BLOB: [u8; BIG_SIZE] = [BLOB_FILL; BIG_SIZE];

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    // Confirm the large data segment was loaded by execv() before streaming the payload.
    let first: u8 = ::core::hint::black_box(BIG_BLOB[0]);
    let last: u8 = ::core::hint::black_box(BIG_BLOB[BIG_SIZE - 1]);
    if first != BLOB_FILL || last != BLOB_FILL {
        // SAFETY: the process holds no resources requiring cleanup; terminate immediately.
        unsafe { bindings::_exit::_exit(TARGET_BLOB_NOT_LOADED) };
    }

    let buf: [u8; CHUNK] = [PATTERN; CHUNK];

    let mut written: usize = 0;
    while written < PAYLOAD_BYTES {
        let want: usize = core::cmp::min(CHUNK, PAYLOAD_BYTES - written);
        match write(STDOUT_FILENO, &buf[..want]) {
            Ok(0) | Err(_) => {
                // SAFETY: the process holds no resources requiring cleanup; terminate immediately.
                unsafe { bindings::_exit::_exit(TARGET_WRITE_FAILED) };
            },
            Ok(count) => {
                let n: usize = usize::try_from(count)
                    .map_err(|_| Error::new(ErrorCode::InvalidArgument, "invalid write length"))?;
                written += n;
            },
        }
    }

    // SAFETY: the process holds no resources requiring cleanup; terminate immediately.
    unsafe { bindings::_exit::_exit(0) };
}
