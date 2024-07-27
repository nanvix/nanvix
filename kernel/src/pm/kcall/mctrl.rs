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
    kcall::KcallArgs,
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

pub fn mctrl(pm: &mut ProcessManager, mm: &mut VirtMemoryManager, args: &KcallArgs) -> i32 {
    // Check if the calling process has memory management capabilities.
    match ProcessManager::has_capability(args.pid, Capability::MemoryManagement) {
        Ok(true) => (),
        Ok(false) => {
            let reason: &str = "process does not have memory management capabilities";
            error!("mctrl(): {}", reason);
            return ErrorCode::PermissionDenied.into_errno();
        },
        Err(e) => return e.code.into_errno(),
    }

    // Unpack kernel call arguments.
    let pid: ProcessIdentifier = ProcessIdentifier::from(args.arg0 as usize);
    let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(args.arg1 as usize) {
        Ok(vaddr) => vaddr,
        Err(e) => return e.code.into_errno(),
    };
    let access: AccessPermission = match AccessPermission::try_from(args.arg2) {
        Ok(access) => access,
        Err(e) => return e.code.into_errno(),
    };

    match do_mctrl(pm, mm, pid, vaddr, access) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
