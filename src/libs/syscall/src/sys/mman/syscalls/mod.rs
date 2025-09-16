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
use ::sys::{
    config::memory_layout::USER_MMAP_BASE,
    mm::VirtualAddress,
};

//==================================================================================================
// Modules
//==================================================================================================

pub mod mmap;
pub mod mprotect;
pub mod munmap;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Base virtual address for dynamic libraries.
/// TODO (#802): replace with a proper memory allocator.
static MMAP_BASE: Lazy<Mutex<VirtualAddress>> = Lazy::new(|| Mutex::new(USER_MMAP_BASE));

/// Map of memory segments that are currently mapped, keyed by base address.
static MMAP_SEGMENTS: Lazy<Mutex<BTreeMap<VirtualAddress, MemorySegment>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));
