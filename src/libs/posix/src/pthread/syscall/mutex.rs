// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    pthread::{
        syscall::MUTEXES,
        PTHREAD_MUTEX_INITIALIZER,
    },
    sys::types::{
        pthread_mutex_t,
        pthread_mutexattr_t,
    },
};
use ::alloc::collections::btree_map::Entry;
use ::nvx::{
    pm::MutexAddress,
    sys::{
        error::{
            Error,
            ErrorCode,
        },
        kcall::pm::{
            lock_mutex,
            unlock_mutex,
        },
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

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

    lock_mutex(MutexAddress::from(mutex as *const pthread_mutex_t as usize))
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

    unlock_mutex(MutexAddress::from(mutex as *const pthread_mutex_t as usize))
}
