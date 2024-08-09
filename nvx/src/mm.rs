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
// Standalone Functions
//==================================================================================================

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

        const HEAP_SIZE: usize = ::kcall::sys::constants::MEGABYTE;

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
