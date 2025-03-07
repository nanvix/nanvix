// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    pthread::{
        pthread_t,
        PTHREAD_MUTEX_INITIALIZER,
    },
    sys::types::{
        pthread_mutex_t,
        pthread_mutexattr_t,
    },
};
use ::alloc::collections::btree_map::{
    BTreeMap,
    Entry,
};
use ::nvx::{
    mm::VirtualAddress,
    pm::ThreadIdentifier,
    sys::{
        error::{
            Error,
            ErrorCode,
        },
        kcall::pm::{
            create_thread,
            exit_thread,
            join_thread,
            lock_mutex,
            unlock_mutex,
        },
    },
};
use ::spin::{
    Lazy,
    Mutex,
};

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
    ::nvx::trace!("pthread_self()");
    ::nvx::pm::gettid().unwrap().into()
}

pub fn pthread_mutex_init(
    mutex: &mut pthread_mutex_t,
    attr: &pthread_mutexattr_t,
) -> Result<(), Error> {
    ::nvx::trace!("pthread_mutex_init(): mutex={:?}, attr={:?}", mutex, attr);

    // Check if mutex is already initialized.
    if MUTEXES
        .lock()
        .contains_key(&(mutex as *const pthread_mutex_t as usize))
    {
        let reason: &str = "mutex is already initialized";
        ::nvx::error!("pthread_mutex_init(): {}", reason);
        return Err(Error::new(ErrorCode::ResourceBusy, reason));
    }

    MUTEXES
        .lock()
        .insert(mutex as *const pthread_mutex_t as usize, *attr);

    Ok(())
}

pub fn pthread_mutex_destroy(mutex: &mut pthread_mutex_t) -> Result<(), Error> {
    ::nvx::trace!("pthread_mutex_destroy(): mutex={:?}", mutex);

    // Check if mutex is not initialized.
    if !MUTEXES
        .lock()
        .contains_key(&(mutex as *const pthread_mutex_t as usize))
    {
        // Check if mutex was statically initialized.
        if *mutex == PTHREAD_MUTEX_INITIALIZER {
            // No ned to remove in this case, as it was not lazily registered.
            return Ok(());
        } else {
            // Check if mutex was statically initialized.
            let reason: &str = "mutex is not initialized";
            ::nvx::error!("pthread_mutex_destroy(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    MUTEXES
        .lock()
        .remove(&(mutex as *const pthread_mutex_t as usize));

    Ok(())
}

pub fn pthread_mutex_lock(mutex: &mut pthread_mutex_t) -> Result<(), Error> {
    ::nvx::trace!("pthread_mutex_lock(): mutex={:p}", mutex as *mut pthread_mutex_t);

    if let Entry::Vacant(entry) = MUTEXES
        .lock()
        .entry(mutex as *const pthread_mutex_t as usize)
    {
        // Check if mutex was statically initialized.
        if *mutex == PTHREAD_MUTEX_INITIALIZER {
            // Lazily register mutex.
            entry.insert(pthread_mutexattr_t::default());
        } else {
            let reason: &str = "mutex is not initialized";
            ::nvx::error!("pthread_mutex_lock(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    lock_mutex(VirtualAddress::from_raw_value(mutex as *const pthread_mutex_t as usize))
}

pub fn pthread_mutex_unlock(mutex: &mut pthread_mutex_t) -> Result<(), Error> {
    ::nvx::trace!("pthread_mutex_unlock(): mutex={:p}", mutex as *mut pthread_mutex_t);

    // Check if mutex is not initialized.
    if *mutex != PTHREAD_MUTEX_INITIALIZER
        && !MUTEXES
            .lock()
            .contains_key(&(mutex as *const pthread_mutex_t as usize))
    {
        // Check if mutex was statically initialized.
        if *mutex == PTHREAD_MUTEX_INITIALIZER {
            // Lazily register mutex.
            MUTEXES
                .lock()
                .insert(mutex as *const pthread_mutex_t as usize, pthread_mutexattr_t::default());
        } else {
            let reason: &str = "mutex is not initialized";
            ::nvx::error!("pthread_mutex_unlock(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    unlock_mutex(VirtualAddress::from_raw_value(mutex as *const pthread_mutex_t as usize))
}
