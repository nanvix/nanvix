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
};
use ::sys::mm::VirtualAddress;

//==================================================================================================
// Modules
//==================================================================================================

pub mod mmap;
pub mod mprotect;
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
