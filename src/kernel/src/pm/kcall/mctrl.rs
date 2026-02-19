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

fn do_mctrl(
    pm: &mut ProcessManager,
    mm: &mut VirtMemoryManager,
    pid: ProcessIdentifier,
    vaddr: PageAligned<VirtualAddress>,
    access: AccessPermission,
) -> Result<(), Error> {
    pm.mctrl(mm, pid, vaddr, access)
}

///
/// # Description
///
/// Kernel call handler for controlling memory access permissions.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `arg0`: Target process identifier.
/// - `arg1`: Virtual address.
/// - `arg2`: Access permission.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn mctrl(caller_pid: ProcessIdentifier, arg0: u32, arg1: u32, arg2: u32) -> KcallResult {
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

    // Check if the calling process has memory management capabilities.
    if pid != caller_pid {
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

    let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(arg1 as usize) {
        Ok(vaddr) => vaddr,
        Err(e) => return KcallResult::Error(e.code.into()),
    };
    let access: AccessPermission = match AccessPermission::try_from(arg2) {
        Ok(access) => access,
        Err(e) => return KcallResult::Error(e.code.into()),
    };

    match do_mctrl(pm, mm, pid, vaddr, access) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
