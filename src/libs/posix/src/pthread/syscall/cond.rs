// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    pthread::{
        syscall::MUTEXES,
        PTHREAD_COND_INITIALIZER,
        PTHREAD_MUTEX_INITIALIZER,
    },
    sys::types::{
        pthread_cond_t,
        pthread_condattr_t,
        pthread_mutex_t,
        pthread_mutexattr_t,
    },
};
use ::alloc::collections::btree_map::{
    BTreeMap,
    Entry,
};
use ::nvx::{
    pm::{
        ConditionAddress,
        MutexAddress,
    },
    sys::{
        error::{
            Error,
            ErrorCode,
        },
        kcall::pm::{
            signal_cond,
            wait_cond,
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

static CONDITIONS: Lazy<Mutex<BTreeMap<usize, pthread_condattr_t>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn pthread_cond_broadcast(cond: &pthread_cond_t) -> Result<(), Error> {
    ::nvx::trace!("pthread_cond_broadcast(): cond={:p}", cond);

    // Check if condition variable is not initialized.
    if let Entry::Vacant(entry) = CONDITIONS
        .lock()
        .entry(cond as *const pthread_cond_t as usize)
    {
        // Check if condition variable was statically initialized.
        if *cond == PTHREAD_COND_INITIALIZER {
            // Lazily register condition variable.
            entry.insert(pthread_condattr_t::default());
        } else {
            let reason: &str = "condition variable is not initialized";
            ::nvx::error!("pthread_cond_broadcast(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    signal_cond(ConditionAddress::from(cond as *const pthread_cond_t as usize), true)
}

pub fn pthread_cond_init(
    cond: &mut pthread_cond_t,
    attr: &pthread_condattr_t,
) -> Result<(), Error> {
    ::nvx::trace!("pthread_cond_init(): cond={:p}, attr={:p}", cond, attr);

    // Check if condition variable is already initialized.
    if CONDITIONS
        .lock()
        .contains_key(&(cond as *const pthread_cond_t as usize))
    {
        let reason: &str = "condition variable is already initialized";
        ::nvx::error!("pthread_cond_init(): {}", reason);
        return Err(Error::new(ErrorCode::ResourceBusy, reason));
    }

    CONDITIONS
        .lock()
        .insert(cond as *const pthread_cond_t as usize, *attr);

    Ok(())
}

pub fn pthread_cond_destroy(cond: &mut pthread_cond_t) -> Result<(), Error> {
    ::nvx::trace!("pthread_cond_destroy(): cond={:p}", cond);

    // Check if condition variable is not initialized.
    if !CONDITIONS
        .lock()
        .contains_key(&(cond as *const pthread_cond_t as usize))
    {
        // Check if condition variable was statically initialized.
        if *cond == PTHREAD_COND_INITIALIZER {
            // No ned to remove in this case, as it was not lazily registered.
            return Ok(());
        } else {
            let reason: &str = "condition variable is not initialized";
            ::nvx::error!("pthread_cond_destroy(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    CONDITIONS
        .lock()
        .remove(&(cond as *const pthread_cond_t as usize));

    Ok(())
}

pub fn pthread_cond_signal(cond: &pthread_cond_t) -> Result<(), Error> {
    ::nvx::trace!("pthread_cond_signal(): cond={:p}", cond);

    // Check if condition variable is not initialized.
    if let Entry::Vacant(_) = CONDITIONS
        .lock()
        .entry(cond as *const pthread_cond_t as usize)
    {
        // Check if condition variable was statically initialized.
        if *cond == PTHREAD_COND_INITIALIZER {
            // Lazily register condition variable.
            CONDITIONS
                .lock()
                .insert(cond as *const pthread_cond_t as usize, pthread_condattr_t::default());
        } else {
            let reason: &str = "condition variable is not initialized";
            ::nvx::error!("pthread_cond_signal(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    signal_cond(ConditionAddress::from(cond as *const pthread_cond_t as usize), false)
}

pub fn pthread_cond_wait(cond: &pthread_cond_t, mutex: &pthread_mutex_t) -> Result<(), Error> {
    ::nvx::trace!("pthread_wait_cond(): cond={:p}, mutex={:p}", cond, mutex);

    // Check if condition variable is not initialized.
    if let Entry::Vacant(entry) = CONDITIONS
        .lock()
        .entry(cond as *const pthread_cond_t as usize)
    {
        // Check if condition variable was statically initialized.
        if *cond == PTHREAD_COND_INITIALIZER {
            // Lazily register condition variable.
            entry.insert(pthread_condattr_t::default());
        } else {
            let reason: &str = "condition variable is not initialized";
            ::nvx::error!("pthread_wait_cond(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    // Check if mutex is not initialized.
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
            ::nvx::error!("pthread_wait_cond(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    wait_cond(
        ConditionAddress::from(cond as *const pthread_cond_t as usize),
        MutexAddress::from(mutex as *const pthread_mutex_t as usize),
    )
}
