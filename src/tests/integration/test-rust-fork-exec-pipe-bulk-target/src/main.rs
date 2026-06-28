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

//! # `execv()` Target That Writes a Bulk Payload to Standard Output
//!
//! Loaded into the guest ramfs at `/target` and `execv()`'d by the child that
//! `fork-exec-pipe-bulk-test` forks after redirecting standard output onto the write end of a pipe
//! with `dup2()`. This target is deliberately a *large* image: its data segment is inflated to
//! `MEMORY_SIZE / 8` (like `execv-big-target`), because the bulk-pipe defect only reproduces when a
//! *large* image is `execv()`'d -- a small one streams its output fine. After exec it validates that
//! its large segment loaded, `write()`s exactly [`TOTAL_BYTES`] bytes of the constant byte
//! [`PATTERN`] to `STDOUT_FILENO` (now the pipe), looping over partial writes, and then `_exit()`s
//! with `0`.
//!
//! Once each `write()` has returned successfully every byte must be delivered to the reader on the
//! other end of the pipe: POSIX guarantees a pipe transfers the full stream and signals end-of-file
//! only after the last writing descriptor is closed. The companion caller drains the pipe and
//! checks it received all [`TOTAL_BYTES`] bytes intact. This mirrors how a `fork()`+`execv()`'d
//! helper (e.g. an interpreter running a script) streams a large result back to its parent through a
//! captured standard stream.

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

/// Total number of bytes the target writes to standard output. Shared with the caller, which reads
/// them back. 1 MiB is far larger than any pipe buffer, so the writer and reader must stream the
/// data; a correct pipe still delivers every byte.
const TOTAL_BYTES: usize = 1 << 20;

/// The constant byte written `TOTAL_BYTES` times. Shared with the caller, which verifies every byte.
const PATTERN: u8 = 0xA5;

/// Size of the stack buffer used to feed `write()`.
const CHUNK: usize = 4096;

/// Exit status reported by the target if a `write()` to standard output fails or returns zero.
const TARGET_WRITE_FAILED: c_int = 3;

/// Exit status reported by the target if its large data segment did not load (a sanity check that
/// the big image really was read in by `execv()`).
const TARGET_BLOB_NOT_LOADED: c_int = 4;

/// Size of the inflation blob: one eighth of the guest's physical memory. This makes the target's
/// on-disk image large (`MEMORY_SIZE / 8`), so `execv()`ing it exercises the large-binary path --
/// the condition under which a forked+exec'd writer's bulk pipe output is silently truncated (a
/// small exec'd image does not trigger it). Scales with the configured `MEMORY_SIZE`.
const BIG_SIZE: usize = ::config::kernel::MEMORY_SIZE / 8;

/// Sentinel byte stored throughout the blob and checked at runtime to confirm the segment loaded.
const BLOB_FILL: u8 = 0xAB;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Large, non-zero-initialized blob that inflates the target's on-disk image and loadable data
/// segment to [`BIG_SIZE`] bytes. Being non-zero forces it into a file-backed (PROGBITS) section
/// rather than BSS, so the on-disk image actually grows. `#[used]` together with the runtime reads
/// in [`main`] prevents it from being optimized or garbage-collected away.
#[used]
static BIG_BLOB: [u8; BIG_SIZE] = [BLOB_FILL; BIG_SIZE];

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    // Confirm the large data segment was loaded by execv() (first and last byte), so the test
    // genuinely exercises the large-image path before streaming the payload.
    let first: u8 = ::core::hint::black_box(BIG_BLOB[0]);
    let last: u8 = ::core::hint::black_box(BIG_BLOB[BIG_SIZE - 1]);
    if first != BLOB_FILL || last != BLOB_FILL {
        // SAFETY: the process holds no resources requiring cleanup; terminate immediately.
        unsafe { bindings::_exit::_exit(TARGET_BLOB_NOT_LOADED) };
    }

    let buf: [u8; CHUNK] = [PATTERN; CHUNK];

    // Write exactly TOTAL_BYTES, looping over partial writes. Each returned write() must deliver its
    // bytes to the pipe reader; the caller checks the total it received.
    let mut written: usize = 0;
    while written < TOTAL_BYTES {
        let want: usize = core::cmp::min(CHUNK, TOTAL_BYTES - written);
        match write(STDOUT_FILENO, &buf[..want]) {
            Ok(0) | Err(_) => {
                // A short or failed write means the target could not stream its payload; report it
                // so the failure is attributed to the writer rather than to a reader-side shortfall.
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

    // Terminate cleanly: every byte accepted by write() above must remain visible to the parent
    // reading the pipe, regardless of how this process ends.
    // SAFETY: the process holds no resources requiring cleanup; terminate immediately.
    unsafe { bindings::_exit::_exit(0) };
}
