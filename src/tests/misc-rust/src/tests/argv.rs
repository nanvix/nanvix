// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Regression test for the kernel→user copy path in `create_process()`.
//!
//! During process creation the kernel copies the argument string from a heap-allocated `CString`
//! (kernel GVA) into a freshly-mapped user page (GPA) via `copy_to_user_unaligned`. The
//! underlying `no_identity_map::memcpy` on Hyperlight expects **GPAs** on both endpoints and
//! internally translates each GPA→GVA before dereferencing.
//!
//! On Hyperlight, after eager copy-on-write pre-faulting, writable kernel pages (heap, BSS, data)
//! may reside at a low-memory GVA while their actual backing frame lives in the scratch region at
//! a different GPA. Without the `virt_to_phys()` translation on the kernel-side source address,
//! `memcpy` receives the raw GVA as if it were a GPA, resolves it to the wrong physical frame,
//! and copies garbage into user space — corrupting `argv`.
//!
//! This test validates that `argv[0]` arrives intact by comparing its first bytes against the
//! known binary name prefix. A mismatch indicates the kernel-side GVA→GPA translation is missing
//! or broken.
//!
//! On microvm, GVA == GPA (identity-mapped), so the bug does not manifest and this test passes
//! unconditionally — it serves as a regression guard exclusively for the Hyperlight backend.

use ::core::sync::atomic::Ordering;
use ::sys::error::{
    Error,
    ErrorCode,
};

const EXPECTED_ARGV0_PREFIX: &[u8] = b"misc-rust";

#[allow(clippy::as_conversions)]
pub fn run() -> Result<(), Error> {
    let argv: *const *const u8 = ::nvx::ARGV.load(Ordering::SeqCst) as *const *const u8;
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
