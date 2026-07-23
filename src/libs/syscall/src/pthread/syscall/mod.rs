// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

/// Condition variables.
mod cond;

/// Mutexes.
mod mutex;

/// Read-write locks.
mod rwlock;

/// Semaphores.
mod sem;

/// Thread-specific data area.
mod tda;

/// Implementation of `pthread_attr_destroy()` system call.
mod pthread_attr_destroy;

/// Implementation of `pthread_attr_init()` system call.
mod pthread_attr_init;

/// Implementation of `pthread_attr_getstack()` system call.
mod pthread_attr_getstack;

/// Implementation of `pthread_attr_setstacksize()` system call.
mod pthread_attr_setstacksize;

/// Implementation of `pthread_getattr_np()` system call.
mod pthread_getattr_np;

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::mem::stack::Stack;
use ::alloc::collections::btree_map::BTreeMap;
use ::spin::{
    Lazy,
    Mutex,
};
use ::sys::{
    error::Error,
    kcall::{
        arch,
        pm::{
            __kcall_create_thread,
            __kcall_exit_thread,
            __kcall_gettid,
            __kcall_join_thread,
        },
    },
    mm::VirtualAddress,
    pm::{
        ThreadCreateArgs,
        ThreadIdentifier,
    },
};
use ::sysalloc::tda::{
    TDA_TID_OFFSET,
    TDA_TID_UNSET,
};
use ::sysapi::sys_types::pthread_t;

//==================================================================================================
// Exports
//==================================================================================================

pub use cond::*;
pub use mutex::*;
pub use pthread_attr_destroy::*;
pub use pthread_attr_getstack::*;
pub use pthread_attr_init::*;
pub use pthread_attr_setstacksize::*;
pub use pthread_getattr_np::*;
pub use rwlock::*;
pub use sem::*;
pub use tda::*;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Map of stacks for threads.
static STACKS: Lazy<Mutex<BTreeMap<pthread_t, Stack>>> = Lazy::new(|| Mutex::new(BTreeMap::new()));

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new thread.
///
/// # Parameters
///
/// - `start_routine`: Function to be executed by the thread.
/// - `arg`: Argument passed to the thread function.
/// - `stack_size`: Size of the stack for the new thread.
///
/// # Return Value
///
/// On successful completion, this function returns an identifier for the newly created thread. On
/// failure, this function returns an error that contains the reason for the failure.
///
pub fn pthread_create(
    func: extern "C" fn(usize) -> usize,
    arg: usize,
    stack_size: usize,
) -> Result<pthread_t, Error> {
    ::syslog::trace!(
        "pthread_create(): func={:?}, arg={arg:?}, stack_size={stack_size}",
        ::core::ptr::addr_of!(func)
    );

    // Create a new stack.
    // NOTE: The stack is automatically deallocated if this object is dropped.
    // TODO: Allocate thread stack on-demand via kernel mmap support (https://github.com/nanvix/nanvix/issues/1699).
    let stack: Stack = Stack::new(stack_size)?;

    let user_tda: Option<VirtualAddress> =
        ::sysalloc::tda::alloc()?.map(|tda_ptr| VirtualAddress::new(tda_ptr as usize));

    let mut args: ThreadCreateArgs = ThreadCreateArgs {
        // Placeholder for user wrapper function, it will be overridden by the kernel call interface.
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: func as usize,
        user_fn_arg1: arg,
        user_stack_base: stack.base(),
        user_stack_size: stack.size(),
        user_tda,
    };

    let thread: pthread_t = __kcall_create_thread(&mut args)?.try_into()?;

    // Store the stack in the global map.
    // NOTE: The stack will be dropped either when the thread is joined or the process exits.
    let previous_stack: Option<Stack> = STACKS.lock().insert(thread, stack);
    debug_assert!(previous_stack.is_none(), "there should be no previous stack for this thread");

    Ok(thread)
}

///
/// # Description
///
/// Joins a thread.
///
/// # Parameters
///
/// - `thread`: Thread identifier.
///
/// # Return Value
///
/// On successful completion, this function returns the return value of the joined thread. On
/// failure, this function returns an error that contains the reason for the failure.
///
pub fn pthread_join(thread: pthread_t) -> Result<usize, Error> {
    ::syslog::trace!("pthread_join(): thread={thread:?}");

    // Attempt to convert thread identifier.
    let tid: ThreadIdentifier = match thread.try_into() {
        Ok(tid) => tid,
        Err(error) => {
            ::syslog::warn!("pthread_join(): {error:?} (thread={thread:?})");
            return Err(error);
        },
    };

    let mut retval: usize = 0;
    __kcall_join_thread(tid, &mut retval)?;

    // Remove the stack from the global map.
    // NOTE: The stack will be dropped when this function returns.
    let stack: Option<Stack> = STACKS.lock().remove(&thread);
    debug_assert!(stack.is_some(), "there should be a stack for this thread");

    Ok(retval)
}

///
/// # Description
///
/// Exits the calling thread.
///
/// # Parameters
///
/// - `retval`: Return value of the thread.
///
/// # Return Value
///
/// On successful completion, this function does not return. On failure, this function returns an
/// error that contains the reason for the failure.
///
pub fn pthread_exit(retval: usize) -> Result<!, Error> {
    ::syslog::trace!("pthread_exit(): retval={:?}", retval);

    // Attempt to clean up thread data area and check for errors.
    if let Err(error) = ::sysalloc::tda::cleanup() {
        // Log warning and continue exiting, as we are about to terminate the thread anyway.
        ::syslog::warn!("pthread_exit(): failed to cleanup thread data area ({error:?})");
    }

    __kcall_exit_thread(retval)
}

///
/// # Description
///
/// Returns the identifier of the calling thread.
///
/// # Return Value
///
/// The identifier of the calling thread.
///
/// # Safety Notes
///
/// This function panics if:
/// - The thread is unable to retrieve its own identifier.
/// - The thread identifier returned by the kernel is not valid.
///
pub fn pthread_self() -> pthread_t {
    // Fast path: read the cached thread ID from the TDA via segment register.
    //
    // Safety: `is_initialized()` is process-wide but sound here because:
    // - It is set only after `set_thread_data_area()` succeeds (main thread init).
    // - Child threads always have a valid TDA configured by the kernel before first execution.
    // - `cleanup()` resets the flag before clearing the segment base and then diverges.
    if ::sysalloc::tda::is_initialized() {
        let tid: u32 = unsafe { arch::read_tda_u32(TDA_TID_OFFSET as u32) };

        // Check if the TDA slot contains a valid thread ID.
        if tid != TDA_TID_UNSET {
            return tid as pthread_t;
        }

        // Cache miss: the TDA slot still holds the TDA_TID_UNSET sentinel because tda::alloc()
        // cannot know the child's TID at allocation time. Resolve via kcall and cache for future
        // calls.
        let real_tid: pthread_t = __kcall_gettid()
            .expect("pthread_self(): gettid kcall failed (TDA cache-miss path)")
            .try_into()
            .expect("pthread_self(): invalid thread identifier from kernel (TDA cache-miss path)");

        unsafe {
            arch::write_tda_u32(TDA_TID_OFFSET as u32, real_tid);
        }

        return real_tid;
    }

    // Fallback: TDA not yet initialized (early startup).
    __kcall_gettid()
        .expect("pthread_self(): gettid kcall failed (early-startup path)")
        .try_into()
        .expect("pthread_self(): invalid thread identifier from kernel (early-startup path)")
}
