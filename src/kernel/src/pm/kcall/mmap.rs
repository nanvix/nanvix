// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::{
        AccessPermission,
        Address,
        PageAligned,
        VirtualAddress,
    },
    kcall::KcallResult,
    mm::VirtMemoryManager,
    pm::ProcessManager,
};
use ::arch::mem;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::{
        Capability,
        ProcessIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_mmap(
    pm: &mut ProcessManager,
    mm: &mut VirtMemoryManager,
    pid: ProcessIdentifier,
    vaddr: PageAligned<VirtualAddress>,
    npages: usize,
    access: AccessPermission,
) -> Result<(), Error> {
    pm.mmap(mm, pid, vaddr, npages, access)
}

///
/// # Description
///
/// Kernel call handler for mapping memory.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `arg0`: Target process identifier.
/// - `arg1`: Virtual address to map.
/// - `arg2`: Number of pages to map.
/// - `arg3`: Access permission.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn mmap(
    caller_pid: ProcessIdentifier,
    arg0: u32,
    arg1: u32,
    arg2: u32,
    arg3: u32,
) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
    // SAFETY: the virtual memory manager is initialized and access is synchronized.
    let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };

    // Unpack kernel call arguments.
    let pid: ProcessIdentifier = match ProcessIdentifier::try_from(arg0) {
        Ok(pid) => pid,
        Err(error) => {
            error!("{error:?}");
            return KcallResult::Error(error.code.into());
        },
    };
    let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(arg1 as usize) {
        Ok(vaddr) => vaddr,
        Err(e) => return KcallResult::Error(e.code.into()),
    };
    let npages: usize = arg2 as usize;
    let access: AccessPermission = match AccessPermission::try_from(arg3) {
        Ok(access) => access,
        Err(e) => return KcallResult::Error(e.code.into()),
    };

    // Validate npages.
    if npages == 0 {
        let reason: &str = "zero page count";
        error!("{reason}");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Cap npages to the maximum number of pages that fit in the user mmap region.
    const MMAP_MAX_PAGES: usize = ::config::memory_layout::USER_MMAP_SIZE / mem::PAGE_SIZE;
    if npages > MMAP_MAX_PAGES {
        let reason: &str = "page count exceeds mmap region capacity";
        error!("{reason} (npages={npages}, max={MMAP_MAX_PAGES})");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Sanity check: ensure the range doesn't overflow.
    let range_size: usize = match npages.checked_mul(mem::PAGE_SIZE) {
        Some(size) => size,
        None => {
            let reason: &str = "page count overflow";
            error!("{reason}");
            return KcallResult::Error(ErrorCode::InvalidArgument.into());
        },
    };

    // Sanity check: ensure the mapped range doesn't overflow the address space.
    if vaddr.into_raw_value().checked_add(range_size).is_none() {
        let reason: &str = "address range overflow";
        error!("{reason}");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Check if attempting to map memory into a different process.
    if pid != caller_pid {
        // Check if the calling process has memory management capabilities.
        match pm.has_capability(caller_pid, Capability::MemoryManagement) {
            Ok(true) => (),
            Ok(false) => {
                let reason: &str = "process does not have memory management capabilities";
                error!("{reason}");
                return KcallResult::Error(ErrorCode::PermissionDenied.into());
            },
            Err(e) => return KcallResult::Error(e.code.into()),
        }
    }

    match do_mmap(pm, mm, pid, vaddr, npages, access) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
