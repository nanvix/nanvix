// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    ffi::{
        c_int,
        c_uint,
    },
    pthread::{
        PTHREAD_COND_INITIALIZER,
        PTHREAD_MUTEX_INITIALIZER,
    },
    sys_types::{
        pthread_condattr_t,
        pthread_mutexattr_t,
        sem_t,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes an unnamed semaphore built on top of the kernel mutex and condition variable
/// primitives.
///
/// # Parameters
///
/// - `sem`: Semaphore to initialize.
/// - `value`: Initial value of the semaphore.
///
/// # Returns
///
/// Upon success, an empty result is returned. Upon failure, an error is returned instead.
///
pub fn sem_init(sem: &mut sem_t, value: c_uint) -> Result<(), Error> {
    // Reject values that cannot be represented by the semaphore counter.
    if value > c_int::MAX as c_uint {
        let reason: &str = "semaphore value exceeds maximum";
        ::syslog::warn!("sem_init(): {} (value={})", reason, value);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Initialize the semaphore state. The `lock` and `cond` words are set to their static
    // initializer sentinels so that they are recognized as initialized even along the lazy
    // registration path (e.g. after `fork()`).
    sem.count = value as c_int;
    sem.lock = PTHREAD_MUTEX_INITIALIZER;
    sem.cond = PTHREAD_COND_INITIALIZER;

    // Register the internal mutex and condition variable, keyed by their addresses.
    super::pthread_mutex_init(&mut sem.lock, &pthread_mutexattr_t::default())?;
    super::pthread_cond_init(&mut sem.cond, &pthread_condattr_t::default())?;

    Ok(())
}

///
/// # Description
///
/// Destroys an unnamed semaphore, releasing its internal mutex and condition variable.
///
/// # Parameters
///
/// - `sem`: Semaphore to destroy.
///
/// # Returns
///
/// Upon success, an empty result is returned. Upon failure, an error is returned instead.
///
pub fn sem_destroy(sem: &mut sem_t) -> Result<(), Error> {
    super::pthread_cond_destroy(&mut sem.cond)?;
    super::pthread_mutex_destroy(&mut sem.lock)?;
    Ok(())
}

///
/// # Description
///
/// Unlocks a semaphore, incrementing its value and waking up a blocked waiter, if any.
///
/// # Parameters
///
/// - `sem`: Semaphore to unlock.
///
/// # Returns
///
/// Upon success, an empty result is returned. Upon failure, an error is returned instead.
///
pub fn sem_post(sem: &mut sem_t) -> Result<(), Error> {
    super::pthread_mutex_lock(&mut sem.lock)?;

    // Refuse to overflow the semaphore value.
    if sem.count == c_int::MAX {
        let _ = super::pthread_mutex_unlock(&mut sem.lock);
        let reason: &str = "semaphore value would overflow";
        ::syslog::warn!("sem_post(): {}", reason);
        return Err(Error::new(ErrorCode::ValueOverflow, reason));
    }

    // Increment the value and wake up a single waiter.
    sem.count += 1;
    let signal_result: Result<(), Error> = super::pthread_cond_signal(&sem.cond);

    let unlock_result: Result<(), Error> = super::pthread_mutex_unlock(&mut sem.lock);

    signal_result.and(unlock_result)
}

///
/// # Description
///
/// Locks a semaphore, blocking the calling thread until its value becomes positive and then
/// decrementing it.
///
/// # Parameters
///
/// - `sem`: Semaphore to lock.
///
/// # Returns
///
/// Upon success, an empty result is returned. Upon failure, an error is returned instead.
///
pub fn sem_wait(sem: &mut sem_t) -> Result<(), Error> {
    super::pthread_mutex_lock(&mut sem.lock)?;

    // Wait until the semaphore value is positive.
    while sem.count == 0 {
        if let Err(error) = super::pthread_cond_wait(&sem.cond, &sem.lock) {
            let _ = super::pthread_mutex_unlock(&mut sem.lock);
            return Err(error);
        }
    }

    // Acquire the semaphore.
    sem.count -= 1;

    super::pthread_mutex_unlock(&mut sem.lock)
}
