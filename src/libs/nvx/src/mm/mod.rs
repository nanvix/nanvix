// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(all(target_os = "none", feature = "allocator"))]
mod allocator;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "none")]
use ::sys::error::Error;

//==================================================================================================
// Exports
//==================================================================================================

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
    if #[cfg(all(target_os = "none", feature = "allocator"))] {
        /// Heap size (in bytes). This value was chosen arbitrarily.
        pub const HEAP_SIZE: usize = 8 * config::constants::MEGABYTE;
        /// Based address for break address.
        pub const BREAK_BASE_RAW: usize = config::memory_layout::USER_HEAP_BASE_RAW + HEAP_SIZE/2;
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
        use ::sys::{
            arch::mem,
            config::memory_layout,
            kcall::{
                self,
            },
            pm::{
                Capability,
                ProcessIdentifier,
            },
        };

        let pid: ProcessIdentifier = kcall::pm::getpid()?;

        // Acquire memory management capability.
        kcall::pm::capctl(Capability::MemoryManagement, true)?;

        // Map underlying pages for the heap.
        let start: usize = memory_layout::USER_HEAP_BASE.into_raw_value();
        let end: usize = start + HEAP_SIZE;
        for vaddr in (start..end).step_by(mem::PAGE_SIZE) {
            kcall::mm::mmap(pid, VirtualAddress::from_raw_value(vaddr)?, AccessPermission::RDWR)?;

            // NOTE: pages allocated with mmap() are always zeroed.
        }

        // Initialize the heap.
        unsafe { allocator::init(start, HEAP_SIZE / 2)? };
    }

    Ok(())
}

/// Cleanups the memory management runtime.
#[cfg(target_os = "none")]
pub fn cleanup() -> Result<(), Error> {
    #[cfg(feature = "allocator")]
    {
        use ::sys::{
            arch::mem,
            config::memory_layout,
            kcall::{
                self,
            },
            pm::{
                Capability,
                ProcessIdentifier,
            },
        };

        let pid: ProcessIdentifier = kcall::pm::getpid()?;

        // Unmap underlying pages for the heap.
        let start: usize = memory_layout::USER_HEAP_BASE.into_raw_value();
        let end: usize = start + HEAP_SIZE;
        for vaddr in (start..end).step_by(mem::PAGE_SIZE) {
            kcall::mm::munmap(pid, VirtualAddress::from_raw_value(vaddr)?)?;
        }

        // Release memory management capability.
        kcall::pm::capctl(Capability::MemoryManagement, false)?;
    }
    Ok(())
}
