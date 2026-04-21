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
    collections::RawArray,
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
// Global Variables
//==================================================================================================

/// Frame allocator storage.
static mut FRAME_ALLOCATOR_STORAGE: [u8; config::kernel::MEMORY_SIZE
    / (mem::FRAME_SIZE * u8::BITS as usize)] =
    [0; config::kernel::MEMORY_SIZE / (mem::FRAME_SIZE * u8::BITS as usize)];

//==================================================================================================
// Standalone Functions
//==================================================================================================

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
            let mmio_addr: VirtualAddress = VirtualAddress::from_raw_value(start);
            let phys_addr: PageAligned<PhysicalAddress> = PageAligned::from_address(unsafe {
                PhysicalAddress::from_mmio_address(mmio_addr)?
            })?;

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

pub fn init(
    kpool: TruncatedMemoryRegion<PhysicalAddress>,
    physical_memory_regions: LinkedList<TruncatedMemoryRegion<PhysicalAddress>>,
    mmio_regions: &LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
) -> Result<(), Error> {
    // Initialize frame allocator singleton.
    info!("initializing the frame allocator ...");
    {
        // Safety: the frame allocator storage is valid and has a static lifetime.
        let storage: RawArray<u8> = unsafe {
            let (ptr, len): (*mut u8, usize) =
                (FRAME_ALLOCATOR_STORAGE.as_mut_ptr(), FRAME_ALLOCATOR_STORAGE.len());
            RawArray::from_raw_parts(ptr, len)?
        };
        // Safety: called exactly once during single-threaded boot.
        unsafe { frame::init(storage)? };
    }
    book_physical_memory_regions(physical_memory_regions)?;

    book_mmio_regions(mmio_regions)?;

    // Initialize kernel page pool.
    info!("initializing the kernel page pool ...");
    let kpool: Kpool = Kpool::new(kpool)?;

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
