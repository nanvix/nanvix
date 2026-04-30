// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Memory manager initialization when `platform-root-virtual-address-space-bootstrap` is enabled.
//!
//! The platform provides the root virtual address space — host-built page tables are used
//! directly and the kernel does not create its own identity map or switch CR3. Only a root
//! [`Vmem`] descriptor is created so the process manager can clone it for user processes.

use crate::{
    collections::Bitmap,
    hal::{
        arch::x86::mem::mmu::page_table::PageTable,
        mem::{
            Address,
            MemoryRegion,
            MemoryRegionType,
            PageAligned,
            PageTableAddress,
            PhysicalAddress,
            TruncatedMemoryRegion,
            VirtualAddress,
        },
    },
    kimage::KernelImage,
};
use ::alloc::{
    collections::LinkedList,
    vec::Vec,
};
use ::sparse_bitmap::SparseBitmap;
use ::sys::error::Error;

use super::{
    phys,
    KernelPage,
    PageTableStorage,
    PhysMemRegion,
    VirtMemRegion,
    VirtMemoryManager,
    Vmem,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Splits memory regions into virtual and physical (platform-bootstrapped VAS path).
///
/// When the platform bootstraps the root VAS, scratch GVAs are NOT identity-mapped (GVA ≠ GPA).
/// The scratch-region addresses are managed by the host and the bump allocator, not the kernel
/// frame allocator, so they require explicit GVA→GPA translation for physical frame booking.
fn parse_memory_regions(
    memory_regions: LinkedList<MemoryRegion<VirtualAddress>>,
) -> Result<(VirtMemRegion, VirtMemRegion, PhysMemRegion), Error> {
    let mut memory_regions: LinkedList<MemoryRegion<VirtualAddress>> = {
        let mut memory_regions: Vec<_> = memory_regions.into_iter().collect();
        memory_regions.sort();
        memory_regions.into_iter().collect()
    };

    let mut virtual_memory_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>> =
        LinkedList::new();
    let mut other_virtual_memory_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>> =
        LinkedList::new();
    let mut physical_memory_regions: LinkedList<TruncatedMemoryRegion<PhysicalAddress>> =
        LinkedList::new();

    while let Some(region) = memory_regions.pop_front() {
        if region.typ() == MemoryRegionType::Reserved || region.typ() == MemoryRegionType::Mmio {
            if PhysicalAddress::from_virtual_address(region.start()).is_ok() {
                if region.typ() != MemoryRegionType::Usable {
                    // On Hyperlight, scratch GVAs are NOT identity-mapped (GVA ≠ GPA).
                    // Skip physical frame booking for scratch-region addresses — they are
                    // managed by the host and the bump allocator, not the kernel frame allocator.
                    let in_scratch: bool =
                        crate::hal::platform::is_scratch_address(region.start().into_raw_value());

                    if !in_scratch {
                        match TruncatedMemoryRegion::from_virtual_memory_region(region.clone()) {
                            Ok(region) => physical_memory_regions.push_back(region),
                            Err(err) => panic!(
                                "failed to create physical memory region {:?} (error={:?})",
                                region, err
                            ),
                        }
                    } else {
                        // Scratch: translate GVA→GPA and create the physical region
                        // with the correct physical address for frame allocator booking.
                        let gpa: usize =
                            crate::hal::platform::gva_to_gpa(region.start().into_raw_value());
                        if let Ok(phys_start) = PageAligned::from_raw_value(gpa) {
                            match TruncatedMemoryRegion::new(
                                &region.name(),
                                phys_start,
                                region.size(),
                                region.typ(),
                                region.perm(),
                            ) {
                                Ok(pr) => physical_memory_regions.push_back(pr),
                                Err(err) => warn!(
                                    "failed to create scratch physical region {:?} (error={:?})",
                                    region, err
                                ),
                            }
                        }
                    }
                }
                virtual_memory_regions
                    .push_back(TruncatedMemoryRegion::from_memory_region(region)?);
            } else {
                other_virtual_memory_regions
                    .push_back(TruncatedMemoryRegion::from_memory_region(region)?);
            }
        }
    }

    Ok((other_virtual_memory_regions, virtual_memory_regions, physical_memory_regions))
}

///
/// # Description
///
/// Initializes the memory manager (platform-bootstrapped root VAS path).
///
/// # Parameters
///
/// - `kimage`: Kernel image.
/// - `memory_regions`: Memory regions.
/// - `mmio_regions`: MMIO regions.
/// - `physical_memory_layout`: Physical memory layout bitmap.
/// - `kpool_bitmap`: Statically-allocated bitmap for the kernel page pool.
///
/// # Returns
///
/// Upon success, the root virtual memory manager is returned. Upon failure, an error is returned
/// instead.
///
pub fn init(
    kimage: &KernelImage,
    memory_regions: LinkedList<MemoryRegion<VirtualAddress>>,
    mmio_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    physical_memory_layout: SparseBitmap,
    kpool_bitmap: Bitmap,
) -> Result<Vmem, Error> {
    info!("initializing the memory manager ...");

    type VirtMemRegions = LinkedList<TruncatedMemoryRegion<VirtualAddress>>;
    type PhysMemRegions = LinkedList<TruncatedMemoryRegion<PhysicalAddress>>;

    let kpool_base: PageAligned<PhysicalAddress> =
        PageAligned::<PhysicalAddress>::from_raw_value(kimage.kpool().start().into_raw_value())?;

    let (_other_virtual_memory_regions, _virtual_memory_regions, physical_memory_regions): (
        VirtMemRegions,
        VirtMemRegions,
        PhysMemRegions,
    ) = parse_memory_regions(memory_regions)?;

    phys::init(
        kpool_base,
        physical_memory_regions,
        &mmio_regions,
        physical_memory_layout,
        kpool_bitmap,
    )?;

    #[cfg(feature = "test")]
    phys::test();

    // On Hyperlight the host-built page tables are used directly — the kernel does not create
    // its own identity map or switch CR3.
    let kernel_pages: LinkedList<KernelPage> = LinkedList::new();
    let kernel_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)> =
        LinkedList::new();
    VirtMemoryManager::init(kernel_pages, kernel_page_tables)
}
