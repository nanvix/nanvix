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

use crate::{
    pthread::pthread_t,
    sys::types::pthread_mutexattr_t,
};
use ::alloc::collections::btree_map::BTreeMap;
use ::nvx::{
    pm::ThreadIdentifier,
    sys::{
        error::Error,
        kcall::pm::{
            create_thread,
            exit_thread,
            join_thread,
        },
    },
};
use ::spin::{
    Lazy,
    Mutex,
};

//==================================================================================================
// Exports
//==================================================================================================

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
    ::nvx::trace!(
        "pthread_create(): _start_routine={:?}, arg={:?}",
        ::core::ptr::addr_of!(start_routine),
        arg
    );
    Ok(create_thread(start_routine, arg)?.into())
}

pub fn pthread_join(thread: pthread_t) -> Result<isize, Error> {
    ::nvx::trace!("pthread_join(): _thread={:?}", thread);

    let mut retval: usize = 0;
    let thread: ThreadIdentifier = thread.into();

    match join_thread(thread, &mut retval) {
        Ok(_) => Ok(retval as isize),
        Err(error) => Err(error),
    }
}

pub fn pthread_exit(retval: usize) -> Result<!, Error> {
    ::nvx::trace!("pthread_exit(): retval={:?}", retval);
    exit_thread(retval)
}

pub fn pthread_self() -> pthread_t {
    ::nvx::pm::gettid().unwrap().into()
}
