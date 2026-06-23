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

//! # `execv()` Target That Verifies an `argv` Token Containing a Space
//!
//! Loaded into the guest ramfs at `/target` and `execv()`'d by the child that
//! `fork-exec-argv-space-test` forks. The caller passes a single argument that contains an embedded
//! space -- [`EXPECTED_ARG`] (`"alpha beta"`) -- as `argv[1]`. POSIX places no restriction on the
//! bytes of an argument other than the terminating NUL, so a space must be carried verbatim and
//! delivered as ONE argument.
//!
//! After exec this target reads `argv[1]` and checks it equals [`EXPECTED_ARG`] exactly, then
//! `_exit()`s `0`. It exits with a distinct non-zero status if the argument is missing (e.g. the
//! space-separated wire format split `"alpha beta"` into two arguments, so `argv[1]` is only
//! `"alpha"`) or does not match. The companion caller checks the child exited `0`.

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
use ::sys::error::Error;
use ::sysapi::ffi::c_int;
use ::syscall::unistd::bindings;

//==================================================================================================
// Constants
//==================================================================================================

/// The argument the caller passes as `argv[1]`. It contains an embedded space, which POSIX requires
/// to be delivered verbatim as a single argument. Must match fork-exec-argv-space-test.
const EXPECTED_ARG: &[u8] = b"alpha beta";

/// Exit status: `argv[1]` was absent (the runtime did not deliver a second argument at all).
const ST_NO_ARGV: c_int = 40;

/// Exit status: `argv[1]` was present but did not equal [`EXPECTED_ARG`] (e.g. it was split on the
/// space, so only `"alpha"` arrived).
const ST_ARG_MISMATCH: c_int = 41;

//==================================================================================================
// Helpers
//==================================================================================================

/// Returns `argv[1]` as a byte slice borrowing the bytes up to (but excluding) the terminating NUL,
/// bounded to 64 bytes, or `None` if absent.
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
    while len < 64 && unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: `ptr` points to `len` initialized bytes.
    Some(unsafe { ::core::slice::from_raw_parts(ptr, len) })
}

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let arg: &[u8] = match argv1() {
        Some(a) => a,
        // SAFETY: the process holds no resources requiring cleanup; terminate immediately.
        None => unsafe { bindings::_exit::_exit(ST_NO_ARGV) },
    };

    if arg == EXPECTED_ARG {
        // SAFETY: the process holds no resources requiring cleanup; terminate immediately.
        unsafe { bindings::_exit::_exit(0) };
    } else {
        // SAFETY: as above.
        unsafe { bindings::_exit::_exit(ST_ARG_MISMATCH) };
    }
}
