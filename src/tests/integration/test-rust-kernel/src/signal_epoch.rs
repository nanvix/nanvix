// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::alloc::alloc::{
    alloc,
    dealloc,
    Layout,
};
use ::arch::mem::PAGE_SIZE;
use ::config::memory_layout::USER_THREAD_STACK_SIZE;
use ::core::sync::atomic::{
    AtomicBool,
    AtomicU32,
    Ordering,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::pm,
    mm::VirtualAddress,
    number::KcallNumber,
    pm::{
        ProcessIdentifier,
        SigAction,
        SigSet,
        ThreadCreateArgs,
        ThreadIdentifier,
        SIGUSR1,
        SIGUSR2,
        SIG_IGN,
    },
};

static OBSERVED_EPOCH: AtomicU32 = AtomicU32::new(0);
static NESTED: AtomicBool = AtomicBool::new(false);
static HANDLER_FAILED: AtomicBool = AtomicBool::new(false);

extern "C" fn handler(signum: i32) {
    match pm::__kcall_sigcheckpoint(None, None) {
        Ok(epoch) => OBSERVED_EPOCH.store(epoch, Ordering::Relaxed),
        Err(error) => {
            ::syslog::error!("signal checkpoint failed in handler (error={error:?})");
            HANDLER_FAILED.store(true, Ordering::Relaxed);
        },
    }
    if signum == SIGUSR1 as i32 && NESTED.load(Ordering::Relaxed) {
        let result: Result<(), Error> =
            pm::getpid().and_then(|pid| pm::__kcall_kill(pid, SIGUSR2 as i32));
        if let Err(error) = result {
            ::syslog::error!("nested signal failed (error={error:?})");
            HANDLER_FAILED.store(true, Ordering::Relaxed);
        }
    }
}

extern "C" fn worker(_arg: usize) -> usize {
    match pm::__kcall_sigcheckpoint(None, None) {
        Ok(epoch) => epoch as usize,
        Err(error) => {
            ::syslog::error!("worker signal checkpoint failed (error={error:?})");
            usize::MAX
        },
    }
}

fn test_thread_local_epoch() -> Result<(), Error> {
    let layout: Layout = Layout::from_size_align(USER_THREAD_STACK_SIZE, PAGE_SIZE)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "invalid stack layout"))?;
    // SAFETY: the layout is nonzero and page-aligned.
    let stack: *mut u8 = unsafe { alloc(layout) };
    if stack.is_null() {
        ::syslog::error!("failed to allocate signal-checkpoint worker stack");
        return Err(Error::new(ErrorCode::OutOfMemory, "failed to allocate worker stack"));
    }
    let mut args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: worker as *const () as usize,
        user_fn_arg1: 0,
        user_stack_base: VirtualAddress::from_raw_value(stack as usize),
        user_stack_size: USER_THREAD_STACK_SIZE,
        user_tda: None,
    };
    let tid: ThreadIdentifier = match pm::__kcall_create_thread(&mut args) {
        Ok(tid) => tid,
        Err(error) => {
            // SAFETY: no thread was created, so the allocation is not in use.
            unsafe { dealloc(stack, layout) };
            return Err(error);
        },
    };
    // Kernel thread exit statuses occupy 32 bits, including on x86_64.
    let mut result: usize = u32::MAX as usize;
    pm::__kcall_join_thread(tid, &mut result)?;
    // SAFETY: the joined thread no longer accesses its stack.
    unsafe { dealloc(stack, layout) };
    assert_eq!(result, 0, "a new thread inherited another thread's signal epoch");
    Ok(())
}

/// Tests atomic signal checkpoints independently of kernel-call interruption and user-space TLS.
pub fn run() -> Result<(), Error> {
    assert_eq!(pm::__kcall_alarm(u32::MAX)?, 0);
    // Exercise the checkpoint's two-argument transport with a full-width unsigned return value.
    let remaining: i64 = ::sys::kcall2!(KcallNumber::Alarm.into(), 0, 0);
    assert_eq!(remaining, i64::from(u32::MAX), "the two-argument transport corrupted the result");

    let action: SigAction = SigAction {
        sa_handler: handler as *const () as usize,
        ..SigAction::default()
    };
    let mut previous: [SigAction; 2] = [SigAction::default(); 2];
    for (signum, old) in [SIGUSR1, SIGUSR2].into_iter().zip(previous.iter_mut()) {
        // SAFETY: both action buffers are valid for the duration of the kernel call.
        unsafe { pm::__kcall_sigaction(signum as i32, &action, old)? };
    }

    let blocked: SigSet = !0;
    let mut old_mask: SigSet = 0;
    let initial: u32 = pm::__kcall_sigcheckpoint(Some(&blocked), Some(&mut old_mask))?;
    let pid: ProcessIdentifier = pm::getpid()?;
    pm::__kcall_kill(pid, SIGUSR1 as i32)?;
    assert_eq!(
        pm::__kcall_sigcheckpoint(None, None)?,
        initial,
        "posting a blocked signal advanced the delivery epoch"
    );

    let unblocked: SigSet = old_mask & !((1u64 << (SIGUSR1 - 1)) | (1u64 << (SIGUSR2 - 1)));
    let before_delivery: u32 = pm::__kcall_sigcheckpoint(Some(&unblocked), None)?;
    assert_eq!(before_delivery, initial, "checkpoint did not precede return-time delivery");
    let delivered: u32 = initial.wrapping_add(1);
    assert_eq!(OBSERVED_EPOCH.load(Ordering::Relaxed), delivered);
    assert_eq!(pm::__kcall_sigcheckpoint(None, None)?, delivered);

    NESTED.store(true, Ordering::Relaxed);
    pm::__kcall_kill(pid, SIGUSR1 as i32)?;
    NESTED.store(false, Ordering::Relaxed);
    let nested: u32 = delivered.wrapping_add(2);
    assert_eq!(OBSERVED_EPOCH.load(Ordering::Relaxed), nested);
    assert_eq!(pm::__kcall_sigcheckpoint(None, None)?, nested);

    let ignored: SigAction = SigAction {
        sa_handler: SIG_IGN,
        ..SigAction::default()
    };
    // SAFETY: the input action is valid and no output buffer is requested.
    unsafe { pm::__kcall_sigaction(SIGUSR2 as i32, &ignored, core::ptr::null_mut())? };
    pm::__kcall_kill(pid, SIGUSR2 as i32)?;
    assert_eq!(pm::__kcall_sigcheckpoint(None, None)?, nested);

    assert!(!HANDLER_FAILED.load(Ordering::Relaxed), "a signal handler kernel call failed");

    pm::__kcall_sigcheckpoint(Some(&blocked), None)?;
    pm::__kcall_kill(pid, SIGUSR1 as i32)?;
    pm::__kcall_alarm(1)?;
    let error: Error = ::sys::kcall::ipc::__kcall_recv_restoring_signals(unblocked)
        .expect_err("a pending signal must interrupt the receive wait");
    pm::__kcall_alarm(0)?;
    assert_eq!(error.code, ErrorCode::Interrupted);
    let received: u32 = nested.wrapping_add(1);
    assert_eq!(OBSERVED_EPOCH.load(Ordering::Relaxed), received);
    assert!(!HANDLER_FAILED.load(Ordering::Relaxed), "the receive handler kernel call failed");

    test_thread_local_epoch()?;
    assert_eq!(pm::__kcall_sigcheckpoint(None, None)?, received);
    pm::__kcall_sigcheckpoint(Some(&old_mask), None)?;
    for (signum, old) in [SIGUSR1, SIGUSR2].into_iter().zip(previous.iter()) {
        // SAFETY: the saved action remains valid and no output buffer is requested.
        unsafe { pm::__kcall_sigaction(signum as i32, old, core::ptr::null_mut())? };
    }
    Ok(())
}
