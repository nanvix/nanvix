// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub(crate) mod frame;
mod kpool;
mod manager;
mod upool;

#[cfg(feature = "test")]
mod test;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    collections::Bitmap,
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
use ::sparse_bitmap::SparseBitmap;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use self::{
    kpool::{
        KernelFrame,
        Kpool,
    },
    manager::PhysMemoryManager,
    upool::UserFrame,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn book_physical_memory_regions(
    physical_memory_regions: LinkedList<TruncatedMemoryRegion<PhysicalAddress>>,
) -> Result<(), Error> {
    info!("booking physical memory regions ...");

    // Book physical memory that is not usable.
    for region in physical_memory_regions.iter() {
        info!(
            "booking: {} @ {:#010x} (size={:#x})",
            region.name(),
            region.start().into_raw_value(),
            region.size()
        );
        frame::alloc_range(region)?;
    }

    Ok(())
}

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
            // Translate GVA→GPA so scratch-region MMIO addresses map to the correct
            // frame allocator index. On microvm this is identity.
            let gpa: usize = crate::hal::platform::gva_to_gpa(start);
            let phys_addr: PageAligned<PhysicalAddress> = match PageAligned::from_raw_value(gpa) {
                Ok(pa) => pa,
                Err(_) => {
                    start += mem::FRAME_SIZE;
                    continue;
                },
            };

            // Attempt to book underlying frame.
            match frame::book(phys_addr) {
                // Frame successfully booked.
                Ok(()) => {},
                // Frame lies outside addressable physical memory.
                Err(e) if e.code == ErrorCode::InvalidArgument => {},
                // Something went wrong.
                Err(e) => {
                    warn!("failed to book frame for mmio region {:?} ({:?})", region, e);
                    return Err(e);
                },
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
/// - `kpool_base`: Base address of the kernel page pool.
/// - `physical_memory_regions`: Physical memory regions to book.
/// - `mmio_regions`: Memory-mapped I/O regions to book.
/// - `physical_memory_layout`: Physical memory layout bitmap.
/// - `kpool_bitmap`: Bitmap for the kernel page pool.
///
/// # Returns
///
/// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
///
pub fn init(
    kpool_base: PageAligned<PhysicalAddress>,
    physical_memory_regions: LinkedList<TruncatedMemoryRegion<PhysicalAddress>>,
    mmio_regions: &LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    physical_memory_layout: SparseBitmap,
    kpool_bitmap: Bitmap,
) -> Result<(), Error> {
    // Initialize frame allocator singleton.
    info!("initializing the frame allocator ...");
    // Safety: called exactly once during single-threaded boot.
    unsafe { frame::init(physical_memory_layout)? };
    book_physical_memory_regions(physical_memory_regions)?;

    book_mmio_regions(mmio_regions)?;

    // Initialize kernel page pool singleton.
    info!("initializing the kernel page pool ...");
    // Safety: called exactly once during single-threaded boot.
    let kpool: Kpool = unsafe { kpool::init(kpool_base, kpool_bitmap)? };

    // Initialize user page pool.
    info!("initializing the user page pool ...");
    let upool: Upool = Upool::new();

    // Initialize physical memory manager singleton.
    info!("initializing the physical memory manager ...");
    PhysMemoryManager::init(kpool, upool)?;

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
