// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Memory manager initialization when `platform-root-virtual-address-space-bootstrap` is enabled.
//!
//! The host builds the initial page tables before guest entry. During kernel init this module
//! walks the host page directory (from CR3), copies every page table into a BSS-backed slot
//! owned by the kernel, and constructs a root [`Vmem`] descriptor. After initialization the
//! kernel switches CR3 to its own page directory (backed by the BSS page-table allocator) so
//! that all paging structures are fully owned and modifiable by the kernel.

use crate::{
    collections::Bitmap,
    hal::{
        arch::x86::mem::mmu::page_table::PageTable,
        mem::{
            Address,
            FrameAddress,
            MemoryRegion,
            MemoryRegionType,
            MmioCachePolicy,
            PageAddress,
            PageAligned,
            PageTableAddress,
            PageTableAligned,
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
use ::arch::mem::{
    self,
    paging::{
        PresentFlag,
        PteWord,
    },
    PAGE_TABLE_LENGTH,
    PGTAB_ALIGNMENT,
};
use ::sparse_bitmap::SparseBitmap;
use ::sys::error::{
    Error,
    ErrorCode,
};

use super::{
    phys,
    virt::PAGE_TABLE_ALLOCATOR,
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

///
/// # Description
///
/// Splits memory regions into virtual and physical (platform-bootstrapped VAS path).
///
/// When the platform bootstraps the root VAS, scratch GVAs are NOT identity-mapped (GVA ≠ GPA).
/// The scratch-region addresses are managed by the host and the bump allocator, not the kernel
/// frame allocator, so they require explicit GVA→GPA translation for physical frame booking.
///
/// # Parameters
///
/// - `memory_regions`: Memory regions to split.
///
/// # Returns
///
/// Upon success, a tuple of (other virtual memory regions, virtual memory regions, physical memory
/// regions) is returned. Upon failure, an error is returned instead.
///
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

    // Walk the host page directory and copy each page table into a BSS-allocated slot so the kernel
    // owns all paging structures. This ensures the kernel can find and modify PTEs for MMIO
    // permission changes without depending on host page table memory.
    let kernel_pages: LinkedList<KernelPage> = LinkedList::new();
    let mut kernel_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)> = {
        let mut page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)> =
            LinkedList::new();

        let host_pd_ptr: *const PteWord = crate::hal::platform::get_root_pd_ptr();

        // SAFETY: The host page directory is mapped and readable after CoW pre-faulting.
        // This runs during single-threaded boot initialization. Each BSS slot returned by
        // alloc_as() is fully initialized via copy_nonoverlapping before any read, so
        // assume_init_mut() is sound (the contents are overwritten, not read as zeroes).
        unsafe {
            for i in 0..PAGE_TABLE_LENGTH {
                let pde: PteWord = *host_pd_ptr.add(i);
                if !PresentFlag::is_set(pde) {
                    continue;
                }

                let pt_gpa: usize = (pde & crate::hal::platform::PTE_ADDR_MASK_U32) as usize;
                let pt_gva: usize = crate::hal::platform::gpa_to_gva(pt_gpa);
                let pt_vaddr: usize = i * mem::PGTAB_SIZE;
                let pt_addr: PageTableAddress =
                    PageTableAddress::new(PageTableAligned::from_raw_value(pt_vaddr)?);

                // Allocate a BSS slot and copy the host page table contents into it.
                let bss_slot: &'static mut [PteWord; PAGE_TABLE_LENGTH] = PAGE_TABLE_ALLOCATOR
                    .alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()
                    .map_err(|e| {
                        error!("page table BSS allocation failed: {}", e);
                        Error::new(
                            ErrorCode::OutOfMemory,
                            "BSS page table allocation failed during host PT copy",
                        )
                    })?
                    .assume_init_mut();

                let host_pt_ptr: *const PteWord = pt_gva as *const PteWord;
                core::ptr::copy_nonoverlapping(
                    host_pt_ptr,
                    bss_slot.as_mut_ptr(),
                    PAGE_TABLE_LENGTH,
                );

                let storage: PageTableStorage = PageTableStorage::Bss(bss_slot);
                let page_table: PageTable<PageTableStorage> = PageTable::from_existing(storage);
                page_tables.push_back((pt_addr, page_table));
            }
        }

        page_tables
    };

    // Map MMIO regions into existing or new page tables.
    let unmapped_mmio: LinkedList<TruncatedMemoryRegion<VirtualAddress>> =
        filter_unmapped_mmio_regions(&mmio_regions, &kernel_page_tables)?;
    map_mmio_regions(&unmapped_mmio, &mut kernel_page_tables)?;

    VirtMemoryManager::init(kernel_pages, kernel_page_tables)
}

///
/// # Description
///
/// Filters MMIO regions to those whose first page PTE is not already present in the host-copied
/// page tables. Regions already initialized by the host (e.g., PEB, scratch) are excluded because
/// their PTEs are already correct.
///
/// # Parameters
///
/// - `mmio_regions`: MMIO regions to check.
/// - `kernel_page_tables`: Page tables copied from the host page directory.
///
/// # Returns
///
/// Upon success, a list of MMIO regions whose PTEs are not yet present is returned. Upon failure,
/// an error is returned instead.
///
fn filter_unmapped_mmio_regions(
    mmio_regions: &LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    kernel_page_tables: &LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
) -> Result<LinkedList<TruncatedMemoryRegion<VirtualAddress>>, Error> {
    let mut result: LinkedList<TruncatedMemoryRegion<VirtualAddress>> = LinkedList::new();

    for region in mmio_regions.iter() {
        let base: usize = region.start().into_raw_value();
        let pt_aligned: usize = ::sys::mm::align_down(base, PGTAB_ALIGNMENT);
        let page_addr: PageAddress = PageAddress::new(PageAligned::from_raw_value(base)?);

        let already_mapped: bool = kernel_page_tables
            .iter()
            .find(|(addr, _)| addr.into_raw_value() == pt_aligned)
            .is_some_and(|(_, pt)| pt.is_page_present(page_addr).unwrap_or(false));

        if !already_mapped {
            result.push_back(region.clone());
        }
    }

    Ok(result)
}

///
/// # Description
///
/// Maps MMIO regions into existing or new page tables.
///
/// For each page in each MMIO region, finds the page table covering the corresponding PDE range
/// in `kernel_page_tables` and maps the PTE into it. If no page table exists for that range, a
/// new BSS-backed page table is allocated and inserted.
///
/// # Parameters
///
/// - `mmio_regions`: MMIO regions whose PTEs are not yet present in `kernel_page_tables`.
/// - `kernel_page_tables`: Page tables copied from the host page directory.
///
/// # Errors
///
/// - `ResourceBusy` if a PTE is already present (address space collision).
/// - `OutOfMemory` if a BSS page table allocation fails.
///
fn map_mmio_regions(
    mmio_regions: &LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    kernel_page_tables: &mut LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
) -> Result<(), Error> {
    let mut sorted_regions: Vec<&TruncatedMemoryRegion<VirtualAddress>> =
        mmio_regions.iter().collect();
    sorted_regions.sort_by_key(|r| r.start().into_raw_value());

    let mut pts: Vec<(PageTableAddress, PageTable<PageTableStorage>)> =
        core::mem::take(kernel_page_tables).into_iter().collect();

    for region in sorted_regions {
        let cache_policy: MmioCachePolicy = region
            .cache_policy()
            .unwrap_or(MmioCachePolicy::UNCACHEABLE);
        let base: usize = region.start().into_raw_value();
        let end: usize = base + (region.size() - 1);
        let mut raw_vaddr: usize = base;

        while raw_vaddr <= end {
            let pt_aligned: usize = ::sys::mm::align_down(raw_vaddr, PGTAB_ALIGNMENT);
            let pt_addr: PageTableAddress =
                PageTableAddress::new(PageTableAligned::from_raw_value(pt_aligned)?);

            // Find existing page table for this PDE range, or allocate a new one.
            let idx: usize = match pts.iter().position(|(addr, _): &(PageTableAddress, _)| {
                addr.into_raw_value() == pt_addr.into_raw_value()
            }) {
                Some(i) => i,
                None => {
                    let insert_pos: usize = pts
                        .iter()
                        .position(|(addr, _): &(PageTableAddress, _)| {
                            addr.into_raw_value() > pt_addr.into_raw_value()
                        })
                        .unwrap_or(pts.len());
                    pts.insert(insert_pos, (pt_addr, alloc_empty_page_table()?));
                    insert_pos
                },
            };

            // TODO(#2261): use fill() to batch consecutive PTEs within the same page table.
            let gpa: usize = crate::hal::platform::gva_to_gpa(raw_vaddr);
            let paddr: FrameAddress = FrameAddress::new(PageAligned::from_raw_value(gpa)?);
            pts[idx].1.map(
                PageAddress::new(PageAligned::from_raw_value(raw_vaddr)?),
                paddr,
                true,
                cache_policy.write_through(),
                cache_policy.cache_enabled(),
                region.perm(),
            )?;

            raw_vaddr += mem::PAGE_SIZE;
        }
    }

    for entry in pts {
        kernel_page_tables.push_back(entry);
    }

    Ok(())
}

///
/// # Description
///
/// Allocates an empty BSS-backed page table.
///
/// # Returns
///
/// Upon success, an empty page table is returned. Upon failure, an error is returned instead.
///
fn alloc_empty_page_table() -> Result<PageTable<PageTableStorage>, Error> {
    // SAFETY: called during single-threaded kernel init; BSS is zero-initialized.
    let bss_slot: &'static mut [PteWord; PAGE_TABLE_LENGTH] = unsafe {
        PAGE_TABLE_ALLOCATOR
            .alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()
            .map_err(|e| {
                error!("page table BSS allocation failed for MMIO: {e}");
                Error::new(ErrorCode::OutOfMemory, "BSS page table allocation failed for MMIO")
            })?
            .assume_init_mut()
    };
    Ok(PageTable::new(PageTableStorage::Bss(bss_slot)))
}
