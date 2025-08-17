// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pthread::syscall::mutex::MUTEXES;
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
        signal_cond,
        wait_cond,
    },
    pm::{
        ConditionAddress,
        MutexAddress,
    },
    time::SystemTime,
};
use ::sysapi::{
    pthread::PTHREAD_COND_INITIALIZER,
    sys_types::pthread_cond_t,
};
use sysapi::{
    pthread::PTHREAD_MUTEX_INITIALIZER,
    sys_types::{
        pthread_condattr_t,
        pthread_mutex_t,
        pthread_mutexattr_t,
    },
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
            ::syslog::error!("pthread_cond_broadcast(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    let _awakened: usize =
        signal_cond(ConditionAddress::from(cond as *const pthread_cond_t as usize), true)?;

    Ok(())
}

pub fn pthread_cond_init(
    cond: &mut pthread_cond_t,
    attr: &pthread_condattr_t,
) -> Result<(), Error> {
    // Check if condition variable is not initialized.
    if let Entry::Vacant(entry) = CONDITIONS
        .lock()
        .entry(cond as *const pthread_cond_t as usize)
    {
        // Condition variable is not initialized.
        entry.insert(*attr);
        Ok(())
    } else {
        // Condition variable is already initialized.
        let reason: &str = "condition variable is already initialized";
        ::syslog::error!("pthread_cond_init(): {}", reason);
        Err(Error::new(ErrorCode::ResourceBusy, reason))
    }
}

pub fn pthread_cond_destroy(cond: &mut pthread_cond_t) -> Result<(), Error> {
    let mut conditions: MutexGuard<'_, BTreeMap<usize, pthread_condattr_t>> = CONDITIONS.lock();

    // Check if condition variable is not initialized.
    if !conditions.contains_key(&(cond as *const pthread_cond_t as usize)) {
        // Check if condition variable was statically initialized.
        if *cond == PTHREAD_COND_INITIALIZER {
            // No need to remove in this case, as it was not lazily registered.
            return Ok(());
        } else {
            let reason: &str = "condition variable is not initialized";
            ::syslog::error!("pthread_cond_destroy(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    conditions.remove(&(cond as *const pthread_cond_t as usize));

    Ok(())
}

pub fn pthread_cond_signal(cond: &pthread_cond_t) -> Result<(), Error> {
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
            ::syslog::error!("pthread_cond_signal(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    let _awakened: usize =
        signal_cond(ConditionAddress::from(cond as *const pthread_cond_t as usize), false)?;

    Ok(())
}

pub fn pthread_cond_timedwait(
    cond: &pthread_cond_t,
    mutex: &pthread_mutex_t,
    timeout: Option<SystemTime>,
) -> Result<(), Error> {
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
            ::syslog::error!("pthread_wait_cond(): {}", reason);
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
            ::syslog::error!("pthread_wait_cond(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    wait_cond(
        ConditionAddress::from(cond as *const pthread_cond_t as usize),
        MutexAddress::from(mutex as *const pthread_mutex_t as usize),
        timeout,
    )
}

pub fn pthread_cond_wait(cond: &pthread_cond_t, mutex: &pthread_mutex_t) -> Result<(), Error> {
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
            ::syslog::error!("pthread_wait_cond(): {}", reason);
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
            ::syslog::error!("pthread_wait_cond(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    wait_cond(
        ConditionAddress::from(cond as *const pthread_cond_t as usize),
        MutexAddress::from(mutex as *const pthread_mutex_t as usize),
        None,
    )
}
