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

/// Scheduling policies.
pub mod sched_policy {
    /// Another scheduling policy.
    pub const SCHED_OTHER: crate::ffi::c_int = 0;

    /// FIFO scheduling policy.
    pub const SCHED_FIFO: crate::ffi::c_int = 1;

    /// Round-robin scheduling policy.
    pub const SCHED_RR: crate::ffi::c_int = 2;
}

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
::static_assert::assert_eq_size!(sched_param, sched_param::SIZE);

impl sched_param {
    /// Size of the `sched_priority` field.
    const SIZE_OF_SCHED_PRIORITY: usize = core::mem::size_of::<c_int>();

    /// Size of `sched_param` structure.
    pub const SIZE: usize = Self::SIZE_OF_SCHED_PRIORITY;
}
