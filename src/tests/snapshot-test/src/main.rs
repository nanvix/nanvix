// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]
#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

extern crate libc_string;
extern crate nvx;

use ::core::sync::atomic::Ordering;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Sentinel exit-code returned after the post-snapshot region completes. The integration test
/// asserts that both the snapshot-save phase and the snapshot-restore phase observe this
/// code; a broken restore path bypasses the post-snapshot region and the guest exits with
/// `0` instead. `BrokenPipe` (POSIX `EPIPE` = 32) is used because it is unlikely to surface
/// elsewhere in this workload.
const SNAPSHOT_RESTORE_SENTINEL: ErrorCode = ErrorCode::BrokenPipe;

/// Flag passed by the test harness to request that the workload trigger a snapshot.
const SNAPSHOT_FLAG: &[u8] = b"--snapshot";

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Returns `true` if `--snapshot` was passed as a command-line argument.
fn should_snapshot() -> bool {
    let argc: i32 = nvx::ARGC.load(Ordering::SeqCst);
    let argv: *mut *const u8 = nvx::ARGV.load(Ordering::SeqCst);

    if argv.is_null() || argc <= 1 {
        return false;
    }

    for i in 1..argc {
        let ptr: *const u8 = unsafe { *argv.add(i as usize) };
        if ptr.is_null() {
            continue;
        }
        let mut matches: bool = true;
        for (j, &expected) in SNAPSHOT_FLAG.iter().enumerate() {
            let byte: u8 = unsafe { *ptr.add(j) };
            if byte == 0 || byte != expected {
                matches = false;
                break;
            }
        }
        if matches && unsafe { *ptr.add(SNAPSHOT_FLAG.len()) } == 0 {
            return true;
        }
    }

    false
}

///
/// # Description
///
/// Minimal integration-test workload for the snapshot save / restore lifecycle. When invoked
/// with `--snapshot` it calls `pm::snapshot()` and, on return, exits with
/// `SNAPSHOT_RESTORE_SENTINEL` to mark that the post-snapshot region executed. Without the
/// flag it exits successfully (`Ok(())`) so the workload can also be used in non-snapshot
/// configurations.
///
/// `pm::snapshot()` returns in both the snapshot-creation run (after the VMM saves state and
/// resumes the guest) and in the snapshot-restore run (after the VMM loads state and resumes
/// the guest). Both runs therefore execute the post-snapshot region and exit with the
/// sentinel, which is what the `snapshot-restore` executor in nanvix-test asserts.
///
#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    if !should_snapshot() {
        return Ok(());
    }

    ::sys::kcall::pm::snapshot()?;

    Err(Error::new(SNAPSHOT_RESTORE_SENTINEL, "post-snapshot region executed"))
}
