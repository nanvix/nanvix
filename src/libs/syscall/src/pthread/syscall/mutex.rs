// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::collections::btree_map::{
    BTreeMap,
    Entry,
};
use ::spin::{
    Lazy,
    Mutex,
    MutexGuard,
};
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
use ::sysapi::{
    pthread::PTHREAD_MUTEX_INITIALIZER,
    sys_types::{
        pthread_mutex_t,
        pthread_mutexattr_t,
    },
};

//==================================================================================================
// Global Variables
//==================================================================================================

/// Global map of mutexes for threads.
pub(super) static MUTEXES: Lazy<Mutex<BTreeMap<usize, pthread_mutexattr_t>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn pthread_mutex_init(
    mutex: &mut pthread_mutex_t,
    attr: &pthread_mutexattr_t,
) -> Result<(), Error> {
    // Check if mutex is not initialized.
    if let Entry::Vacant(entry) = MUTEXES
        .lock()
        .entry(mutex as *const pthread_mutex_t as usize)
    {
        // Mutex was is not initialized.
        entry.insert(*attr);
        Ok(())
    } else {
        let reason: &str = "mutex is not initialized";
        ::syslog::error!("pthread_mutex_init(): {}", reason);
        Err(Error::new(ErrorCode::InvalidArgument, reason))
    }
}

pub fn pthread_mutex_destroy(mutex: &mut pthread_mutex_t) -> Result<(), Error> {
    let mut mutexes: MutexGuard<'_, BTreeMap<usize, pthread_mutexattr_t>> = MUTEXES.lock();

    // Check if mutex is not initialized.
    if !mutexes.contains_key(&(mutex as *const pthread_mutex_t as usize)) {
        // Check if mutex was statically initialized.
        if *mutex == PTHREAD_MUTEX_INITIALIZER {
            // No need to remove in this case, as it was not lazily registered.
            return Ok(());
        } else {
            let reason: &str = "mutex is not initialized";
            ::syslog::error!("pthread_mutex_destroy(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    mutexes.remove(&(mutex as *const pthread_mutex_t as usize));

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
            ::syslog::error!("pthread_mutex_unlock(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    unlock_mutex(MutexAddress::from(mutex as *const pthread_mutex_t as usize))
}
