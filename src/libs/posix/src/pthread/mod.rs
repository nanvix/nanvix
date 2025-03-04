// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

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
        pub use syscall::pthread_mutex_init;
        pub use syscall::pthread_mutex_lock;
        pub use syscall::pthread_mutex_unlock;
    }
}

#[cfg(all(feature = "syscall", feature = "staticlib"))]
pub mod bindings;

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use crate::sys::types::{
    pthread_attr_t,
    pthread_mutex_t,
    pthread_mutexattr_t,
    pthread_t,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Used to identify the null thread.
pub const PTHREAD_NULL: pthread_t = 0;

/// Used to initialize a mutex statically.
pub const PTHREAD_MUTEX_INITIALIZER: pthread_mutex_t = 0xffffffff;
