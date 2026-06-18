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
        __kcall_lock_mutex,
        __kcall_unlock_mutex,
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
    // Register (or re-register) the mutex's attributes.
    //
    // Initialization is idempotent: an existing entry for this address is overwritten rather than
    // rejected. This mirrors the kernel, which keeps mutexes in a per-process table and lazily
    // (re)creates them on access (`Process::get_mutex` uses `or_insert_with`). It is also what
    // makes `fork()` work: the kernel intentionally drops the child's inherited mutexes and
    // recreates them lazily, but this userspace registry is inherited through copy-on-write
    // memory, so a child that re-initializes a mutex (e.g. CPython rebuilding its GIL in
    // `PyOS_AfterFork_Child`) would otherwise observe a stale, already-registered entry and fail.
    // POSIX leaves re-initialization of an initialized mutex undefined and glibc/musl tolerate it;
    // Nanvix defines it as "reset", consistent with its lazy, kernel-backed model.
    MUTEXES
        .lock()
        .insert(mutex as *const pthread_mutex_t as usize, *attr);
    Ok(())
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
            ::syslog::warn!("pthread_mutex_destroy(): {}", reason);
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
            ::syslog::warn!("pthread_mutex_lock(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    __kcall_lock_mutex(MutexAddress::from(mutex as *const pthread_mutex_t as usize), None)
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
            ::syslog::warn!("pthread_mutex_timedlock(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    __kcall_lock_mutex(MutexAddress::from(mutex as *const pthread_mutex_t as usize), timeout)
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
            ::syslog::warn!("pthread_mutex_trylock(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    // Try to lock the mutex and parse the result.
    match __kcall_lock_mutex(
        MutexAddress::from(mutex as *const pthread_mutex_t as usize),
        Some(SystemTime::default()),
    ) {
        // Success.
        Ok(()) => Ok(()),
        // Failure.
        Err(error) => {
            // Check if we have to interpose the error.
            if error.code == ErrorCode::OperationTimedOut {
                ::syslog::warn!("pthread_mutex_trylock(): mutex is already locked");
                // Mutex is already locked.
                Err(Error::new(ErrorCode::ResourceBusy, "mutex is already locked"))
            } else {
                ::syslog::warn!(
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
            ::syslog::warn!("pthread_mutex_unlock(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    __kcall_unlock_mutex(MutexAddress::from(mutex as *const pthread_mutex_t as usize))
}
