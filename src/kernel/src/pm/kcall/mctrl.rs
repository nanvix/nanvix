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
    kcall::{
        KcallArgs,
        KcallResult,
    },
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

pub fn mctrl(pm: &mut ProcessManager, mm: &mut VirtMemoryManager, args: &KcallArgs) -> KcallResult {
    // Check if the calling process has memory management capabilities.
    match pm.has_capability(args.pid, Capability::MemoryManagement) {
        Ok(true) => (),
        Ok(false) => {
            let reason: &str = "process does not have memory management capabilities";
            error!("{reason}");
            return KcallResult::Error(ErrorCode::PermissionDenied.into());
        },
        Err(e) => return KcallResult::Error(e.code.into()),
    }

    // Unpack kernel call arguments.
    let pid: ProcessIdentifier = match ProcessIdentifier::try_from(args.arg0) {
        Ok(pid) => pid,
        Err(error) => {
            error!("{error:?}");
            return KcallResult::Error(error.code.into());
        },
    };
    let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(args.arg1 as usize) {
        Ok(vaddr) => vaddr,
        Err(e) => return KcallResult::Error(e.code.into()),
    };
    let access: AccessPermission = match AccessPermission::try_from(args.arg2) {
        Ok(access) => access,
        Err(e) => return KcallResult::Error(e.code.into()),
    };

    match do_mctrl(pm, mm, pid, vaddr, access) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
