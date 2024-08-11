// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::error::Error;

//==================================================================================================
// Exports
//==================================================================================================

pub use ::kcall::mm::*;

//==================================================================================================
// Constants
//==================================================================================================

/// Heap size (in bytes).
#[cfg(feature = "slab-allocator")]
const HEAP_SIZE: usize = ::kcall::sys::constants::MEGABYTE;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Initializes memory management runtime.
pub fn init() -> Result<(), Error> {
    #[cfg(feature = "slab-allocator")]
    {
        use ::allocator::Heap;
        use ::kcall::{
            arch::mem,
            mm,
            pm::ProcessIdentifier,
            sys::config::memory_layout,
        };

        let pid: ProcessIdentifier = kcall::pm::getpid()?;

        // Acquire memory management capability.
        ::kcall::pm::capctl(kcall::pm::Capability::MemoryManagement, true)?;

        // Map underlying pages for the heap.
        let start: usize = memory_layout::USER_HEAP_BASE.into_raw_value();
        let end: usize = start + HEAP_SIZE;
        for vaddr in (start..end).step_by(mem::PAGE_SIZE) {
            ::kcall::mm::mmap(
                pid,
                VirtualAddress::from_raw_value(vaddr)?,
                mm::AccessPermission::RDWR,
            )?;

            // Zero page.
            unsafe {
                core::ptr::write_bytes(vaddr as *mut u8, 0, mem::PAGE_SIZE);
            }
        }

        // Initialize the heap.
        unsafe { Heap::init(start, HEAP_SIZE)? };
    }

    Ok(())
}

/// Cleanups the memory management runtime.
pub fn cleanup() -> Result<(), Error> {
    #[cfg(feature = "slab-allocator")]
    {
        use ::kcall::{
            arch::mem,
            pm::ProcessIdentifier,
            sys::config::memory_layout,
        };

        let pid: ProcessIdentifier = kcall::pm::getpid()?;

        // Unmap underlying pages for the heap.
        let start: usize = memory_layout::USER_HEAP_BASE.into_raw_value();
        let end: usize = start + HEAP_SIZE;
        for vaddr in (start..end).step_by(mem::PAGE_SIZE) {
            ::kcall::mm::munmap(pid, VirtualAddress::from_raw_value(vaddr)?)?;
        }

        // Release memory management capability.
        ::kcall::pm::capctl(kcall::pm::Capability::MemoryManagement, false)?;
    }
    Ok(())
}
