// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::common::{
    StressError,
    WorkerStack,
    error_code_from_usize,
    error_code_to_usize,
    thread_args,
};
use ::core::{
    convert::TryFrom,
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::pm::{
        __kcall_create_thread,
        __kcall_getpid,
        __kcall_gettid,
        __kcall_join_thread,
    },
    pm::{
        ProcessIdentifier,
        ThreadCreateArgs,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

const IDENTITY_RETURN_TAG: usize = 0xdecafbad;
const IDENTITY_FAILURE: usize = usize::MAX;

//==================================================================================================
// Globals
//==================================================================================================

static THREAD_IDENTITY_PID: AtomicUsize = AtomicUsize::new(0);
static THREAD_IDENTITY_TID: AtomicUsize = AtomicUsize::new(0);
static THREAD_IDENTITY_ERROR: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Verifies `getpid`/`gettid` consistency in a child thread to ensure thread IDs differ while the
/// process ID matches the parent, mirroring service threads spun off from a daemon.
///
/// # Returns
///
/// `Ok(())` on success or an error if thread lifecycle or kcall operations fail.
///
pub fn run() -> Result<(), StressError> {
    THREAD_IDENTITY_PID.store(0, Ordering::Relaxed);
    THREAD_IDENTITY_TID.store(0, Ordering::Relaxed);

    let main_pid: ProcessIdentifier = __kcall_getpid()?;
    let main_tid: ThreadIdentifier = __kcall_gettid()?;

    let stack: WorkerStack = WorkerStack::new(::config::memory_layout::USER_THREAD_STACK_SIZE)?;
    let mut args: ThreadCreateArgs = thread_args(&stack, thread_identity_worker, 0);
    let tid: ThreadIdentifier = __kcall_create_thread(&mut args)?;

    let mut retval: usize = 0;
    __kcall_join_thread(tid, &mut retval)?;
    drop(stack);

    if retval == IDENTITY_FAILURE {
        let code_raw: usize = THREAD_IDENTITY_ERROR.swap(0, Ordering::AcqRel);
        let code: ErrorCode = error_code_from_usize(code_raw);
        return Err(Error::new(code, "identity worker failed"));
    }

    if retval != IDENTITY_RETURN_TAG {
        return Err(Error::new(ErrorCode::InvalidArgument, "identity worker returned wrong tag"));
    }

    let observed_pid: usize = THREAD_IDENTITY_PID.load(Ordering::Acquire);
    let observed_tid: usize = THREAD_IDENTITY_TID.load(Ordering::Acquire);
    let main_pid_usize: usize = usize::try_from(main_pid)?;
    let main_tid_usize: usize = usize::try_from(main_tid)?;

    if observed_pid != main_pid_usize {
        return Err(Error::new(ErrorCode::InvalidArgument, "worker pid mismatch"));
    }

    if observed_tid == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument, "worker tid not recorded"));
    }

    if observed_tid == main_tid_usize {
        return Err(Error::new(ErrorCode::InvalidArgument, "worker tid matches parent"));
    }

    Ok(())
}

//==================================================================================================
// Worker Functions
//==================================================================================================

///
/// # Description
///
/// Entry point for the identity worker that records its PID and TID.
///
/// # Parameters
///
/// - `_`: Unused parameter required by the worker ABI.
///
/// # Returns
///
/// Completion tag used by the parent for validation.
extern "C" fn thread_identity_worker(_: usize) -> usize {
    match thread_identity_worker_impl() {
        Ok(tag) => tag,
        Err(err) => {
            THREAD_IDENTITY_ERROR.store(error_code_to_usize(err.code), Ordering::Release);
            IDENTITY_FAILURE
        },
    }
}

///
/// # Description
///
/// Captures the worker's process and thread identifiers for later validation.
///
/// # Returns
///
/// `Ok(tag)` on success where `tag` encodes the identity completion marker.
fn thread_identity_worker_impl() -> Result<usize, Error> {
    let pid: ProcessIdentifier = __kcall_getpid()?;
    let tid: ThreadIdentifier = __kcall_gettid()?;

    THREAD_IDENTITY_PID.store(usize::try_from(pid)?, Ordering::Release);
    THREAD_IDENTITY_TID.store(usize::try_from(tid)?, Ordering::Release);

    Ok(IDENTITY_RETURN_TAG)
}
