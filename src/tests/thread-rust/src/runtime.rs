// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::config::memory_layout::USER_THREAD_STACK_SIZE;
use ::core::time::Duration;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::pm::{
        __kcall_create_thread,
        __kcall_detach_thread,
        __kcall_gettime,
        __kcall_join_thread,
    },
    pm::{
        ThreadCreateArgs,
        ThreadIdentifier,
    },
    time::SystemTime,
};
use ::syscall::safe::mem::stack::Stack;

//==================================================================================================
// Structures
//==================================================================================================

/// Lightweight owner of a kernel thread spawned through `create_thread()`.
#[allow(dead_code)]
pub struct KernelThread {
    tid: Option<ThreadIdentifier>,
    stack: Option<Stack>,
}

//==================================================================================================
// Implementations
//==================================================================================================

#[allow(dead_code)]
impl KernelThread {
    /// Spawns a new kernel thread that starts executing `entry` with `arg`.
    pub fn spawn(entry: extern "C" fn(usize) -> usize, arg: usize) -> Result<Self, Error> {
        let stack: Stack = Stack::new(USER_THREAD_STACK_SIZE)?;

        let mut args: ThreadCreateArgs = ThreadCreateArgs {
            user_fn: ThreadCreateArgs::NULL_USER_FN,
            user_fn_arg0: raw_entry_address(entry),
            user_fn_arg1: arg,
            user_stack_base: stack.base(),
            user_stack_size: stack.size(),
            user_tda: None,
        };

        let tid: ThreadIdentifier = __kcall_create_thread(&mut args)?;

        Ok(Self {
            tid: Some(tid),
            stack: Some(stack),
        })
    }

    /// Waits for the kernel thread to finish and returns its exit status.
    pub fn join(mut self) -> Result<usize, Error> {
        let mut retval: usize = 0;
        let tid: ThreadIdentifier = self
            .tid
            .take()
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "thread handle missing"))?;
        __kcall_join_thread(tid, &mut retval)?;
        drop(self.stack.take());
        Ok(retval)
    }

    /// Detaches the thread so it is auto-harvested when it exits.
    ///
    /// The caller's `Stack` handle is intentionally leaked because the thread may still be
    /// using it. The kernel will unmap the thread's stack pages once the detached thread
    /// terminates (and, in the worst case, on process teardown).
    pub fn detach(mut self) -> Result<(), Error> {
        let tid: ThreadIdentifier = self
            .tid
            .take()
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "thread handle missing"))?;
        __kcall_detach_thread(tid)?;
        // Intentionally leak the stack — the thread may still be using it.
        if let Some(stack) = self.stack.take() {
            core::mem::forget(stack);
        }
        Ok(())
    }
}

impl Drop for KernelThread {
    fn drop(&mut self) {
        if self.tid.is_some() {
            panic!("KernelThread dropped without joining or detaching");
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Returns the current monotonic system time.
pub fn monotonic_now() -> Result<SystemTime, Error> {
    let mut now: SystemTime = SystemTime::default();
    __kcall_gettime(&mut now)?;
    Ok(now)
}

/// Computes an absolute deadline `duration` in the future.
pub fn deadline_from_now(duration: Duration) -> Result<SystemTime, Error> {
    let now: SystemTime = monotonic_now()?;
    now.checked_add_duration(&duration)
        .ok_or_else(|| Error::new(ErrorCode::ValueOutOfRange, "deadline overflow"))
}

//==================================================================================================
// Helpers
//==================================================================================================

/// Converts an extern "C" function pointer to the raw value expected by the kernel call ABI.
#[allow(clippy::as_conversions, clippy::fn_to_numeric_cast)]
pub fn raw_entry_address(entry: extern "C" fn(usize) -> usize) -> usize {
    entry as *const () as usize
}

/// Converts a raw pointer to its integer representation for kernel calls.
#[allow(clippy::as_conversions)]
pub fn raw_pointer_address<T>(ptr: *mut T) -> usize {
    ptr as usize
}
