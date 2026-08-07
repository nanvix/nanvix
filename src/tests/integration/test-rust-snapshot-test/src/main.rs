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
extern crate nvx_crt0;

#[cfg(target_arch = "aarch64")]
use ::core::arch::asm;
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

/// Flag passed by the test harness to keep the guest running indefinitely after
/// `pm::snapshot()` returns, so any clean host exit must originate from the snapshot-completion
/// path rather than guest termination.
const NO_EXIT_FLAG: &[u8] = b"--no-exit";

#[cfg(target_arch = "aarch64")]
const SNAPSHOT_FP_PATTERN: u64 = 0x0123_4567_89ab_cdef;
#[cfg(target_arch = "aarch64")]
const SNAPSHOT_FPSR: u64 = 0x01;
#[cfg(target_arch = "aarch64")]
const FPCR_RMODE_MASK: u64 = 0b11 << 22;
#[cfg(target_arch = "aarch64")]
const SNAPSHOT_RMODE: u64 = 0b01 << 22;

#[cfg(target_arch = "aarch64")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FpState {
    d8: u64,
    fpcr: u64,
    fpsr: u64,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Returns `true` if `--snapshot` was passed as a command-line argument.
fn should_snapshot() -> bool {
    has_flag(SNAPSHOT_FLAG)
}

/// Returns `true` if `--no-exit` was passed as a command-line argument.
fn should_loop_after_snapshot() -> bool {
    has_flag(NO_EXIT_FLAG)
}

#[cfg(target_arch = "aarch64")]
fn read_fp_state() -> FpState {
    let d8: u64;
    let fpcr: u64;
    let fpsr: u64;
    unsafe {
        asm!(
            "fmov {d8}, d8",
            "mrs {fpcr}, fpcr",
            "mrs {fpsr}, fpsr",
            d8 = out(reg) d8,
            fpcr = out(reg) fpcr,
            fpsr = out(reg) fpsr,
            options(nomem, nostack),
        );
    }
    FpState { d8, fpcr, fpsr }
}

#[cfg(target_arch = "aarch64")]
fn write_fp_state(state: FpState) {
    unsafe {
        asm!(
            "fmov d8, {d8}",
            "msr fpcr, {fpcr}",
            "msr fpsr, {fpsr}",
            d8 = in(reg) state.d8,
            fpcr = in(reg) state.fpcr,
            fpsr = in(reg) state.fpsr,
            options(nomem, nostack),
        );
    }
}

/// Returns `true` if `flag` appears as a standalone argument (followed by a NUL terminator) in
/// `argv`.
fn has_flag(flag: &[u8]) -> bool {
    let argc: i32 = nvx_crt0::ARGC.load(Ordering::SeqCst);
    let argv: *mut *const u8 = nvx_crt0::ARGV.load(Ordering::SeqCst);

    if argv.is_null() || argc <= 1 {
        return false;
    }

    for i in 1..argc {
        let ptr: *const u8 = unsafe { *argv.add(i as usize) };
        if ptr.is_null() {
            continue;
        }
        let mut matches: bool = true;
        for (j, &expected) in flag.iter().enumerate() {
            let byte: u8 = unsafe { *ptr.add(j) };
            if byte == 0 || byte != expected {
                matches = false;
                break;
            }
        }
        if matches && unsafe { *ptr.add(flag.len()) } == 0 {
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

    #[cfg(target_arch = "aarch64")]
    let original_fp: FpState = read_fp_state();
    #[cfg(target_arch = "aarch64")]
    let snapshot_fp: FpState = FpState {
        d8: SNAPSHOT_FP_PATTERN,
        fpcr: (original_fp.fpcr & !FPCR_RMODE_MASK) | SNAPSHOT_RMODE,
        fpsr: SNAPSHOT_FPSR,
    };
    #[cfg(target_arch = "aarch64")]
    write_fp_state(snapshot_fp);

    ::sys::kcall::pm::snapshot()?;

    #[cfg(target_arch = "aarch64")]
    {
        let restored_fp: FpState = read_fp_state();
        write_fp_state(original_fp);
        if restored_fp != snapshot_fp {
            return Err(Error::new(
                ErrorCode::TryAgain,
                "snapshot restore did not preserve FP state",
            ));
        }
    }

    if should_loop_after_snapshot() {
        // Spin forever so guest exit cannot mask a host-side snapshot-completion hang.
        loop {
            ::core::hint::spin_loop();
        }
    }

    Err(Error::new(SNAPSHOT_RESTORE_SENTINEL, "post-snapshot region executed"))
}
