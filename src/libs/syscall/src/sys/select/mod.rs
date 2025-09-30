// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

/// Messages for the `select` module.
pub mod message;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        /// Syscall interface for the `select` module.
        mod syscall;

        /// Bindings for the `select` module.
        pub mod bindings;
    }
}
