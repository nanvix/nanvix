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
    pm::ThreadIdentifier,
};

//==================================================================================================
// Exports
//==================================================================================================

use ::sysapi::{
    pthread::pthread_mutexattr_t,
    sys_types::pthread_t,
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
    Ok(create_thread(start_routine, arg)?.into())
}

pub fn pthread_join(thread: pthread_t) -> Result<isize, Error> {
    ::syslog::trace!("pthread_join(): _thread={:?}", thread);

    let mut retval: usize = 0;
    let thread: ThreadIdentifier = thread.into();

    match join_thread(thread, &mut retval) {
        Ok(_) => Ok(retval as isize),
        Err(error) => Err(error),
    }
}

pub fn pthread_exit(retval: usize) -> Result<!, Error> {
    ::syslog::trace!("pthread_exit(): retval={:?}", retval);
    exit_thread(retval)
}

pub fn pthread_self() -> pthread_t {
    ::sys::kcall::pm::gettid().unwrap().into()
}
