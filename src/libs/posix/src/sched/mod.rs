// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use self::syscall::{
            sched_yield,
        };
    }
}

#[cfg(all(feature = "syscall", feature = "staticlib"))]
pub mod bindings;
