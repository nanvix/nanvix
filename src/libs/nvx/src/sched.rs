// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(target_os = "none")]
pub use ::sys::kcall::sched::sched_yield;
