// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

#[cfg(target_arch = "aarch64")]
use ::core::arch::asm;
use ::core::{
    ptr,
    sync::atomic::{
        AtomicBool,
        AtomicUsize,
        Ordering,
    },
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        mm,
        pm,
    },
    mm::{
        AccessPermission,
        VirtualAddress,
    },
    pm::{
        ProcessIdentifier,
        SigAction,
        SIGSEGV,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Wild-pointer target. It sits in the unmapped gap above the user mmap region and below the user
/// stack, so a write through it always faults with no valid mapping: the kernel cannot resolve it
/// via demand paging or copy-on-write, and — in this no-daemon image — no exception owner has
/// claimed the page-fault vector via `evctrl()`, so the fault maps to a synchronous `SIGSEGV`.
const WILD_PTR: usize = 0xe000_0000;

/// Byte the wild write stores, read back after the fault is resolved to confirm the interrupted
/// instruction re-executed correctly after `sigreturn()`.
const WILD_VALUE: u8 = 0x42;

/// FP/SIMD pattern installed in the interrupted context.
#[cfg(target_arch = "aarch64")]
const INTERRUPTED_FP_PATTERN: u64 = 0x1122_3344_5566_7788;

/// FP/SIMD pattern installed by the signal handler.
#[cfg(target_arch = "aarch64")]
const HANDLER_FP_PATTERN: u64 = 0x8877_6655_4433_2211;

//==================================================================================================
// Global State
//==================================================================================================

/// Records whether the `SIGSEGV` handler ran.
static HANDLER_RAN: AtomicBool = AtomicBool::new(false);

/// Counts how many times the handler ran, so a single fault (rather than a fault loop) is asserted.
static FAULT_COUNT: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Handler installed for `SIGSEGV`. It resolves the wild-pointer fault by mapping the faulting page
/// writable, so that when the kernel restores the interrupted context through `sigreturn()` the
/// original write re-executes and completes instead of faulting again. A synchronous fault handler
/// cannot simply return without resolving the fault, or the faulting instruction would re-fault in
/// an endless loop.
extern "C" fn sigsegv_handler(_signum: i32) {
    FAULT_COUNT.fetch_add(1, Ordering::SeqCst);
    if let Ok(pid) = pm::getpid() {
        // Best-effort: a failure here leaves the page unmapped, so the resumed write faults again
        // and the test's assertions fail loudly rather than the suite passing silently.
        let _ = mm::__kcall_mmap(pid, VirtualAddress::new(WILD_PTR), 1, AccessPermission::RDWR);
    }
    HANDLER_RAN.store(true, Ordering::SeqCst);

    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!(
            "dup v0.2d, {pattern}",
            pattern = in(reg) HANDLER_FP_PATTERN,
            out("v0") _,
            options(nomem, nostack),
        );
    }
}

/// Returns the address of [`sigsegv_handler`] for the `sa_handler` slot. Forming a pointer-sized
/// value from a function item is exactly what the disposition's handler slot expects.
fn sigsegv_handler_addr() -> usize {
    sigsegv_handler as *const () as usize
}

#[cfg(target_arch = "aarch64")]
unsafe fn fault_with_fp_state(wild: *mut u8) -> (u64, u64) {
    let low: u64;
    let high: u64;
    unsafe {
        asm!(
            "dup v0.2d, {pattern}",
            "strb {value:w}, [{wild}]",
            "umov {low}, v0.d[0]",
            "umov {high}, v0.d[1]",
            pattern = in(reg) INTERRUPTED_FP_PATTERN,
            value = in(reg) u64::from(WILD_VALUE),
            wild = in(reg) wild,
            low = lateout(reg) low,
            high = lateout(reg) high,
            out("v0") _,
            options(nostack),
        );
    }
    (low, high)
}

//==================================================================================================
// Entry Point
//==================================================================================================

///
/// # Description
///
/// Entry point of the synchronous-`SIGSEGV` test. Validates that a wild-pointer write — a page
/// fault that the kernel cannot resolve and that no exception owner has claimed — is mapped to a
/// catchable `SIGSEGV` delivered to the faulting thread, runs the installed handler, and resumes the
/// interrupted instruction once the handler resolves the fault.
///
/// This exercises the synchronous (exception-driven) signal path end-to-end: vector-to-signal
/// mapping, faulting-context capture, signal-frame build, handler invocation, and `sigreturn()`
/// resumption of the faulting instruction.
///
/// # Expected Behavior
///
/// The handler runs exactly once, maps the faulting page, and the resumed write stores
/// [`WILD_VALUE`] at [`WILD_PTR`]. The process then exits successfully (exit code 0).
///
#[no_mangle]
pub fn main() -> Result<(), Error> {
    // Install a catching disposition for SIGSEGV.
    let act: SigAction = SigAction {
        sa_handler: sigsegv_handler_addr(),
        sa_mask: 0,
        sa_flags: 0,
        sa_sigaction: 0,
    };
    let signum: i32 = i32::try_from(SIGSEGV)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "signal number out of range"))?;
    // SAFETY: `act` is a valid, properly aligned `SigAction`; the old-action pointer is null.
    unsafe { pm::__kcall_sigaction(signum, &raw const act, ptr::null_mut()) }?;

    // Write through a wild pointer. The access faults with no valid mapping; the kernel converts it
    // into a synchronous SIGSEGV delivered to this thread. The handler maps the page so this write
    // completes when the interrupted instruction re-executes after the kernel restores the context.
    let wild: *mut u8 = ptr::with_exposed_provenance_mut(WILD_PTR);
    // SAFETY: the write intentionally faults; the handler resolves the fault before the instruction
    // is restarted.
    #[cfg(target_arch = "aarch64")]
    {
        let (low, high): (u64, u64) = unsafe { fault_with_fp_state(wild) };
        if low != INTERRUPTED_FP_PATTERN || high != INTERRUPTED_FP_PATTERN {
            return Err(Error::new(
                ErrorCode::TryAgain,
                "SIGSEGV return did not restore the interrupted FP/SIMD state",
            ));
        }
    }
    #[cfg(not(target_arch = "aarch64"))]
    unsafe {
        ptr::write_volatile(wild, WILD_VALUE);
    }

    // The handler must have run.
    if !HANDLER_RAN.load(Ordering::SeqCst) {
        return Err(Error::new(ErrorCode::NoSuchEntry, "SIGSEGV handler did not run"));
    }

    // The fault must have been delivered exactly once (the resumed write must not fault again).
    if FAULT_COUNT.load(Ordering::SeqCst) != 1 {
        return Err(Error::new(
            ErrorCode::TryAgain,
            "SIGSEGV did not resolve in a single delivery",
        ));
    }

    // The interrupted write must have completed after the handler resolved the fault, confirming the
    // faulting context was captured and restored correctly.
    // SAFETY: the handler mapped this page, so the read is now valid.
    let observed: u8 = unsafe { ptr::read_volatile(wild) };
    if observed != WILD_VALUE {
        return Err(Error::new(
            ErrorCode::BadAddress,
            "wild write did not complete after the handler",
        ));
    }

    // Release the page the handler mapped so the test leaves no lingering mapping.
    let pid: ProcessIdentifier = pm::getpid()?;
    let _ = mm::__kcall_munmap(pid, VirtualAddress::new(WILD_PTR));

    Ok(())
}
