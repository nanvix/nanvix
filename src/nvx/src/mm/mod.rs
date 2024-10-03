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

/// Heap size (in bytes). This value was chosen arbitrarily.
#[cfg(all(target_os = "none", feature = "allocator"))]
const HEAP_SIZE: usize = 4 * ::sys::constants::MEGABYTE;

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

            // Zero page.
            unsafe {
                core::ptr::write_bytes(vaddr as *mut u8, 0, mem::PAGE_SIZE);
            }
        }

        // Initialize the heap.
        unsafe { allocator::init(start, HEAP_SIZE)? };
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
