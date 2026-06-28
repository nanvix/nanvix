// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Regression test for the kernel→user argument copy path in `create_process()`.
//!
//! During process creation the kernel copies the argument string from a heap-allocated `CString`
//! (kernel GVA) into a freshly-mapped user page (GPA) via `copy_to_user_unaligned`.
//!
//! This test validates that `argv[0]` arrives intact by comparing its first bytes against the
//! known binary name prefix. A mismatch indicates a broken kernel-side copy path.

use ::core::sync::atomic::Ordering;
use ::sys::error::{
    Error,
    ErrorCode,
};

const EXPECTED_ARGV0_PREFIX: &[u8] = b"test-rust-misc";

#[allow(clippy::as_conversions)]
pub fn run() -> Result<(), Error> {
    let argv: *const *const u8 = ::nvx_crt0::ARGV.load(Ordering::SeqCst) as *const *const u8;
    let arg0: *const u8 = unsafe { *argv };

    for (i, &expected) in EXPECTED_ARGV0_PREFIX.iter().enumerate() {
        let got: u8 = unsafe { *arg0.add(i) };
        if got != expected {
            ::syslog::error!("argv[0] byte {i}: expected {expected:#04x}, got {got:#04x}");
            return Err(Error::new(ErrorCode::InvalidArgument, "argv[0] corrupted"));
        }
    }

    Ok(())
}
