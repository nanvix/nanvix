// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(all(target_os = "none", feature = "allocator"))]
mod allocator;
#[cfg(all(target_os = "none", feature = "allocator"))]
pub mod heap;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "none")]
use ::sys::error::Error;

//==================================================================================================
// Exports
//==================================================================================================

pub use ::arch::mem::*;

pub use ::sys::mm::*;

#[cfg(target_os = "none")]
pub use ::sys::kcall::mm::{
    mmap,
    munmap,
};

//==================================================================================================
// Constants
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(all(target_os = "none", feature = "allocator", feature = "staticlib"))] {
        /// Heap size for Rust runtime.
        const RUST_HEAP_SIZE: usize = config::memory_layout::USER_HEAP_SIZE/2;
        /// Heap size for C runtime.
        pub const C_HEAP_SIZE: usize = config::memory_layout::USER_HEAP_SIZE/2;
    } else if #[cfg(all(target_os = "none", feature = "allocator"))] {
        /// Heap size for Rust runtime.
        const RUST_HEAP_SIZE: usize = config::memory_layout::USER_HEAP_SIZE;
        /// Heap size for C runtime.
        pub const C_HEAP_SIZE: usize = 0;
    }
}

cfg_if::cfg_if! {
    if #[cfg(all(target_os = "none", feature = "allocator"))] {
        /// Based address for break address.
        pub const BREAK_BASE_RAW: usize =
            config::memory_layout::USER_HEAP_BASE_RAW + RUST_HEAP_SIZE;
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Initializes memory management runtime.
#[cfg(target_os = "none")]
pub fn init() -> Result<(), Error> {
    #[cfg(feature = "allocator")]
    {
        use crate::mm::allocator;
        use ::arch::mem;
        use ::sys::{
            config::memory_layout,
            kcall::{
                self,
            },
            pm::ProcessIdentifier,
        };

        let pid: ProcessIdentifier = kcall::pm::getpid()?;

        // Initialize the heap.
        unsafe {
            allocator::init(pid, memory_layout::USER_HEAP_BASE, mem::PAGE_SIZE, RUST_HEAP_SIZE)?
        };
    }

    Ok(())
}

/// Cleanups the memory management runtime.
#[cfg(target_os = "none")]
pub fn cleanup() -> Result<(), Error> {
    Ok(())
}
