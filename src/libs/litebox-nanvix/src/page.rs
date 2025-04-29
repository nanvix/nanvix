// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::NanvixUserland;
use ::core::{
    ops::Range,
    panic,
};
use ::litebox::platform::{
    page_mgmt::{
        AllocationError,
        DeallocationError,
        MemoryRegionPermissions,
        PermissionUpdateError,
        RemapError,
    },
    trivial_providers::TransparentMutPtr,
    PageManagementProvider,
};
use ::posix::nvx::{
    self,
    mm::{
        AccessPermission,
        VirtualAddress,
    },
    pm::ProcessIdentifier,
    sys::{
        arch::mem,
        kcall,
    },
};

//==================================================================================================

impl<const ALIGN: usize> PageManagementProvider<ALIGN> for NanvixUserland {
    fn allocate_pages(
        &self,
        range: Range<usize>,
        initial_permissions: MemoryRegionPermissions,
        can_grow_down: bool,
    ) -> Result<Self::RawMutPointer<u8>, AllocationError> {
        nvx::trace!(
            "allocate_pages(): range={:x?}, initial_permissions={:?}, can_grow_down={:?}",
            range,
            initial_permissions,
            can_grow_down
        );

        if can_grow_down {
            // TODO: implement this functionality.
            nvx::debug!("allocate_pages(): can_grow_down is not supported");
        }

        // Check if range is page-aligned.
        if range.start % mem::PAGE_SIZE != 0 {
            return Err(AllocationError::Unaligned);
        }
        if range.end % mem::PAGE_SIZE != 0 {
            return Err(AllocationError::Unaligned);
        }

        let start: usize = range.start;
        let end: usize = range.end;
        let pid: ProcessIdentifier = kcall::pm::getpid().unwrap();
        for vaddr in (start..end).step_by(mem::PAGE_SIZE) {
            debug_assert!(vaddr != end);

            // FIXME: do not use 0b111u8 as permission.
            let permission: AccessPermission = 0b111u8.try_into().unwrap();

            // Attempt to map page.
            let vaddr: VirtualAddress = VirtualAddress::new(vaddr);
            if let Err(error) = kcall::mm::mmap(pid, vaddr, permission) {
                nvx::error!(
                    "allocate_pages(): failed to map page {:X?} (error={:?})",
                    vaddr,
                    error
                );
                return Err(AllocationError::OutOfMemory);
            }

            // FIXME: we are leaking memory if we fail.

            // NOTE: pages allocated with mmap() are always zeroed.
        }

        Ok(TransparentMutPtr {
            inner: range.start as *mut u8,
        })
    }

    unsafe fn deallocate_pages(&self, range: Range<usize>) -> Result<(), DeallocationError> {
        // Check if range is page-aligned.
        if range.start % mem::PAGE_SIZE != 0 {
            return Err(DeallocationError::Unaligned);
        }
        if range.end % mem::PAGE_SIZE != 0 {
            return Err(DeallocationError::Unaligned);
        }

        let pid: ProcessIdentifier = kcall::pm::getpid().unwrap();
        let start: usize = range.start;
        let end: usize = range.end;
        let ret: Result<(), DeallocationError> = Ok(());

        for vaddr in (start..end).step_by(mem::PAGE_SIZE) {
            debug_assert!(vaddr != end);

            let vaddr: VirtualAddress = VirtualAddress::from_raw_value(vaddr);

            if let Err(error) = kcall::mm::munmap(pid, vaddr) {
                // Save error.
                panic!(
                    "unmap_range(): failed to unmap page at {:X?}, skipping (error={:?})",
                    vaddr, error
                );
            }
        }

        ret
    }

    unsafe fn remap_pages(
        &self,
        old_range: Range<usize>,
        new_range: Range<usize>,
    ) -> Result<(), RemapError> {
        nvx::trace!("remap_pages(): old_range={:x?}, new_range={:x?}", old_range, new_range);
        unimplemented!("remap_pages() not implemented")
    }

    unsafe fn update_permissions(
        &self,
        range: Range<usize>,
        new_permissions: MemoryRegionPermissions,
    ) -> Result<(), PermissionUpdateError> {
        nvx::trace!(
            "update_permissions(): range={:x?}, new_permissions={:?}",
            range,
            new_permissions
        );
        // TODO: implement this function.
        Ok(())
    }
}
