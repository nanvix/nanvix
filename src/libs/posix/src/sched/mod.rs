// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::c_int;

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

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Used to set and get scheduling parameters.
///
#[derive(Default, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct sched_param {
    /// Process or thread scheduling priority.
    pub sched_priority: c_int,
}
::nvx::sys::static_assert_size!(sched_param, sched_param::SIZE);

impl sched_param {
    /// Size of the `sched_priority` field.
    const SIZE_OF_SCHED_PRIORITY: usize = core::mem::size_of::<c_int>();

    /// Size of `sched_param` structure.
    pub const SIZE: usize = Self::SIZE_OF_SCHED_PRIORITY;
}
