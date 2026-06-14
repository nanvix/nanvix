// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use vstd::prelude::*;

pub(crate) mod frame;
mod kframe;
mod manager;
mod upool;

#[cfg(feature = "test")]
mod test;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::{
        Address,
        PageAligned,
        PhysicalAddress,
        TruncatedMemoryRegion,
        VirtualAddress,
    },
    mm::phys::upool::Upool,
};
use ::alloc::collections::LinkedList;
use ::arch::mem;
use ::bitmap::Bitmap;
use ::sys::error::Error;
#[cfg(verus_keep_ghost)]
use ::vstd::prelude::*;

// Include specifications.
#[cfg(verus_keep_ghost)]
include!("mod.spec.rs");
#[cfg(verus_keep_ghost)]
include!("mod.proof.rs");

//==================================================================================================
// Exports
//==================================================================================================

pub use self::{
    kframe::KernelFrame,
    manager::PhysMemoryManager,
    upool::UserFrame,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        phys_view().initialized,
        phys_view().inv(),
    ensures
        phys_view().inv(),
        phys_view().initialized,
        match result {
            Ok(_) => phys_view().frames.all_reserved(
                phys_regions_frame_set(&physical_memory_regions)),
            Err(_) => true,
        },
)]
fn book_physical_memory_regions(
    physical_memory_regions: LinkedList<TruncatedMemoryRegion<PhysicalAddress>>,
) -> Result<(), Error> { ... }

#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        phys_view().initialized,
        phys_view().inv(),
    ensures
        phys_view().inv(),
        phys_view().initialized,
        match result {
            Ok(_) => forall|a: int|
                #[trigger] mmio_regions_frame_set(mmio_regions).contains(a)
                    && phys_view().frames.covers(a)
                    ==> phys_view().frames.reserved(a),
            Err(_) => true,
        },
)]
fn book_mmio_regions(
    mmio_regions: &LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
) -> Result<(), Error> { ... }

///
/// # Description
///
/// Initializes the physical memory manager.
///
/// # Parameters
///
/// - `physical_memory_regions`: Physical memory regions to book.
/// - `mmio_regions`: Memory-mapped I/O regions to book.
/// - `physical_memory_layout`: Physical memory layout bitmap.
///
/// # Returns
///
/// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
///
#[verus_spec(result =>
    requires
        phys_view().inv(),
    ensures
        phys_view().inv(),
        match result {
            Ok(_) => {
                &&& phys_view().live()
                &&& phys_view().frames.all_reserved(
                    phys_regions_frame_set(&physical_memory_regions))
                &&& forall|a: int|
                    #[trigger] mmio_regions_frame_set(mmio_regions).contains(a)
                        && phys_view().frames.covers(a)
                        ==> phys_view().frames.reserved(a)
            },
            Err(_) => true,
        },
)]
pub fn init(
    physical_memory_regions: LinkedList<TruncatedMemoryRegion<PhysicalAddress>>,
    mmio_regions: &LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    physical_memory_layout: Bitmap,
) -> Result<(), Error> { ... }

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(feature = "test")]
pub fn test() -> bool { ... }
