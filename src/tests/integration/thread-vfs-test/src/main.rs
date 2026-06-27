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

//! # Multi-threaded VFS Per-Process State Attribution Test (nanvix/nanvix#2529)
//!
//! Acceptance test for issue #2529: vfsd must attribute a filesystem request to the *owning
//! process* of the calling thread, not to the calling thread itself. vfsd keys per-process state
//! (the file-descriptor table and the working directory) by `ProcessIdentifier`, but a guest
//! syscall request carries the caller's `ThreadIdentifier` as its source, because vfsd needs the
//! TID to route the reply. Recovering the PID by casting that TID is correct only for a
//! single-threaded process whose main thread happens to share the PID value. PIDs and TIDs are
//! drawn from independent counters, and a secondary thread always has a *distinct* TID, so a
//! request issued from a non-main thread casts to a phantom process whose descriptor table is
//! empty.
//!
//! This test makes the main thread open descriptors and then has a *secondary* thread perform I/O
//! on them. With the bug present the secondary thread's request is misattributed and the
//! descriptors are absent from the phantom per-process table, so the read/write fails with `EBADF`
//! and the test FAILS. Once vfsd resolves the caller's owning PID authoritatively, both threads
//! share the one per-process descriptor table, the I/O succeeds, and the test passes -- guarding
//! the fix.
//!
//! `/input.dat` is pre-seeded with the payload by the test harness (see the standalone image
//! wiring in `build/make/nanvixd.mk`); `/output.dat` is created by the test at runtime.

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

use ::config::memory_layout::USER_THREAD_STACK_SIZE;
use ::core::sync::atomic::{
    AtomicI32,
    Ordering,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::pm::{
        __kcall_create_thread,
        __kcall_join_thread,
    },
    pm::{
        ThreadCreateArgs,
        ThreadIdentifier,
    },
};
use ::sysapi::{
    fcntl::{
        file_access_mode::{
            O_RDONLY,
            O_RDWR,
        },
        file_creation_flags::{
            O_CREAT,
            O_TRUNC,
        },
    },
    ffi::c_int,
    sys_types::mode_t,
    unistd::STDOUT_FILENO,
};
use ::syscall::{
    fcntl::open,
    safe::mem::stack::Stack,
    unistd::{
        close,
        read,
        write,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Pre-seeded input file the secondary thread reads through a descriptor the main thread opened.
const INPUT_PATH: &str = "/input.dat";

/// Output file the main thread creates and the secondary thread writes through.
const OUTPUT_PATH: &str = "/output.dat";

/// Payload shared with the seeded input file (see the image wiring in `build/make/nanvixd.mk`) and
/// written through the output descriptor. Must match the seeded bytes exactly.
const PAYLOAD: &[u8] = b"THREAD-VFS-2529-PAYLOAD";

/// Permissions for the created output file (read/write for the owner).
const FILE_MODE: mode_t = 0o600;

/// Mode argument for `open()` calls that do not create a file (ignored without `O_CREAT`).
const NO_MODE: mode_t = 0;

// Secondary-thread outcome codes, returned as the thread's exit value and checked by the main
// thread after join.
const WORKER_OK: usize = 0;
const WORKER_READ_FAILED: usize = 1;
const WORKER_READ_MISMATCH: usize = 2;
const WORKER_WRITE_FAILED: usize = 3;

//==================================================================================================
// Shared State
//==================================================================================================

/// Descriptor opened by the main thread on [`INPUT_PATH`], read by the secondary thread.
static SHARED_RFD: AtomicI32 = AtomicI32::new(-1);

/// Descriptor opened by the main thread on [`OUTPUT_PATH`], written by the secondary thread.
static SHARED_WFD: AtomicI32 = AtomicI32::new(-1);

//==================================================================================================
// Secondary Thread
//==================================================================================================

/// Reads [`INPUT_PATH`] and writes [`OUTPUT_PATH`] through descriptors the *main* thread opened.
///
/// Runs on a non-main thread whose `ThreadIdentifier` differs from the process `ProcessIdentifier`.
/// Both operations exercise the per-process descriptor-table attribution that #2529 is about: with
/// the bug present they fail with `EBADF`, because the descriptors are absent from the phantom
/// per-process table keyed by this thread's TID.
extern "C" fn worker_entry(_arg: usize) -> usize {
    let rfd: c_int = SHARED_RFD.load(Ordering::Acquire);
    let wfd: c_int = SHARED_WFD.load(Ordering::Acquire);

    // Cross-thread read on the descriptor the main thread opened.
    let mut buf: [u8; PAYLOAD.len()] = [0u8; PAYLOAD.len()];
    let mut filled: usize = 0;
    while filled < buf.len() {
        match read(rfd, &mut buf[filled..]) {
            Ok(n) => {
                let n: usize = match usize::try_from(n) {
                    Ok(n) => n,
                    Err(_) => return WORKER_READ_FAILED,
                };
                // A zero-length read means EOF before the full payload arrived.
                if n == 0 {
                    return WORKER_READ_FAILED;
                }
                filled += n;
            },
            // EBADF here is the #2529 symptom: the descriptor is missing from the misattributed
            // per-process table.
            Err(_) => return WORKER_READ_FAILED,
        }
    }
    if buf.as_slice() != PAYLOAD {
        return WORKER_READ_MISMATCH;
    }

    // Cross-thread write on the descriptor the main thread opened.
    let mut remaining: &[u8] = PAYLOAD;
    while !remaining.is_empty() {
        match write(wfd, remaining) {
            Ok(n) => {
                let n: usize = match usize::try_from(n) {
                    Ok(n) => n,
                    Err(_) => return WORKER_WRITE_FAILED,
                };
                if n == 0 {
                    return WORKER_WRITE_FAILED;
                }
                remaining = &remaining[n..];
            },
            Err(_) => return WORKER_WRITE_FAILED,
        }
    }

    WORKER_OK
}

//==================================================================================================
// Helpers
//==================================================================================================

/// Converts an `extern "C"` function pointer to the raw value expected by the kernel-call ABI.
#[allow(clippy::as_conversions, clippy::fn_to_numeric_cast)]
fn raw_entry_address(entry: extern "C" fn(usize) -> usize) -> usize {
    entry as *const () as usize
}

/// Spawns [`worker_entry`] on a secondary thread, waits for it, and returns its outcome code.
fn run_worker() -> Result<usize, Error> {
    let stack: Stack = Stack::new(USER_THREAD_STACK_SIZE)?;
    let mut args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: raw_entry_address(worker_entry),
        user_fn_arg1: 0,
        user_stack_base: stack.base(),
        user_stack_size: stack.size(),
        user_tda: None,
    };
    let tid: ThreadIdentifier = __kcall_create_thread(&mut args)?;

    let mut retval: usize = 0;
    __kcall_join_thread(tid, &mut retval)?;
    // The thread has been joined, so its stack is no longer in use and can be released.
    drop(stack);

    Ok(retval)
}

//==================================================================================================
// Test
//==================================================================================================

/// Verifies that a secondary thread shares the main thread's per-process VFS descriptor table.
fn test_thread_shares_process_fd_table() -> Result<(), Error> {
    // The main thread opens both descriptors; they land in THIS process's per-process descriptor
    // table inside vfsd.
    let rfd: c_int = open(INPUT_PATH, O_RDONLY, NO_MODE)?;
    let wfd: c_int = open(OUTPUT_PATH, O_RDWR | O_CREAT | O_TRUNC, FILE_MODE)?;
    SHARED_RFD.store(rfd, Ordering::Release);
    SHARED_WFD.store(wfd, Ordering::Release);

    // The secondary thread (a non-main thread, whose TID differs from this process PID) reads and
    // writes those descriptors. This succeeds only if vfsd attributes its requests to the owning
    // process rather than casting its TID to a phantom process identifier.
    let outcome: usize = run_worker()?;
    assert!(
        outcome == WORKER_OK,
        "secondary-thread VFS I/O on the main thread's descriptors failed (outcome={}); vfsd \
         misattributed the per-process descriptor table to the calling thread's TID",
        outcome
    );

    close(rfd)?;
    close(wfd)?;

    // End-to-end: the bytes the secondary thread wrote must be in the file. Re-open from the main
    // thread (a fresh descriptor at offset zero) and read them back.
    let vfd: c_int = open(OUTPUT_PATH, O_RDONLY, NO_MODE)?;
    let mut buf: [u8; PAYLOAD.len()] = [0u8; PAYLOAD.len()];
    let n: usize = usize::try_from(read(vfd, &mut buf)?)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "invalid read length"))?;
    close(vfd)?;
    assert!(
        n == PAYLOAD.len() && buf.as_slice() == PAYLOAD,
        "data written by the secondary thread is not visible/correct (read {} bytes)",
        n
    );

    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    ::syslog::info!(
        "thread-vfs-test: starting multi-threaded VFS per-process attribution test (#2529)"
    );

    test_thread_shares_process_fd_table()?;
    ::syslog::info!("thread-vfs-test: PASS - thread_shares_process_fd_table");

    // Magic string consumed by the CI harness to mark a successful run.
    let magic_string: &[u8] = b"ok";
    write(STDOUT_FILENO, magic_string)?;

    Ok(())
}
