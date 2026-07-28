// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod input_message;
pub mod message;
pub mod socket_message;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use syscall::poll;
        pub mod bindings;
    }
}
