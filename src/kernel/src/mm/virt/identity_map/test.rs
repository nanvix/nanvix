// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::KERNEL_PD_PADDR;
// `sync_kernel_pdes` performs the 2-level PDE copy and exists on x86 only; the
// companion test below is gated to match.
#[cfg(not(target_arch = "x86_64"))]
use super::sync_kernel_pdes;
#[cfg(not(target_arch = "x86_64"))]
use crate::{
    hal::mem::PageDirectoryAddress,
    mm::VirtMemoryManager,
};
use ::arch::mem::{
    self,
    paging::{
        self,
        PageDirectoryEntry,
        Table,
        TableIndex,
    },
};
use ::config::kernel::MEMORY_SIZE;
use ::core::sync::atomic::Ordering;

///
/// # Description
///
/// Verifies that [`super::init`] pre-allocates a page table for every PDE index in
/// `[0, MEMORY_SIZE)`. Reads each PDE from the kernel page directory and
/// asserts the present bit is set.
///
fn test_init_preallocates_identity_map_pdes() -> bool {
    let pd_paddr: usize = KERNEL_PD_PADDR.load(Ordering::Acquire);
    if pd_paddr == 0 {
        error!("kernel PD not initialized");
        return false;
    }

    let pde_count: usize = MEMORY_SIZE / mem::PGTAB_SIZE;

    for i in 0..pde_count {
        let pde_idx: TableIndex = paging::pd_index(i * mem::PGTAB_SIZE);
        // SAFETY: the kernel PD is identity-mapped and initialized.
        let pd: Table<PageDirectoryEntry> = unsafe { Table::from_address(pd_paddr) };
        let pde: PageDirectoryEntry = match unsafe { pd.read(pde_idx) } {
            Some(pde) => pde,
            None => {
                error!("failed to read PDE at index {}", i);
                return false;
            },
        };
        if !pde.is_present() {
            error!("PDE at index {} is not present after init()", i);
            return false;
        }
    }

    true
}

///
/// # Description
///
/// Allocates a zeroed kernel page, treats it as a page directory, calls
/// [`sync_kernel_pdes`], and verifies that every present kernel PDE in
/// `[0, MEMORY_SIZE)` was copied into the target PD with the same frame address.
///
/// x86-only: on x86_64 the active page tables are a 4-level hierarchy managed by
/// the HAL and `sync_kernel_pdes` is a no-op, so this 2-level PDE-copy check does
/// not apply.
#[cfg(not(target_arch = "x86_64"))]
fn test_sync_kernel_pdes_copies_to_target() -> bool {
    let kernel_pd_paddr: usize = KERNEL_PD_PADDR.load(Ordering::Acquire);
    if kernel_pd_paddr == 0 {
        error!("kernel PD not initialized");
        return false;
    }

    // Allocate a zeroed kernel page to serve as the target page directory.
    let target_page = {
        // SAFETY: the memory manager is initialized and access is synchronized.
        let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };
        match mm.alloc_kpage(true) {
            Ok(page) => page,
            Err(e) => {
                error!("failed to allocate target page (error={e:?})");
                return false;
            },
        }
    };

    let target_pd_raw: usize = target_page.base().into_raw_value();
    let target_pd_paddr: PageDirectoryAddress =
        match PageDirectoryAddress::from_raw_value(target_pd_raw) {
            Ok(addr) => addr,
            Err(e) => {
                error!("invalid target PD address (error={e:?})");
                return false;
            },
        };

    // Sync kernel identity-mapping PDEs into the target PD.
    if let Err(e) = sync_kernel_pdes(target_pd_paddr) {
        error!("sync_kernel_pdes failed (error={e:?})");
        return false;
    }

    let pde_count: usize = MEMORY_SIZE / mem::PGTAB_SIZE;

    for i in 0..pde_count {
        let pde_idx: TableIndex = paging::pd_index(i * mem::PGTAB_SIZE);

        // SAFETY: both PDs are identity-mapped.
        let kernel_pd: Table<PageDirectoryEntry> = unsafe { Table::from_address(kernel_pd_paddr) };
        let kernel_pde: PageDirectoryEntry = match unsafe { kernel_pd.read(pde_idx) } {
            Some(pde) => pde,
            None => {
                error!("failed to read kernel PDE at index {}", i);
                return false;
            },
        };

        // SAFETY: the target PD is backed by a zeroed kernel page.
        let target_pd: Table<PageDirectoryEntry> = unsafe { Table::from_address(target_pd_raw) };
        let target_pde: PageDirectoryEntry = match unsafe { target_pd.read(pde_idx) } {
            Some(pde) => pde,
            None => {
                error!("failed to read target PDE at index {}", i);
                return false;
            },
        };

        // Both must agree on presence.
        if kernel_pde.is_present() != target_pde.is_present() {
            error!(
                "PDE mismatch at index {}: kernel present={}, target present={}",
                i,
                kernel_pde.is_present(),
                target_pde.is_present()
            );
            return false;
        }

        // If present, the target must point to the same page table frame.
        if kernel_pde.is_present() && kernel_pde.frame_address() != target_pde.frame_address() {
            error!(
                "PDE frame mismatch at index {}: kernel={:#x}, target={:#x}",
                i,
                kernel_pde.frame_address(),
                target_pde.frame_address()
            );
            return false;
        }
    }

    // target_page drops here, returning the kernel page to the pool.
    true
}

/// Runs all identity-mapping virtual memory tests.
pub fn test() -> bool {
    let mut passed: bool = true;

    passed &= run_test!(test_init_preallocates_identity_map_pdes);
    #[cfg(not(target_arch = "x86_64"))]
    {
        passed &= run_test!(test_sync_kernel_pdes_copies_to_target);
    }

    passed
}
