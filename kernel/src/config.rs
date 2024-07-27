// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Description
//!
//! Kernel-wide configuration constants.
//!

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::mem,
    klib,
};

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Total size of physical memory (in bytes).
///
pub const MEMORY_SIZE: usize = 256 * klib::MEGABYTE;

///
/// # Description
///
/// Total size of the kernel pool (in bytes).
///
pub const KPOOL_SIZE: usize = 4 * klib::MEGABYTE;

///
/// # Description
///
/// Kernel stack size (in bytes).
///
pub const KSTACK_SIZE: usize = 8 * mem::PAGE_SIZE;

///
/// # Description
///
/// Timer frequency (in Hz).
///
pub const TIMER_FREQ: u32 = 100;

///
/// # Description
///
/// Scheduler frequency (in ticks).
///
pub const SCHEDULER_FREQ: usize = 128;
