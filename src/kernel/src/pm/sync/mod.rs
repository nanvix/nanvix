// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod condvar;
pub mod mutex;
pub mod semaphore;

#[cfg(feature = "smp")]
pub mod spinlock;

#[cfg(feature = "smp")]
pub mod fence;
