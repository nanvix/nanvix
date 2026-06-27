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

//! # `execv()` Target That Reads From an Inherited File Descriptor
//!
//! Loaded into the guest ramfs at `/target` and `execv()`'d by each iteration of
//! `fork-exec-loop-test`. The parent opened a file before forking and passes that descriptor's
//! number as `argv[1]`; after exec the target reads from that INHERITED descriptor and verifies the
//! contents, exiting 0 on success or with a distinct non-zero status identifying the failing step.
//!
//! Reading the inherited descriptor only works if the child received the parent's vfsd-side file
//! descriptor table through the fork-clone. That is exactly the state the repeated `fork()`+`execv()`
//! race drops (vfsd: `failed to clone filesystem state (... error=AlreadyExists)`), so this is the
//! per-iteration check whose reliability the caller exercises.

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

use ::core::sync::atomic::Ordering;
use ::sysapi::ffi::c_int;
use ::syscall::unistd::{
    bindings,
    read,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Expected contents of the inherited file. Must match the seed written by the test harness.
const EXPECTED: &[u8] = b"COLD-READ-PAYLOAD-OK";

/// Exit status: argv did not carry the inherited descriptor number.
const ST_NO_ARGV: c_int = 40;

/// Exit status: read() from the inherited descriptor failed or returned the wrong contents.
const ST_READ_MISMATCH: c_int = 42;

//==================================================================================================
// Helpers
//==================================================================================================

/// Parses a non-negative decimal integer from the leading ASCII digits of `bytes`, stopping at the
/// end of the slice or at the first NUL byte (whichever comes first; the input need not be
/// NUL-terminated). Returns `None` if a non-digit byte precedes any NUL, or if no digits are present.
fn parse_fd(bytes: &[u8]) -> Option<c_int> {
    let mut acc: i32 = 0;
    let mut seen: bool = false;
    for &b in bytes {
        if b == 0 {
            break;
        }
        if !b.is_ascii_digit() {
            return None;
        }
        let digit: i32 = i32::from(b - b'0');
        acc = acc.checked_mul(10)?.checked_add(digit)?;
        seen = true;
    }
    if seen { Some(acc) } else { None }
}

/// Returns `argv[1]` as a byte slice borrowing the bytes up to (but excluding) the terminating NUL,
/// bounded to 32 bytes, or `None` if absent.
fn argv1() -> Option<&'static [u8]> {
    let argc: usize = usize::try_from(nvx_crt0::ARGC.load(Ordering::SeqCst)).unwrap_or(0);
    if argc < 2 {
        return None;
    }
    let argv: *mut *const u8 = nvx_crt0::ARGV.load(Ordering::SeqCst);
    if argv.is_null() {
        return None;
    }
    // SAFETY: argc >= 2 guarantees argv[1] is a valid NUL-terminated C string pointer.
    let ptr: *const u8 = unsafe { *argv.add(1) };
    if ptr.is_null() {
        return None;
    }
    let mut len: usize = 0;
    // SAFETY: the string is NUL-terminated; stop at the first NUL (bounded for safety).
    while len < 32 && unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: `ptr` points to `len` initialized bytes.
    Some(unsafe { ::core::slice::from_raw_parts(ptr, len) })
}

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), ::sys::error::Error> {
    let fd: c_int = match argv1().and_then(parse_fd) {
        Some(fd) => fd,
        // SAFETY: the process holds no resources requiring cleanup; terminate immediately.
        None => unsafe { bindings::_exit::_exit(ST_NO_ARGV) },
    };

    // Read from the INHERITED descriptor. This requires the parent's vfsd fd table to have been
    // cloned onto this child during fork; if the clone was refused, the descriptor is unknown here.
    let mut buf: [u8; EXPECTED.len()] = [0u8; EXPECTED.len()];
    let n: usize = match read(fd, &mut buf) {
        // read() never returns more than buf.len() bytes, so narrowing to usize cannot fail in
        // practice; route any unexpected value through the read-failure path rather than masking
        // it as 0.
        Ok(bytes) => match usize::try_from(bytes) {
            Ok(n) => n,
            // SAFETY: the process holds no resources requiring cleanup; terminate immediately.
            Err(_) => unsafe { bindings::_exit::_exit(ST_READ_MISMATCH) },
        },
        // SAFETY: the process holds no resources requiring cleanup; terminate immediately.
        Err(_) => unsafe { bindings::_exit::_exit(ST_READ_MISMATCH) },
    };

    if n != EXPECTED.len() || buf != *EXPECTED {
        // SAFETY: as above.
        unsafe { bindings::_exit::_exit(ST_READ_MISMATCH) };
    }

    Ok(())
}
