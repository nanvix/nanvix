// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::{
        Address,
        PageAligned,
        VirtualAddress,
    },
    kcall::KcallResult,
    mm::{
        KernelPage,
        VirtMemoryManager,
    },
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

fn do_mcopy(
    pm: &mut ProcessManager,
    mm: &mut VirtMemoryManager,
    src_pid: ProcessIdentifier,
    src_vaddr: PageAligned<VirtualAddress>,
    dst_pid: ProcessIdentifier,
    dst_vaddr: PageAligned<VirtualAddress>,
) -> Result<(), Error> {
    // Allocate a kernel page to use a scratch memory.
    let kpage: KernelPage = mm.alloc_kpage(true)?;

    // Copy to kernel page.
    pm.vmcopy_from_user(
        src_pid,
        kpage.base().into_virtual_address().into_inner(),
        src_vaddr.into_inner(),
        mem::PAGE_SIZE,
    )?;

    // Copy from kernel page.
    pm.vmcopy_to_user(
        dst_pid,
        dst_vaddr.into_inner(),
        kpage.base().into_virtual_address().into_inner(),
        mem::PAGE_SIZE,
    )?;

    Ok(())
}

///
/// # Description
///
/// Kernel call handler for copying memory between processes.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `arg0`: Source process identifier.
/// - `arg1`: Source virtual address.
/// - `arg2`: Destination process identifier.
/// - `arg3`: Destination virtual address.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn mcopy(
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

    // Unpack kernel call arguments.
    let src_pid: ProcessIdentifier = match ProcessIdentifier::try_from(arg0) {
        Ok(pid) => pid,
        Err(error) => {
            error!("{error:?}");
            return KcallResult::Error(error.code.into());
        },
    };
    let src_vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(arg1 as usize) {
        Ok(vaddr) => vaddr,
        Err(e) => return KcallResult::Error(e.code.into()),
    };
    let dst_pid: ProcessIdentifier = match ProcessIdentifier::try_from(arg2) {
        Ok(pid) => pid,
        Err(error) => {
            error!("{error:?}");
            return KcallResult::Error(error.code.into());
        },
    };
    let dst_vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(arg3 as usize) {
        Ok(vaddr) => vaddr,
        Err(e) => return KcallResult::Error(e.code.into()),
    };

    match do_mcopy(pm, mm, src_pid, src_vaddr, dst_pid, dst_vaddr) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
