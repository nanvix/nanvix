// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use syscall::Pointer;
        pub use syscall::pthread_attr_destroy;
        pub use syscall::pthread_attr_init;
        pub use syscall::pthread_create;
        pub use syscall::pthread_exit;
        pub use syscall::pthread_getattr_np;
        pub use syscall::pthread_join;
        pub use syscall::pthread_self;
        pub use syscall::pthread_mutex_destroy;
        pub use syscall::pthread_mutex_init;
        pub use syscall::pthread_mutex_lock;
        pub use syscall::pthread_mutex_timedlock;
        pub use syscall::pthread_mutex_trylock;
        pub use syscall::pthread_mutex_unlock;
        pub use syscall::pthread_rwlock_init;
        pub use syscall::pthread_rwlock_destroy;
        pub use syscall::pthread_rwlock_rdlock;
        pub use syscall::pthread_rwlock_wrlock;
        pub use syscall::pthread_rwlock_unlock;
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
        pub mod bindings;
    }
}
