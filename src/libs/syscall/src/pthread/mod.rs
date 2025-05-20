// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    sys::types::pthread_cond_t,
};
use ::core::mem;

//==================================================================================================
// Modules
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use syscall::pthread_create;
        pub use syscall::pthread_exit;
        pub use syscall::pthread_join;
        pub use syscall::pthread_self;
        pub use syscall::pthread_mutex_destroy;
        pub use syscall::pthread_mutex_init;
        pub use syscall::pthread_mutex_lock;
        pub use syscall::pthread_mutex_timedlock;
        pub use syscall::pthread_mutex_trylock;
        pub use syscall::pthread_mutex_unlock;
        pub use syscall::pthread_cond_broadcast;
        pub use syscall::pthread_cond_destroy;
        pub use syscall::pthread_cond_init;
        pub use syscall::pthread_cond_signal;
        pub use syscall::pthread_cond_timedwait;
        pub use syscall::pthread_cond_wait;
        pub use syscall::pthread_key_create;
        pub use syscall::pthread_key_delete;
        pub use syscall::pthread_setspecific;
        pub use syscall::pthread_getspecific;
    }
}

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use crate::sys::types::{
    pthread_attr_t,
    pthread_mutex_t,
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
    use super::*;

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

///
/// # Description
///
/// Mutex attributes.
///
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct pthread_mutexattr_t {
    /// Whether the mutex attributes are initialized.
    is_initialized: c_int,
    /// Type of mutex.
    type_: c_int,
    /// Whether the mutex is recursive.
    recursive: c_int,
}
::static_assert::assert_eq_size!(pthread_mutexattr_t, pthread_mutexattr_t::SIZE);

impl pthread_mutexattr_t {
    /// Size of the `is_initialized` field.
    const SIZE_OF_IS_INITIALIZED: usize = mem::size_of::<c_int>();
    /// Size of the `type_` field.
    const SIZE_OF_TYPE: usize = mem::size_of::<c_int>();
    /// Size of the `recursive` field.
    const SIZE_OF_RECURSIVE: usize = mem::size_of::<c_int>();

    /// Size of `pthread_mutexattr_t` structure.
    pub const SIZE: usize =
        Self::SIZE_OF_IS_INITIALIZED + Self::SIZE_OF_TYPE + Self::SIZE_OF_RECURSIVE;
}

impl Default for pthread_mutexattr_t {
    fn default() -> Self {
        Self {
            is_initialized: 1,
            type_: pthread_mutex_type::PTHREAD_MUTEX_NORMAL,
            recursive: 0,
        }
    }
}
