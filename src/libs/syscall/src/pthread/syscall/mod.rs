// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

/// Condition variables.
mod cond;

/// Mutexes.
mod mutex;

/// Thread-specific data area.
mod tda;

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::collections::btree_map::BTreeMap;
use ::spin::{
    Lazy,
    Mutex,
};
use ::sys::{
    error::Error,
    kcall::pm::{
        create_thread,
        exit_thread,
        join_thread,
    },
    mm::VirtualAddress,
    pm::{
        ThreadCreateArgs,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Exports
//==================================================================================================

use ::sysapi::sys_types::{
    pthread_mutexattr_t,
    pthread_t,
};
pub use cond::*;
pub use mutex::*;
pub use tda::*;

//==================================================================================================
// Globals
//==================================================================================================

static MUTEXES: Lazy<Mutex<BTreeMap<usize, pthread_mutexattr_t>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn pthread_create(
    start_routine: extern "C" fn(usize) -> usize,
    arg: usize,
) -> Result<pthread_t, Error> {
    ::syslog::trace!(
        "pthread_create(): _start_routine={:?}, arg={:?}",
        ::core::ptr::addr_of!(start_routine),
        arg
    );

    let mut args: ThreadCreateArgs = ThreadCreateArgs {
        // Placeholder for user wrapper function, it will be overridden by the kernel call interface.
        user_wrapper_fn: VirtualAddress::from_raw_value(0),
        user_fn: VirtualAddress::from_raw_value(start_routine as usize),
        user_fn_arg: arg,
    };

    create_thread(&mut args)?.try_into()
}

pub fn pthread_join(thread: pthread_t) -> Result<usize, Error> {
    ::syslog::trace!("pthread_join(): _thread={:?}", thread);

    let mut retval: usize = 0;
    let thread: ThreadIdentifier = match thread.try_into() {
        Ok(tid) => tid,
        Err(error) => {
            ::syslog::error!("pthread_join(): {error:?} (thread={thread:?})");
            return Err(error);
        },
    };

    match join_thread(thread, &mut retval) {
        Ok(()) => Ok(retval),
        Err(error) => Err(error),
    }
}

pub fn pthread_exit(retval: usize) -> Result<!, Error> {
    ::syslog::trace!("pthread_exit(): retval={:?}", retval);
    exit_thread(retval)
}

pub fn pthread_self() -> pthread_t {
    ::sys::kcall::pm::gettid()
        .expect("a thread must be able to get its own identifier")
        .try_into()
        .expect("thread identifiers returned by the kernel must be valid")
}
