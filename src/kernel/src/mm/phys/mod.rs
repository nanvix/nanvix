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
// The function is therefore `external_body`. Its abstract effect — every frame of every
// region becomes booked — is modelled by `PhysMemView::spec_book_frames` and the
// `lemma_book_physical_*` signatures in `mod.proof.rs`; binding that effect to the global
// allocator state is deferred to a later phase per the View design.
#[verus_verify(external_body)]
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
// `book_physical_memory_regions`. Its abstract effect — every *tracked* MMIO frame
// (`is_covered`) becomes booked while untracked frames are skipped — is modelled by
// `PhysMemView::spec_book_frames` over `M.intersect(covered())` and the
// `lemma_book_mmio_*` signatures in `mod.proof.rs`.
#[verus_verify(external_body)]
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
        // `init` is the one-shot boot entry point for the global physical-memory
        // subsystem. Its caller-visible abstract effect — establish the frame-allocator
        // invariant (`PhysMemView::inv`) and pre-reserve all boot-known RAM frames and all
        // tracked MMIO frames so they are never handed out by `alloc` — is modelled by
        // `PhysMemView::spec_initialize` / `spec_book_frames` and the `lemma_init_*`
        // signatures in `mod.proof.rs`. Binding that effect to the global singleton state
        // (`frame::instance()@` / `INSTANCE_INIT`) is deferred to a later phase (the
        // functions take no `self`/ghost handle through which the post-state could be named;
        // see the View design "Notes for Later Phases"). At this layer the body is verified
        // for memory/type safety and that every callee precondition is met.
        true,
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
