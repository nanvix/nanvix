// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
#![cfg_attr(not(feature = "rustc-dep-of-std"), feature(allocator_api))]
#![no_std]

//==================================================================================================
// Modules
//==================================================================================================

mod allocator;
mod heap;

/// Thread data area.
pub mod tda;

/// Virtual address space allocator for the unified mmap region.
pub mod vaddr;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

//==================================================================================================
// Exports
//==================================================================================================

pub use ::arch::mem::*;

pub use ::sys::mm::*;

pub use ::sys::kcall::mm::{
    mmap,
    munmap,
};

pub use allocator::*;
pub use heap::{
    map_range,
    unmap_range,
};

#[cfg(feature = "rustc-dep-of-std")]
pub use allocator::{
    alloc,
    dealloc,
};
