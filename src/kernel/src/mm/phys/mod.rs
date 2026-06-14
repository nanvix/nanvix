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

// This function iterates the supplied `LinkedList` of regions. The std `LinkedList`
// iterator cannot be given a Verus `for`-loop specification from this crate (the orphan
// rule forbids implementing vstd's iterator traits for the foreign `Iter` type, and the
// pinned `vstd` dependency cannot be extended — see `mod.spec.rs` / `verus-unsupported.md`).
// The function is therefore `external_body`; its `#[verus_spec]` contract is honored by the
// caller (`init`). The abstract effect — every frame of every region becomes booked — is
// modelled by `PhysMemView::spec_book_frames` / `region_frames` and discharged by
// `lemma_book_region_reserves_region_frames` in `mod.proof.rs`. Because the region frames are
// the contents of the (un-viewable) `LinkedList`, they cannot be enumerated in this boundary
// contract; what is expressed is the caller-relevant guarantee that the allocator stays
// initialized and well-formed across booking.
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        phys_view().initialized,
        phys_view().inv(),
    ensures
        phys_view().inv(),
        match result {
            Ok(()) => phys_view().initialized,
            Err(_) => true,
        },
)]
fn book_physical_memory_regions(
    physical_memory_regions: LinkedList<TruncatedMemoryRegion<PhysicalAddress>>,
) -> Result<(), Error> {
    info!("booking physical memory regions ...");

    // Book physical memory that is not usable.
    for region in physical_memory_regions.iter() {
        info!("booking: {:?}", region);
        frame::alloc_range(region)?;
    }

    Ok(())
}

// `external_body` for the same std-`LinkedList` reason as
// `book_physical_memory_regions`; its `#[verus_spec]` contract is honored by the caller
// (`init`). The abstract effect — every *tracked* MMIO frame (`is_covered`) becomes booked
// while untracked frames are skipped — is modelled by `PhysMemView::spec_book_frames` over
// `M.intersect(covered())` and discharged by `lemma_book_mmio_skip_untracked` /
// `lemma_book_mmio_books_tracked` in `mod.proof.rs`. As with the physical-region helper, the
// concrete MMIO frame set is the contents of the (un-viewable) `LinkedList`, so the boundary
// contract expresses the caller-relevant guarantee: the allocator stays initialized and
// well-formed (the skip-if-not-covered tolerance never aborts boot).
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        phys_view().initialized,
        phys_view().inv(),
    ensures
        phys_view().inv(),
        match result {
            Ok(()) => phys_view().initialized,
            Err(_) => true,
        },
)]
fn book_mmio_regions(
    mmio_regions: &LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
) -> Result<(), Error> {
    info!("booking memory-mapped i/o regions ...");

    // Book memory-mapped I/O regions.
    for region in mmio_regions.iter() {
        info!("booking: {:?}", region);
        let mut start: usize = region.start().into_raw_value();
        let end: usize = start + (region.size() - 1);
        while start < end {
            let mmio_addr: usize = crate::hal::platform::gva_to_gpa(start);
            let phys_addr: PageAligned<PhysicalAddress> = PageAligned::from_address(unsafe {
                // MMIO GPAs may legitimately lie outside tracked RAM, so they must not go through
                // the regular physical-address validator here.
                PhysicalAddress::from_mmio_address(VirtualAddress::from_raw_value(mmio_addr))?
            })?;

            // Only book frames that the frame allocator actually tracks.
            // MMIO regions above RAM (e.g. the LAPIC at 0xFEE0_0000) are not
            // covered by the bitmap and must be skipped.
            if frame::is_covered(phys_addr) {
                frame::book(phys_addr)?;
            }
            start += mem::FRAME_SIZE;
        }
    }

    Ok(())
}

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
#[verus_spec(ret =>
    ensures
        // The frame-allocator invariant is established/preserved on every path: this
        // is the precondition every later `frame::*` / `PhysMemoryManager::*` call
        // relies on. (Vacuous before the allocator becomes initialized.)
        phys_view().inv(),
        match ret {
            // One-shot boot success: the global subsystem is now initialized and its
            // reservation state is well-formed, so reserved frames (allocated) are
            // disjoint from the free pool and can never be handed out by `alloc`.
            Ok(()) => {
                &&& phys_view().initialized
                &&& phys_view().frames.allocated_frames.disjoint(phys_view().frames.free_frames)
            },
            // Failure is terminal for the boot path; no completeness of reservation is
            // promised, only that the invariant still holds (checked above).
            Err(_) => true,
        },
)]
pub fn init(
    physical_memory_regions: LinkedList<TruncatedMemoryRegion<PhysicalAddress>>,
    mmio_regions: &LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    physical_memory_layout: Bitmap,
) -> Result<(), Error> {
    // Initialize frame allocator singleton.
    info!("initializing the frame allocator ...");
    // Safety: called exactly once during single-threaded boot.
    unsafe { frame::init(physical_memory_layout)? };
    book_physical_memory_regions(physical_memory_regions)?;

    book_mmio_regions(mmio_regions)?;

    // Initialize user page pool.
    info!("initializing the user page pool ...");
    let upool: Upool = Upool::new();

    // Initialize physical memory manager singleton.
    info!("initializing the physical memory manager ...");
    PhysMemoryManager::init(upool)?;

    Ok(())
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(feature = "test")]
pub fn test() -> bool {
    let mut passed = true;

    passed &= test::test();

    passed
}
