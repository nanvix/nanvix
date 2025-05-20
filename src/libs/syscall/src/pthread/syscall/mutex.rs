// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    pthread::{
        pthread_mutexattr_t,
        syscall::MUTEXES,
        PTHREAD_MUTEX_INITIALIZER,
    },
    sys::types::pthread_mutex_t,
};
use ::alloc::collections::btree_map::Entry;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::pm::{
        lock_mutex,
        unlock_mutex,
    },
    pm::MutexAddress,
    time::SystemTime,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn pthread_mutex_init(
    mutex: &mut pthread_mutex_t,
    attr: &pthread_mutexattr_t,
) -> Result<(), Error> {
    // Check if mutex is already initialized.
    if MUTEXES
        .lock()
        .contains_key(&(mutex as *const pthread_mutex_t as usize))
    {
        let reason: &str = "mutex is already initialized";
        ::syslog::error!("pthread_mutex_init(): {}", reason);
        return Err(Error::new(ErrorCode::ResourceBusy, reason));
    }

    MUTEXES
        .lock()
        .insert(mutex as *const pthread_mutex_t as usize, *attr);

    Ok(())
}

pub fn pthread_mutex_destroy(mutex: &mut pthread_mutex_t) -> Result<(), Error> {
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
            ::syslog::error!("pthread_mutex_destroy(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    MUTEXES
        .lock()
        .remove(&(mutex as *const pthread_mutex_t as usize));

    Ok(())
}

pub fn pthread_mutex_lock(mutex: &mut pthread_mutex_t) -> Result<(), Error> {
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
            ::syslog::error!("pthread_mutex_lock(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    lock_mutex(MutexAddress::from(mutex as *const pthread_mutex_t as usize), None)
}

pub fn pthread_mutex_timedlock(
    mutex: &mut pthread_mutex_t,
    timeout: Option<SystemTime>,
) -> Result<(), Error> {
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
            ::syslog::error!("pthread_mutex_timedlock(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    lock_mutex(MutexAddress::from(mutex as *const pthread_mutex_t as usize), timeout)
}

pub fn pthread_mutex_trylock(mutex: &mut pthread_mutex_t) -> Result<(), Error> {
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
            ::syslog::error!("pthread_mutex_trylock(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    // Try to lock the mutex and parse the result.
    match lock_mutex(
        MutexAddress::from(mutex as *const pthread_mutex_t as usize),
        Some(SystemTime::default()),
    ) {
        // Success.
        Ok(()) => Ok(()),
        // Failure.
        Err(error) => {
            // Check if we have to interpose the error.
            if error.code == ErrorCode::OperationTimedOut {
                ::syslog::error!("pthread_mutex_trylock(): mutex is already locked");
                // Mutex is already locked.
                Err(Error::new(ErrorCode::ResourceBusy, "mutex is already locked"))
            } else {
                ::syslog::error!(
                    "pthread_mutex_trylock(): failed to lock mutex (error={:?})",
                    error
                );
                // Some other error occurred.
                Err(error)
            }
        },
    }
}

pub fn pthread_mutex_unlock(mutex: &mut pthread_mutex_t) -> Result<(), Error> {
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
            ::syslog::error!("pthread_mutex_unlock(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    unlock_mutex(MutexAddress::from(mutex as *const pthread_mutex_t as usize))
}
