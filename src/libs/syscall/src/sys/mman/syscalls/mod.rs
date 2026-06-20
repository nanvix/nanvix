// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::mem::segment::MemorySegment;
use ::alloc::collections::BTreeMap;
use ::spin::{
    Lazy,
    Mutex,
    MutexGuard,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        Address,
        VirtualAddress,
    },
};

//==================================================================================================
// Modules
//==================================================================================================

pub mod mlock;
pub mod mmap;
pub mod mprotect;
pub mod munlock;
pub mod munmap;

//==================================================================================================
// Re-exports
//==================================================================================================

/// Re-export the unified virtual address space reservation from `sysalloc`.
pub use ::sysalloc::vaddr::reserve as mmap_reserve;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Map of memory segments that are currently mapped, keyed by base address.
static MMAP_SEGMENTS: Lazy<Mutex<BTreeMap<VirtualAddress, MemorySegment>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn validate_mapped_lock_range(
    name: &'static str,
    addr: VirtualAddress,
    length: usize,
) -> Result<(), Error> {
    let segments: MutexGuard<'_, BTreeMap<VirtualAddress, MemorySegment>> = MMAP_SEGMENTS.lock();
    let mapped: bool = crate::sys::mman::bindings::is_lock_range_mapped(
        addr.into_raw_value(),
        length,
        segments
            .iter()
            .map(|(segment_base, segment)| (segment_base.into_raw_value(), segment.capacity())),
    );

    if mapped {
        Ok(())
    } else {
        let reason: &'static str = "memory range is not mapped";
        ::syslog::warn!("{name}(): {reason} (addr={addr:?}, length={length})");
        Err(Error::new(ErrorCode::OutOfMemory, reason))
    }
}
