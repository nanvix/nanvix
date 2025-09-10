// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys_types::{
    pthread_cond_t,
    pthread_mutex_t,
    pthread_rwlock_t,
    pthread_t,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Used to identify the null thread.
pub const PTHREAD_NULL: pthread_t = 0;

/// Used to initialize a condition variable statically
pub const PTHREAD_COND_INITIALIZER: pthread_cond_t = 0xffffffff;

/// Used to initialize a mutex statically.
pub const PTHREAD_MUTEX_INITIALIZER: pthread_mutex_t = 0xffffffff;

/// Type of mutex in [`crate::sys::types::pthread_mutexattr_t`].
pub mod pthread_mutex_type {
    use crate::ffi::c_int;

    /// A type of mutex that does not detect deadlock.  A thread attempting to re-lock this mutex
    /// without first unlocking it shall deadlock. Attempting to unlock a mutex locked by a
    /// different thread results in undefined behavior. Attempting to unlock an unlocked mutex
    /// results in undefined behavior.
    pub const PTHREAD_MUTEX_NORMAL: c_int = 0;

    /// A type of mutex that allows recursive locking. A thread attempting to re-lock this mutex
    /// without first unlocking it shall succeed in locking the mutex. The re-locking deadlock which
    /// can occur with mutexes of type [`PTHREAD_MUTEX_NORMAL`] cannot occur with this type of mutex.
    /// Multiple locks of this mutex shall require the same number of unlocks to release the mutex
    /// before another thread can acquire the mutex. A thread attempting to unlock a mutex which
    /// another thread has locked shall return with an error. A thread attempting to unlock an
    /// unlocked mutex shall return with an error.
    pub const PTHREAD_MUTEX_RECURSIVE: c_int = 1;

    /// A type of mutex that provides error checking. A thread attempting to re-lock this mutex
    /// without first unlocking it shall return with an error. A thread attempting to unlock a mutex
    /// which another thread has locked shall return with an error. A thread attempting to unlock an
    /// unlocked mutex shall return with an error.
    pub const PTHREAD_MUTEX_ERRORCHECK: c_int = 2;

    /// A type of mutex that provides no guarantees. Attempting to unlock a mutex of this type which
    /// was not locked by the calling thread results in undefined behavior. Attempting to unlock a
    /// mutex of this type which is not locked results in undefined behavior. An implementation may
    /// map this mutex to one of the other mutex types.
    pub const PTHREAD_MUTEX_DEFAULT: c_int = 3;
}

/// Used to initialize a read-write lock statically.
pub const PTHREAD_RWLOCK_INITIALIZER: pthread_rwlock_t = 0xffffffff;
